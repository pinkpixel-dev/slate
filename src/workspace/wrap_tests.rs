use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{open_workspace, press, test_dir, type_text};
use crate::storage::Storage;

/// A line far wider than the 900px test window, then a second line.
fn long_text() -> String {
    format!("{}\nend", "word ".repeat(60))
}

/// Which buffer line Down lands on from the start of the long first line.
/// With wrap on, Down stays inside line 0; with wrap off it reaches line 1.
fn line_after_down(cx: &mut TestAppContext, handle: AnyWindowHandle, workspace: &Entity<Workspace>) -> u32 {
    press(cx, handle, "ctrl-home");
    cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();
    press(cx, handle, "down");
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).cursor_position().line)
}

#[gpui_kit::test]
fn alt_z_wraps_every_tab_and_is_saved(cx: &mut TestAppContext) {
    let dir = test_dir("wrap-toggle");
    let (handle, workspace) = open_workspace(cx, &dir);
    type_text(cx, handle, &long_text());
    assert_eq!(line_after_down(cx, handle, &workspace), 1, "wrap is off by default");

    press(cx, handle, "alt-z");
    assert_eq!(line_after_down(cx, handle, &workspace), 0);
    assert!(Storage::in_dir(&dir).load_settings().word_wrap);

    // New tabs start with the current setting.
    press(cx, handle, "ctrl-n");
    type_text(cx, handle, &long_text());
    assert_eq!(line_after_down(cx, handle, &workspace), 0);

    press(cx, handle, "alt-z");
    assert_eq!(line_after_down(cx, handle, &workspace), 1);
    assert!(!Storage::in_dir(&dir).load_settings().word_wrap);
}

#[gpui_kit::test]
fn the_settings_switch_applies_wrap_after_the_update(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("wrap-settings"));
    type_text(cx, handle, &long_text());

    // The settings panel has no window, so the change lands once the app is idle.
    cx.update(|cx| workspace.update(cx, |workspace, cx| workspace.set_word_wrap(true, cx)));
    cx.run_until_parked();
    assert_eq!(line_after_down(cx, handle, &workspace), 0);
}
