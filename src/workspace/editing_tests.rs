use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{open_file, open_workspace, press, test_dir, type_text};
use crate::text_format::{Indent, LineEnding};

fn editor_text(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> String {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).value().to_string())
}

fn cursor_line(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> u32 {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).cursor_position().line)
}

fn dialog_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

fn editor_font_size(cx: &mut TestAppContext) -> f32 {
    cx.update(|cx| f32::from(cx.theme().mono_font_size))
}

/// A Rust tab holding `text`, cursor at the start.
fn rust_tab(cx: &mut TestAppContext, name: &str, text: &str) -> (AnyWindowHandle, Entity<Workspace>) {
    let dir = test_dir(name);
    let file = dir.join("main.rs");
    std::fs::write(&file, text).unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &file);
    (handle, workspace)
}

#[gpui_kit::test]
fn line_shortcuts_duplicate_move_and_undo(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("line-ops"));
    type_text(cx, handle, "a\nb\nc");
    press(cx, handle, "ctrl-home");

    press(cx, handle, "ctrl-shift-d");
    assert_eq!(editor_text(cx, &workspace), "a\na\nb\nc");
    assert_eq!(cursor_line(cx, &workspace), 1, "the cursor follows the copy");

    press(cx, handle, "alt-down");
    press(cx, handle, "alt-down");
    assert_eq!(editor_text(cx, &workspace), "a\nb\nc\na");
    press(cx, handle, "alt-up");
    assert_eq!(editor_text(cx, &workspace), "a\nb\na\nc");

    press(cx, handle, "ctrl-z");
    assert_eq!(editor_text(cx, &workspace), "a\nb\nc\na", "each move is one undo step");
    assert!(cx.update(|cx| workspace.read(cx).active_buffer().document.is_dirty()));
}

#[gpui_kit::test]
fn ctrl_slash_toggles_comments_for_the_language(cx: &mut TestAppContext) {
    let (handle, workspace) = rust_tab(cx, "comment", "let x = 1;\n");
    press(cx, handle, "ctrl-/");
    assert_eq!(editor_text(cx, &workspace), "// let x = 1;\n");
    press(cx, handle, "ctrl-/");
    assert_eq!(editor_text(cx, &workspace), "let x = 1;\n");

    // Plain text has no comment syntax, so nothing happens.
    press(cx, handle, "ctrl-n");
    type_text(cx, handle, "plain");
    press(cx, handle, "ctrl-/");
    assert_eq!(editor_text(cx, &workspace), "plain");
}

#[gpui_kit::test]
fn line_shortcuts_stay_out_of_the_find_bar(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("line-ops-find"));
    type_text(cx, handle, "one\ntwo");
    press(cx, handle, "ctrl-f");
    press(cx, handle, "ctrl-shift-d");
    press(cx, handle, "alt-up");
    assert_eq!(editor_text(cx, &workspace), "one\ntwo");
}

#[gpui_kit::test]
fn ctrl_g_jumps_to_a_line(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("go-to-line"));
    type_text(cx, handle, "1\n2\n3\n4\n5");

    press(cx, handle, "ctrl-g");
    assert!(dialog_open(cx, handle));
    type_text(cx, handle, "nope");
    press(cx, handle, "enter");
    assert!(dialog_open(cx, handle), "not a line number, so it stays open");

    press(cx, handle, "ctrl-a");
    type_text(cx, handle, "3");
    press(cx, handle, "enter");
    assert!(!dialog_open(cx, handle));
    assert_eq!(cursor_line(cx, &workspace), 2);

    press(cx, handle, "ctrl-g");
    type_text(cx, handle, "99");
    press(cx, handle, "enter");
    assert_eq!(cursor_line(cx, &workspace), 4, "past the end lands on the last line");
}

#[gpui_kit::test]
fn zoom_changes_the_editor_font_without_saving_it(cx: &mut TestAppContext) {
    let dir = test_dir("zoom");
    let (handle, workspace) = open_workspace(cx, &dir);
    let base = editor_font_size(cx);

    press(cx, handle, "ctrl-=");
    press(cx, handle, "ctrl-=");
    assert_eq!(editor_font_size(cx), base + 2.);
    press(cx, handle, "ctrl--");
    assert_eq!(editor_font_size(cx), base + 1.);

    // Changing the theme keeps the zoom.
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.update_appearance(cx, |settings| settings.theme = Some("Gruvbox Dark".into()));
        })
    });
    assert_eq!(editor_font_size(cx), base + 1.);
    let saved = std::fs::read_to_string(crate::storage::Storage::in_dir(&dir).settings_path()).unwrap();
    assert!(!saved.contains("zoom"));

    press(cx, handle, "ctrl-0");
    assert_eq!(editor_font_size(cx), base);
}

#[gpui_kit::test]
fn quick_open_finds_a_file_in_the_folder(cx: &mut TestAppContext) {
    let dir = test_dir("quick-open");
    let project = dir.join("project");
    std::fs::create_dir_all(project.join("src/workspace")).unwrap();
    std::fs::write(project.join("README.md"), "readme").unwrap();
    std::fs::write(project.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(project.join("src/workspace/tab_strip.rs"), "// tabs").unwrap();

    let (handle, workspace) = open_workspace(cx, &dir);
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| workspace.show_folder(project.clone(), cx));
    });
    cx.run_until_parked();

    press(cx, handle, "ctrl-p");
    assert!(dialog_open(cx, handle));
    assert_eq!(cx.update(|cx| workspace.read(cx).quick_open.matches.len()), 3);

    type_text(cx, handle, "tabst");
    assert_eq!(
        cx.update(|cx| workspace.read(cx).quick_open.matches.clone()),
        ["src/workspace/tab_strip.rs"]
    );
    press(cx, handle, "enter");
    assert!(!dialog_open(cx, handle));
    assert_eq!(editor_text(cx, &workspace), "// tabs");
}

#[gpui_kit::test]
fn quick_open_needs_somewhere_to_look(cx: &mut TestAppContext) {
    let (handle, _) = open_workspace(cx, &test_dir("quick-open-none"));
    press(cx, handle, "ctrl-p");
    assert!(!dialog_open(cx, handle));
}

#[gpui_kit::test]
fn crlf_files_stay_crlf_and_can_be_switched(cx: &mut TestAppContext) {
    let dir = test_dir("crlf");
    let file = dir.join("dos.txt");
    std::fs::write(&file, "one\r\ntwo\r\n").unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &file);

    assert_eq!(editor_text(cx, &workspace), "one\ntwo\n");
    assert_eq!(
        cx.update(|cx| workspace.read(cx).active_buffer().document.line_ending()),
        LineEnding::Crlf
    );

    type_text(cx, handle, "zero\n");
    press(cx, handle, "ctrl-s");
    assert_eq!(std::fs::read(&file).unwrap(), b"zero\r\none\r\ntwo\r\n");

    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.set_active_line_ending(LineEnding::Lf, window, cx));
    })
    .unwrap();
    assert!(cx.update(|cx| workspace.read(cx).active_buffer().document.is_dirty()));
    press(cx, handle, "ctrl-s");
    assert_eq!(std::fs::read(&file).unwrap(), b"zero\none\ntwo\n");
}

#[gpui_kit::test]
fn indentation_is_detected_and_drives_the_tab_key(cx: &mut TestAppContext) {
    let (handle, workspace) = rust_tab(cx, "indent", "fn a() {\n\tb();\n}\n");
    let indent = cx.update(|cx| workspace.read(cx).active_buffer().indent);
    assert_eq!(indent, Indent { hard_tabs: true, width: 4 });

    press(cx, handle, "tab");
    assert!(editor_text(cx, &workspace).starts_with("\tfn a()"));

    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.set_active_indent(Indent { hard_tabs: false, width: 2 }, cx)
        })
    });
    press(cx, handle, "tab");
    assert!(editor_text(cx, &workspace).starts_with("\t  fn a()"));
}
