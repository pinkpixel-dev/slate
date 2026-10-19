use std::time::Duration;

use gpui_kit::component::WindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{open_file, open_workspace, press, test_dir, type_text};

fn tab_names(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Vec<String> {
    cx.update(|cx| workspace.read(cx).buffers.iter().map(|buffer| buffer.document.display_name()).collect())
}

fn active_name(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> String {
    cx.update(|cx| workspace.read(cx).active_buffer().document.display_name())
}

fn dialog_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

#[gpui_kit::test]
fn ctrl_shift_t_reopens_closed_files_newest_first(cx: &mut TestAppContext) {
    let dir = test_dir("reopen");
    let (a, b, c) = (dir.join("a.txt"), dir.join("b.txt"), dir.join("c.txt"));
    for file in [&a, &b, &c] {
        std::fs::write(file, "text").unwrap();
    }
    let (handle, workspace) = open_workspace(cx, &dir);
    for file in [&a, &b, &c] {
        open_file(cx, handle, &workspace, file);
    }
    press(cx, handle, "ctrl-w"); // c
    press(cx, handle, "ctrl-w"); // b
    std::fs::remove_file(&b).unwrap();
    press(cx, handle, "ctrl-w"); // a, which leaves a fresh untitled tab

    press(cx, handle, "ctrl-shift-t");
    assert_eq!(tab_names(cx, &workspace), ["a.txt"], "reuses the empty tab");
    press(cx, handle, "ctrl-shift-t");
    assert_eq!(tab_names(cx, &workspace), ["a.txt", "c.txt"], "b.txt is gone from disk, so it's skipped");
    press(cx, handle, "ctrl-shift-t");
    assert_eq!(tab_names(cx, &workspace), ["a.txt", "c.txt"], "nothing left to reopen");
}

#[gpui_kit::test]
fn find_in_files_opens_the_match_at_its_line(cx: &mut TestAppContext) {
    let dir = test_dir("find-in-files");
    let project = dir.join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("notes.md"), "nothing to see").unwrap();
    std::fs::write(project.join("src/lib.rs"), "fn one() {}\n\nfn two() { Needle(); }\n").unwrap();

    let (handle, workspace) = open_workspace(cx, &dir);
    cx.update(|cx| workspace.update(cx, |workspace, cx| workspace.show_folder(project.clone(), cx)));
    cx.run_until_parked();

    press(cx, handle, "ctrl-shift-f");
    assert!(dialog_open(cx, handle));
    type_text(cx, handle, "needle");
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    let found = cx.update(|cx| workspace.read(cx).find_in_files.matches.clone());
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].file.as_str(), found[0].line, found[0].column), ("src/lib.rs", 2, 11));

    press(cx, handle, "enter");
    assert!(!dialog_open(cx, handle));
    assert_eq!(active_name(cx, &workspace), "lib.rs");
    let cursor = cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).cursor_position());
    assert_eq!((cursor.line, cursor.character), (2, 11));
}

#[gpui_kit::test]
fn find_in_files_needs_somewhere_to_look(cx: &mut TestAppContext) {
    let (handle, _) = open_workspace(cx, &test_dir("find-in-files-none"));
    press(cx, handle, "ctrl-shift-f");
    assert!(!dialog_open(cx, handle));
}

#[gpui_kit::test]
fn the_word_count_follows_edits(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("word-count"));
    type_text(cx, handle, "one two\nthree");
    assert_eq!(cx.update(|cx| workspace.read(cx).active_buffer().words(cx)), 3);
    type_text(cx, handle, " four");
    assert_eq!(cx.update(|cx| workspace.read(cx).active_buffer().words(cx)), 4);
}
