use std::path::PathBuf;

use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext, px};

use crate::storage::Storage;

use super::Workspace;
use super::editing::SortLines;
use super::tab_menu::CopyPath;
use super::tests::{open_file, open_workspace, press, test_dir};

fn editor_text(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> String {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).value().to_string())
}

/// A tab for `name` holding `text`, with trimming on save switched on or off.
fn tab(cx: &mut TestAppContext, dir: &str, name: &str, text: &str, trim: bool) -> (AnyWindowHandle, Entity<Workspace>, PathBuf) {
    let dir = test_dir(dir);
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    cx.update(|cx| workspace.update(cx, |workspace, _| workspace.settings.trim_whitespace_on_save = trim));
    open_file(cx, handle, &workspace, &path);
    (handle, workspace, path)
}

#[gpui_kit::test]
fn saving_trims_line_ends_and_undo_brings_them_back(cx: &mut TestAppContext) {
    let (handle, workspace, path) = tab(cx, "tidy-save", "main.rs", "fn main() {  \n    run();\t\n}", true);
    press(cx, handle, "ctrl-s");

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "fn main() {\n    run();\n}\n");
    assert_eq!(editor_text(cx, &workspace), "fn main() {\n    run();\n}\n", "the editor matches the file");

    press(cx, handle, "ctrl-z");
    assert_eq!(editor_text(cx, &workspace), "fn main() {  \n    run();\t\n}", "one undo step");
}

#[gpui_kit::test]
fn markdown_keeps_trailing_spaces_but_gets_a_final_newline(cx: &mut TestAppContext) {
    let (handle, _, path) = tab(cx, "tidy-md", "notes.md", "line one  \nline two", true);
    press(cx, handle, "ctrl-s");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "line one  \nline two\n");
}

#[gpui_kit::test]
fn saving_leaves_whitespace_alone_when_the_setting_is_off(cx: &mut TestAppContext) {
    let (handle, workspace, path) = tab(cx, "tidy-off", "main.rs", "a  \nb", false);
    press(cx, handle, "ctrl-s");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "a  \nb");
    assert_eq!(editor_text(cx, &workspace), "a  \nb");
}

#[gpui_kit::test]
fn copy_path_puts_the_full_path_on_the_clipboard(cx: &mut TestAppContext) {
    let (handle, _, path) = tab(cx, "copy-path", "main.rs", "", false);
    cx.update_window(handle, |_, window, cx| window.dispatch_action(Box::new(CopyPath), cx))
        .unwrap();
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some(path.display().to_string().as_str()));
}

#[gpui_kit::test]
fn sort_lines_runs_on_the_whole_file_as_one_undo_step(cx: &mut TestAppContext) {
    let (handle, workspace, _) = tab(cx, "sort-lines", "list.txt", "pear\nApple\nbanana\n", false);
    cx.update_window(handle, |_, window, cx| window.dispatch_action(Box::new(SortLines), cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(editor_text(cx, &workspace), "Apple\nbanana\npear\n");

    press(cx, handle, "ctrl-z");
    assert_eq!(editor_text(cx, &workspace), "pear\nApple\nbanana\n");
}

#[gpui_kit::test]
fn the_sidebar_comes_back_at_its_dragged_width(cx: &mut TestAppContext) {
    let dir = test_dir("sidebar-width");
    let (handle, workspace) = open_workspace(cx, &dir);
    press(cx, handle, "ctrl-b");
    cx.update_window(handle, |_, window, cx| {
        window.refresh();
        workspace.update(cx, |workspace, cx| {
            let layout = workspace.body_layout.clone();
            layout.update(cx, |layout, cx| layout.resize_panel(0, px(300.), window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(Storage::in_dir(&dir).load_state().sidebar_width, Some(300.));

    // A fresh window starts the sidebar at the saved width.
    let (handle, workspace) = open_workspace(cx, &dir);
    press(cx, handle, "ctrl-b");
    cx.run_until_parked();
    let width = cx.update(|cx| workspace.read(cx).body_layout.read(cx).sizes().first().copied());
    assert_eq!(width, Some(px(300.)));
}
