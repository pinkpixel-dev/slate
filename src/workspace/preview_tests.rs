use std::path::PathBuf;

use gpui_kit::{AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{open_file, open_workspace, press, test_dir, type_text};

/// The text the active tab's preview shows (Markdown markers rendered away),
/// or `None` while the preview is closed.
fn preview_text(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Option<String> {
    cx.update(|cx| {
        let preview = workspace.read(cx).active_buffer().preview.clone()?;
        Some(preview.read(cx).rendered_text().as_str().to_string())
    })
}

fn markdown_file(name: &str, text: &str) -> (PathBuf, PathBuf) {
    let dir = test_dir(name);
    let path = dir.join("notes.md");
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

#[gpui_kit::test]
fn ctrl_shift_v_shows_the_rendered_text_and_follows_typing(cx: &mut TestAppContext) {
    let (dir, path) = markdown_file("preview-live", "# Title\n\nSome **bold** text.\n");
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &path);

    press(cx, handle, "ctrl-shift-v");
    cx.run_until_parked();
    let shown = preview_text(cx, &workspace).expect("preview is open");
    assert!(shown.contains("Title") && shown.contains("Some bold text."), "{shown:?}");
    assert!(!shown.contains("**"), "Markdown is rendered, not shown raw");

    press(cx, handle, "ctrl-end");
    type_text(cx, handle, "\nAdded line");
    cx.run_until_parked();
    assert!(preview_text(cx, &workspace).unwrap().contains("Added line"));

    press(cx, handle, "ctrl-shift-v");
    assert_eq!(preview_text(cx, &workspace), None);
}

#[gpui_kit::test]
fn each_tab_has_its_own_preview(cx: &mut TestAppContext) {
    let (dir, path) = markdown_file("preview-tabs", "# One\n");
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &path);
    press(cx, handle, "ctrl-shift-v");

    press(cx, handle, "ctrl-n");
    assert_eq!(preview_text(cx, &workspace), None);
    press(cx, handle, "ctrl-shift-tab");
    assert!(preview_text(cx, &workspace).is_some());
}

#[gpui_kit::test]
fn the_preview_comes_back_with_the_session(cx: &mut TestAppContext) {
    let (dir, path) = markdown_file("preview-session", "# Kept\n");
    let (handle, workspace) = open_workspace(cx, &dir);
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.restore_session(window, cx));
    })
    .unwrap();
    open_file(cx, handle, &workspace, &path);
    press(cx, handle, "ctrl-shift-v");
    press(cx, handle, "ctrl-q");

    let storage = crate::storage::Storage::in_dir(&dir);
    let (handle, workspace) = cx.update(|cx| {
        gpui_kit::open_window(Default::default(), cx, |window, cx| {
            cx.new(|cx| Workspace::new(storage, window, cx))
        })
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.restore_session(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    // The file reloads in the background, and the preview follows it.
    assert_eq!(preview_text(cx, &workspace).as_deref().map(str::trim), Some("Kept"));
}
