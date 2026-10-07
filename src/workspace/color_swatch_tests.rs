use gpui_kit::component::{Colorize as _, WindowExt as _};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, AppContext as _, Bounds, Entity, TestAppContext, point, px, size};

use super::Workspace;
use super::color_swatches::{EditColor, layout_swatches};
use super::tests::{open_file, open_workspace, press, test_dir, type_text};

fn editor_text(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> String {
    cx.update(|cx| workspace.read(cx).active_buffer().editor.read(cx).value().to_string())
}

/// The text of every color value the active tab found.
fn found(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Vec<String> {
    let text = editor_text(cx, workspace);
    cx.update(|cx| {
        workspace
            .read(cx)
            .active_buffer()
            .colors
            .iter()
            .flat_map(|line| line.colors.iter().map(|color| text[color.range.clone()].to_string()))
            .collect()
    })
}

fn css_tab(cx: &mut TestAppContext, name: &str, text: &str) -> (AnyWindowHandle, Entity<Workspace>) {
    let dir = test_dir(name);
    let file = dir.join("style.css");
    std::fs::write(&file, text).unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &file);
    (handle, workspace)
}

/// Applies `hex` to the active tab's first color value, as the picker's Apply does.
fn apply_first(cx: &mut TestAppContext, handle: AnyWindowHandle, workspace: &Entity<Workspace>, original: &str, hex: &str) {
    let color = gpui_kit::Hsla::parse_hex(hex).unwrap();
    let original = original.to_string();
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| {
            let buffer = workspace.active_buffer();
            let (id, value) = (buffer.id, buffer.colors[0].colors[0].clone());
            workspace.apply_color(id, &value, &original, color, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn colors_are_found_on_open_and_follow_typing(cx: &mut TestAppContext) {
    let (handle, workspace) = css_tab(cx, "swatch-scan", "a { color: #42a4cf; }\n");
    assert_eq!(found(cx, &workspace), ["#42a4cf"]);

    press(cx, handle, "ctrl-end");
    type_text(cx, handle, "b { background: rgb(0 0 0 / 50%); }");
    assert_eq!(found(cx, &workspace), ["#42a4cf", "rgb(0 0 0 / 50%)"]);
}

#[gpui_kit::test]
fn applying_a_color_keeps_the_format_and_undoes_in_one_step(cx: &mut TestAppContext) {
    let (handle, workspace) = css_tab(cx, "swatch-apply", "a { color: #ABC; border: 1px solid #fff; }\n");
    apply_first(cx, handle, &workspace, "#ABC", "#ff0000");
    assert_eq!(editor_text(cx, &workspace), "a { color: #F00; border: 1px solid #fff; }\n");
    assert_eq!(found(cx, &workspace), ["#F00", "#fff"], "the new value is picked up again");

    press(cx, handle, "ctrl-z");
    assert_eq!(editor_text(cx, &workspace), "a { color: #ABC; border: 1px solid #fff; }\n");
}

#[gpui_kit::test]
fn applying_skips_text_that_changed_under_the_picker(cx: &mut TestAppContext) {
    let (handle, workspace) = css_tab(cx, "swatch-stale", "a { color: #123456; }\n");
    apply_first(cx, handle, &workspace, "#654321", "#ff0000");
    assert_eq!(editor_text(cx, &workspace), "a { color: #123456; }\n");
}

#[gpui_kit::test]
fn edit_color_opens_the_picker_only_on_a_color(cx: &mut TestAppContext) {
    let (handle, workspace) = css_tab(cx, "swatch-action", "a { color: #42a4cf; }\n");
    let has_dialog = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx)).unwrap()
    };
    let edit_color = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.dispatch_action(Box::new(EditColor), cx))
            .unwrap();
        cx.run_until_parked();
    };

    press(cx, handle, "ctrl-home");
    edit_color(cx);
    assert!(!has_dialog(cx), "the cursor isn't on a color");

    for _ in 0..13 {
        press(cx, handle, "right");
    }
    edit_color(cx);
    assert!(has_dialog(cx));
    assert_eq!(editor_text(cx, &workspace), "a { color: #42a4cf; }\n", "opening changes nothing");
}

#[gpui_kit::test]
fn swatches_sit_after_visible_lines_only(cx: &mut TestAppContext) {
    let mut text = String::new();
    for i in 0..200 {
        text.push_str(&format!("line{i} {{ color: #00ff00; }}\n"));
    }
    let (handle, workspace) = css_tab(cx, "swatch-layout", &text);
    cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();

    let screen = Bounds::new(point(px(0.), px(0.)), size(px(900.), px(600.)));
    let rows = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let buffer = workspace.read(cx).active_buffer();
            let state = buffer.editor.read(cx);
            let line_height = state.line_height().unwrap();
            let swatches = layout_swatches(state, &buffer.colors, screen);
            let first_line_end = state.range_to_bounds(&(buffer.colors[0].end..buffer.colors[0].end)).unwrap();
            (swatches, line_height, first_line_end)
        })
    };

    let (swatches, line_height, line_end) = rows(cx);
    assert!(!swatches.is_empty() && swatches.len() < 200, "only on-screen lines: {}", swatches.len());
    let first = swatches[0].0;
    assert!(first.origin.x > line_end.origin.x, "past the end of the line");
    assert!(first.origin.y >= line_end.origin.y && first.origin.y < line_end.origin.y + line_height);

    press(cx, handle, "ctrl-end");
    cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();
    let (swatches, _, _) = rows(cx);
    let text = editor_text(cx, &workspace);
    let lines: Vec<_> = cx.update(|cx| workspace.read(cx).active_buffer().colors.iter().map(|line| line.start).collect());
    let first_shown = swatches.first().map(|(_, value)| value.range.start).unwrap();
    assert!(first_shown > lines[100], "scrolled-away lines get no swatch ({} of {})", first_shown, text.len());
}
