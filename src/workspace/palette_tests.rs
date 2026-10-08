use gpui_kit::component::WindowExt as _;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::{KEY_CONTEXT, NewFile, Workspace};
use super::tests::{open_workspace, press, test_dir, type_text};

fn palette_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

/// Opens the palette, types `query`, and confirms the top match.
fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, query: &str) {
    press(cx, handle, "ctrl-shift-p");
    assert!(palette_open(cx, handle));
    type_text(cx, handle, query);
    press(cx, handle, "enter");
}

fn tab_count(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> usize {
    cx.update(|cx| workspace.read(cx).buffers.len())
}

#[gpui_kit::test]
fn typing_and_enter_runs_a_command_and_closes_the_palette(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("palette-run"));

    run(cx, handle, "new file");
    assert!(!palette_open(cx, handle));
    assert_eq!(tab_count(cx, &workspace), 2);

    run(cx, handle, "word wrap");
    assert!(cx.update(|cx| workspace.read(cx).settings.word_wrap));
}

#[gpui_kit::test]
fn the_palette_can_open_the_find_bar(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("palette-find"));
    run(cx, handle, "find and replace");
    cx.update(|cx| {
        let find = &workspace.read(cx).find;
        assert!(find.open && find.replace_mode);
    });
}

#[gpui_kit::test]
fn escape_closes_the_palette_without_running_anything(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("palette-escape"));
    press(cx, handle, "ctrl-shift-p");
    press(cx, handle, "escape");
    assert!(!palette_open(cx, handle));
    assert_eq!(tab_count(cx, &workspace), 1);
}

#[gpui_kit::test]
fn shortcut_hints_resolve_from_inside_the_palette(cx: &mut TestAppContext) {
    let (handle, _) = open_workspace(cx, &test_dir("palette-hints"));
    press(cx, handle, "ctrl-shift-p");
    let hint = cx
        .update_window(handle, |_, window, _| {
            Kbd::binding_for_action(&NewFile, Some(KEY_CONTEXT), window).map(|kbd| format!("{kbd:?}"))
        })
        .unwrap();
    assert!(hint.is_some_and(|hint| hint.contains("\"n\"")), "Ctrl+N shows next to New File");
}

#[gpui_kit::test]
fn quick_open_lists_recent_files_when_there_is_no_folder(cx: &mut TestAppContext) {
    let dir = test_dir("quick-open-recent");
    let file = dir.join("recent.md");
    std::fs::write(&file, "hi").unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    cx.update(|cx| workspace.update(cx, |workspace, _| workspace.state.add_recent(&file)));

    press(cx, handle, "ctrl-p");
    assert!(palette_open(cx, handle));
    let matches = cx.update(|cx| workspace.read(cx).quick_open.matches.clone());
    assert_eq!(matches, [file.to_string_lossy().into_owned()]);

    press(cx, handle, "enter");
    let opened = cx.update(|cx| workspace.read(cx).active_buffer().document.path().map(|p| p.to_path_buf()));
    assert_eq!(opened, Some(file));
}
