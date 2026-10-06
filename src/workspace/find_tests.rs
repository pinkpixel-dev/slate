use gpui_kit::{AnyWindowHandle, Entity, TestAppContext};

use super::Workspace;
use super::tests::{click, open_file, open_workspace, press, test_dir, type_text};

fn editor_text(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> String {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).value().to_string())
}

fn match_label(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Option<String> {
    cx.update(|cx| workspace.read(cx).match_label(cx))
}

fn selection(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> std::ops::Range<usize> {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).selected_range())
}

fn find_open(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> bool {
    cx.update(|cx| workspace.read(cx).find.open)
}

/// A tab with `text` in it and the cursor back at the start.
fn workspace_with(cx: &mut TestAppContext, name: &str, text: &str) -> (AnyWindowHandle, Entity<Workspace>) {
    let (handle, workspace) = open_workspace(cx, &test_dir(name));
    type_text(cx, handle, text);
    press(cx, handle, "ctrl-home");
    (handle, workspace)
}

#[gpui_kit::test]
fn ctrl_f_finds_and_steps_through_matches(cx: &mut TestAppContext) {
    let (handle, workspace) = workspace_with(cx, "find-steps", "foo bar\nFoo baz\nfoo");

    press(cx, handle, "ctrl-f");
    assert!(find_open(cx, &workspace));
    type_text(cx, handle, "foo");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 3"));
    assert_eq!(selection(cx, &workspace), 0..3);

    press(cx, handle, "enter");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("2 of 3"));
    assert_eq!(selection(cx, &workspace), 8..11);

    press(cx, handle, "shift-enter");
    press(cx, handle, "shift-enter");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("3 of 3"), "previous wraps around");

    // Escape closes the bar and leaves the last match selected in the editor.
    press(cx, handle, "escape");
    assert!(!find_open(cx, &workspace));
    assert_eq!(selection(cx, &workspace), 16..19);
}

#[gpui_kit::test]
fn the_search_starts_at_the_cursor_and_is_seeded_from_the_selection(cx: &mut TestAppContext) {
    let (handle, workspace) = workspace_with(cx, "find-anchor", "one two\none two\none two");

    // Select the second "two" (bytes 12..15), then open find.
    cx.update(|cx| {
        let editor = workspace.read(cx).active_buffer().editor.clone();
        editor.update(cx, |state, cx| state.set_selected_range(12..15, cx));
    });
    press(cx, handle, "ctrl-f");

    let query = cx.update(|cx| workspace.read(cx).find.query.read(cx).value().to_string());
    assert_eq!(query, "two");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("2 of 3"));
}

#[gpui_kit::test]
fn match_case_narrows_the_results(cx: &mut TestAppContext) {
    let (handle, workspace) = workspace_with(cx, "find-case", "Slate slate SLATE");

    press(cx, handle, "ctrl-f");
    type_text(cx, handle, "slate");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 3"));

    press(cx, handle, "alt-c");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 1"));

    type_text(cx, handle, "x");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("No results"));
}

#[gpui_kit::test]
fn replace_one_then_replace_all(cx: &mut TestAppContext) {
    let (handle, workspace) = workspace_with(cx, "find-replace", "cat cat cat");

    press(cx, handle, "ctrl-h");
    type_text(cx, handle, "cat");
    press(cx, handle, "tab");
    type_text(cx, handle, "dog");

    press(cx, handle, "enter");
    assert_eq!(editor_text(cx, &workspace), "dog cat cat");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 2"));

    click(cx, handle, "replace-all");
    assert_eq!(editor_text(cx, &workspace), "dog dog dog");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("No results"));
    assert!(cx.update(|cx| workspace.read(cx).active_buffer().document.is_dirty()));
}

#[gpui_kit::test]
fn switching_tabs_moves_the_search_to_the_new_tab(cx: &mut TestAppContext) {
    let dir = test_dir("find-tabs");
    let file = dir.join("notes.txt");
    std::fs::write(&file, "apple\napple\n").unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    type_text(cx, handle, "apple pie");

    press(cx, handle, "ctrl-f");
    type_text(cx, handle, "apple");
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 1"));

    open_file(cx, handle, &workspace, &file);
    assert!(find_open(cx, &workspace));
    assert_eq!(match_label(cx, &workspace).as_deref(), Some("1 of 2"));

    // The tab we left no longer shows highlights.
    let first_active = cx.update(|cx| {
        workspace.read(cx).buffers[0].editor.read(cx).search_session().is_active()
    });
    assert!(!first_active);
}
