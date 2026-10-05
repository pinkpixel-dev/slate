use gpui_kit::component::WindowExt as _;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions,
    point, px, size,
};

use super::Workspace;

fn open_workspace(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<Workspace>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::init(cx);
        super::init(cx);

        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(900.), px(600.)));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(window, cx)))
            .expect("test window should open")
    })
}

/// Types into the focused editor, then lets the change events settle.
fn type_text(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.input(text, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    cx.update_window(handle, |_, window, cx| window.press(keys, cx))
        .unwrap();
    cx.run_until_parked();
}

fn click(cx: &mut TestAppContext, handle: AnyWindowHandle, id: &'static str) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click(id, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn has_dialog(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

#[gpui_kit::test]
fn quitting_with_unsaved_edits_asks_first(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx);

    type_text(cx, handle, "hello");
    assert!(cx.update(|cx| workspace.read(cx).document.is_dirty()));

    press(cx, handle, "ctrl-q");
    assert!(has_dialog(cx, handle), "Ctrl+Q should ask before discarding edits");

    click(cx, handle, "dialog-cancel");
    assert!(!has_dialog(cx, handle));
    assert!(cx.update(|cx| workspace.read(cx).document.is_dirty()));
    assert_eq!(cx.update(|cx| workspace.read(cx).editor.read(cx).value()), "hello");
}

#[gpui_kit::test]
fn discarding_before_a_new_file_clears_the_editor(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx);

    type_text(cx, handle, "scratch");
    press(cx, handle, "ctrl-n");
    assert!(has_dialog(cx, handle));

    click(cx, handle, "dialog-discard");
    assert!(!has_dialog(cx, handle));
    cx.update(|cx| {
        let workspace = workspace.read(cx);
        assert!(!workspace.document.is_dirty());
        assert_eq!(workspace.editor.read(cx).value(), "");
    });
}

#[gpui_kit::test]
fn quitting_without_edits_closes_right_away(cx: &mut TestAppContext) {
    let (handle, _workspace) = open_workspace(cx);

    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap();
    press(cx, handle, "ctrl-q");

    assert!(
        cx.update_window(handle, |_, _, _| ()).is_err(),
        "the window should be gone"
    );
}
