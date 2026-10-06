use std::path::Path;
use std::time::Duration;

use gpui_kit::component::WindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext, WindowOptions};

use super::Workspace;
use super::tests::{open_file, open_workspace, press, test_dir, type_text};
use crate::session::Session;
use crate::storage::Storage;

/// Another Slate window on the same storage, like the next launch. The app is
/// already initialized by the first `open_workspace`.
fn relaunch(cx: &mut TestAppContext, dir: &Path) -> (AnyWindowHandle, Entity<Workspace>) {
    let storage = Storage::in_dir(dir);
    let (handle, workspace) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Workspace::new(storage, window, cx))
        })
        .expect("test window should open")
    });
    restore(cx, handle, &workspace);
    (handle, workspace)
}

fn restore(cx: &mut TestAppContext, handle: AnyWindowHandle, workspace: &Entity<Workspace>) {
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.restore_session(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
}

fn tabs(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Vec<(String, String, bool)> {
    cx.update(|cx| {
        workspace
            .read(cx)
            .buffers
            .iter()
            .map(|buffer| {
                (
                    buffer.document.display_name(),
                    buffer.editor.read(cx).value().to_string(),
                    buffer.document.is_dirty(),
                )
            })
            .collect()
    })
}

fn has_dialog(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

#[gpui_kit::test]
fn quitting_keeps_every_tab_and_unsaved_edit_for_next_launch(cx: &mut TestAppContext) {
    let dir = test_dir("session-round-trip");
    let edited = dir.join("edited.md");
    let clean = dir.join("clean.rs");
    std::fs::write(&edited, "# Notes\n").unwrap();
    std::fs::write(&clean, "fn main() {}\n").unwrap();

    let (handle, workspace) = open_workspace(cx, &dir);
    restore(cx, handle, &workspace);
    type_text(cx, handle, "scratch");
    open_file(cx, handle, &workspace, &edited);
    press(cx, handle, "ctrl-end");
    type_text(cx, handle, "more");
    open_file(cx, handle, &workspace, &clean);
    cx.update(|cx| workspace.update(cx, |workspace, cx| workspace.show_folder(dir.clone(), cx)));
    press(cx, handle, "ctrl-pageup");

    press(cx, handle, "ctrl-q");
    assert!(
        cx.update_window(handle, |_, _, _| ()).is_err(),
        "quitting closes right away, without asking, while restore is on"
    );

    let (_, workspace) = relaunch(cx, &dir);
    assert_eq!(
        tabs(cx, &workspace),
        [
            ("Untitled".into(), "scratch".into(), true),
            ("edited.md".into(), "# Notes\nmore".into(), true),
            ("clean.rs".into(), "fn main() {}\n".into(), false),
        ]
    );
    cx.update(|cx| {
        let workspace = workspace.read(cx);
        assert_eq!(workspace.active, 1, "the active tab comes back");
        assert!(workspace.sidebar_open);
        assert_eq!(workspace.sidebar.read(cx).root(), Some(dir.as_path()));
    });
}

#[gpui_kit::test]
fn edits_are_saved_a_second_after_typing_stops(cx: &mut TestAppContext) {
    let dir = test_dir("session-debounce");
    let (handle, workspace) = open_workspace(cx, &dir);
    restore(cx, handle, &workspace);
    let session_path = Storage::in_dir(&dir).session_path();

    type_text(cx, handle, "draft");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert!(!session_path.exists(), "still waiting for typing to stop");

    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    let session = Storage::in_dir(&dir).load_session();
    assert_eq!(session.tabs.len(), 1);
    assert_eq!(session.tabs[0].text.as_deref(), Some("draft"));
}

#[gpui_kit::test]
fn a_launch_with_files_leaves_the_saved_session_alone(cx: &mut TestAppContext) {
    let dir = test_dir("session-args");
    let storage = Storage::in_dir(&dir);
    let saved = Session {
        tabs: vec![crate::session::SessionTab {
            untitled_number: 1,
            text: Some("keep me".into()),
            ..Default::default()
        }],
        ..Session::default()
    };
    crate::storage::write_atomic(&storage.session_path(), &crate::storage::to_json(&saved)).unwrap();

    // No `restore_session` call: this is `slate somefile`.
    let (handle, _) = open_workspace(cx, &dir);
    type_text(cx, handle, "other work");
    press(cx, handle, "ctrl-q");
    assert!(has_dialog(cx, handle), "without restore, quitting asks as usual");
    assert_eq!(storage.load_session(), saved);
}

#[gpui_kit::test]
fn turning_restore_off_forgets_the_session_and_brings_back_the_prompt(cx: &mut TestAppContext) {
    let dir = test_dir("session-off");
    let (handle, workspace) = open_workspace(cx, &dir);
    restore(cx, handle, &workspace);
    type_text(cx, handle, "draft");
    cx.update(|cx| workspace.update(cx, |workspace, cx| workspace.write_session(cx)));
    let session_path = Storage::in_dir(&dir).session_path();
    assert!(session_path.exists());

    cx.update(|cx| workspace.update(cx, |workspace, cx| workspace.set_restore_session(false, cx)));
    assert!(!session_path.exists());
    assert!(!Storage::in_dir(&dir).load_settings().restore_session);

    press(cx, handle, "ctrl-q");
    assert!(has_dialog(cx, handle));
}
