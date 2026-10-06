use std::time::Duration;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{open_file, open_workspace, press, test_dir};
use crate::storage::Storage;

fn settle(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();
    cx.run_until_parked();
}

fn long_rust_file(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = test_dir(name);
    let path = dir.join("main.rs");
    let body: String = (0..300).map(|i| format!("fn f{i}() {{ let x = {i}; }}\n")).collect();
    std::fs::write(&path, body).unwrap();
    (dir, path)
}

#[gpui_kit::test]
fn the_minimap_draws_every_line_with_syntax_colors(cx: &mut TestAppContext) {
    let (dir, path) = long_rust_file("minimap-lines");
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &path);
    settle(cx, handle);

    cx.update(|cx| {
        let workspace = workspace.read(cx);
        let lines = &workspace.active_buffer().minimap.lines();
        assert_eq!(lines.len(), 301, "300 lines plus the empty one after the last newline");
        let plain = cx.theme().foreground.opacity(0.55);
        assert!(
            lines[0].iter().any(|run| run.color != plain),
            "keywords get their syntax color"
        );
    });
}

#[gpui_kit::test]
fn clicking_the_minimap_scrolls_the_editor(cx: &mut TestAppContext) {
    let (dir, path) = long_rust_file("minimap-click");
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &path);
    settle(cx, handle);

    let scrolled = |cx: &mut TestAppContext| {
        cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).scroll_offset().y)
    };
    assert_eq!(scrolled(cx), gpui_kit::px(0.));
    // Kit's test `click` only finds its own components, so press the
    // minimap's handler directly, in the middle of where it was painted.
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            let bounds = workspace.minimap_bounds.get();
            let event = gpui_kit::MouseDownEvent {
                button: gpui_kit::MouseButton::Left,
                position: bounds.center(),
                ..Default::default()
            };
            workspace.on_minimap_down(&event, cx);
        })
    });
    settle(cx, handle);
    assert!(scrolled(cx) < gpui_kit::px(0.), "clicking below the box scrolls down");
}

#[gpui_kit::test]
fn ctrl_shift_m_toggles_it_and_word_wrap_hides_it(cx: &mut TestAppContext) {
    let dir = test_dir("minimap-toggle");
    let (handle, workspace) = open_workspace(cx, &dir);
    let visible = |cx: &mut TestAppContext, workspace: &Entity<Workspace>| {
        cx.update(|cx| workspace.read(cx).minimap_visible())
    };
    assert!(visible(cx, &workspace), "on by default");

    press(cx, handle, "alt-z");
    assert!(!visible(cx, &workspace), "word wrap hides it");
    press(cx, handle, "alt-z");
    assert!(visible(cx, &workspace));

    press(cx, handle, "ctrl-shift-m");
    assert!(!visible(cx, &workspace));
    assert!(!Storage::in_dir(&dir).load_settings().show_minimap);
}
