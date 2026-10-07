use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::color_picker::{ColorPickerState, ColorSelect};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;

use super::Workspace;
use super::buffer::BufferId;
use crate::color_values::{self, ColorLine, ColorValue};

actions!(slate, [EditColor, ToggleColorSwatches]);

/// Space between the end of a line and its first swatch.
const LINE_GAP: f32 = 10.;
/// Space between swatches on the same line.
const SWATCH_GAP: f32 = 4.;
/// Swatch size as a share of the line height.
const SWATCH_SCALE: f32 = 0.6;

/// A swatch drawn this frame: where it is, and the color value behind it.
type Swatch = (Bounds<Pixels>, ColorValue);

impl Workspace {
    /// Re-reads the tab's color values after its text changed.
    pub(super) fn refresh_colors(&mut self, id: BufferId, cx: &mut Context<Self>) {
        if !self.settings.color_swatches {
            return;
        }
        let Some(buffer) = self.buffer_mut(id) else {
            return;
        };
        let text = buffer.editor.read(cx).text().to_string();
        buffer.colors = Rc::new(color_values::scan(&text));
    }

    pub(super) fn toggle_color_swatches(&mut self, _: &ToggleColorSwatches, _: &mut Window, cx: &mut Context<Self>) {
        self.set_color_swatches(!self.settings.color_swatches, cx);
    }

    pub(super) fn set_color_swatches(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.color_swatches = show;
        self.save_settings();
        // Edits made while swatches were off weren't scanned.
        let ids: Vec<_> = self.buffers.iter().map(|buffer| buffer.id).collect();
        for id in ids {
            self.refresh_colors(id, cx);
        }
        cx.notify();
    }

    /// Opens the picker for the color value under the cursor: the keyboard
    /// route to what clicking a swatch does.
    pub(super) fn edit_color(&mut self, _: &EditColor, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settings.color_swatches {
            return;
        }
        let buffer = self.active_buffer();
        let cursor = buffer.editor.read(cx).cursor();
        if let Some(value) = color_values::at_offset(&buffer.colors, cursor).cloned() {
            let id = buffer.id;
            self.open_color_picker(id, value, window, cx);
        }
    }

    fn open_color_picker(&mut self, id: BufferId, value: ColorValue, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(original) = self
            .buffers
            .iter()
            .find(|buffer| buffer.id == id)
            .map(|buffer| buffer.editor.read(cx).text().slice(value.range.clone()).to_string())
        else {
            return;
        };
        let picker = cx.new(|cx| ColorPickerState::new(window, cx).default_value(to_hsla(value.rgba)));
        let this = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _, _| {
            let apply = this.clone();
            let picker_for_apply = picker.clone();
            let value = value.clone();
            let original = original.clone();

            dialog
                .w(px(360.))
                .title("Edit Color")
                .child(v_flex().gap_2().child(ColorSelect::new(&picker)))
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("edit-color-cancel")
                                .outline()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("edit-color-apply")
                                .primary()
                                .label("Apply")
                                .on_click(move |_, window, cx| {
                                    let color = picker_for_apply.read(cx).value();
                                    window.close_dialog(cx);
                                    if let Some(color) = color {
                                        _ = apply.update(cx, |workspace, cx| {
                                            workspace.apply_color(id, &value, &original, color, window, cx)
                                        });
                                    }
                                }),
                        ),
                )
        });
    }

    /// Rewrites one color value in its own format, as a single undo step.
    /// Does nothing if the text there changed while the picker was open.
    pub(super) fn apply_color(
        &mut self,
        id: BufferId,
        value: &ColorValue,
        original: &str,
        color: Hsla,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.buffers.iter().find(|buffer| buffer.id == id).map(|buffer| buffer.editor.clone())
        else {
            return;
        };
        let text = editor.read(cx).text();
        if text.len() < value.range.end || text.slice(value.range.clone()).to_string() != original {
            return;
        }
        let rgb = color.to_rgb();
        let replacement = color_values::format([rgb.r, rgb.g, rgb.b, rgb.a], &value.format);
        let caret = value.range.start + replacement.len();
        editor.update(cx, |state, cx| {
            state.focus(window, cx);
            state.set_selected_range(value.range.clone(), cx);
            state.replace(replacement, window, cx);
            state.set_selected_range(caret..caret, cx);
        });
    }

    /// The editor with a clickable swatch after each line that has colors in it.
    pub(super) fn render_with_swatches(&self, editor: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        let buffer = self.active_buffer();
        if !self.settings.color_swatches || buffer.colors.is_empty() {
            return editor;
        }
        let state = buffer.editor.clone();
        let colors = buffer.colors.clone();
        let id = buffer.id;
        let this = cx.entity().downgrade();
        let border = cx.theme().foreground.opacity(0.3);
        let backdrop = cx.theme().background;

        // Kit stores the editor's layout while painting it, so the swatches are
        // placed in this canvas's paint, which runs after the editor's.
        let overlay = canvas(
            // Normal hitboxes don't block the editor underneath; this one only
            // tells whether a dialog or menu is covering the editor.
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |bounds, hitbox, window, cx| {
                let swatches = layout_swatches(&state.read(cx), &colors, bounds);
                if swatches.is_empty() {
                    return;
                }
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    for (rect, value) in &swatches {
                        let swatch = fill(*rect, backdrop).corner_radii(px(2.));
                        window.paint_quad(swatch);
                        let swatch = fill(*rect, to_hsla(value.rgba))
                            .corner_radii(px(2.))
                            .border_widths(px(1.))
                            .border_color(border);
                        window.paint_quad(swatch);
                    }
                });

                let over = |position: &Point<Pixels>| swatches.iter().any(|(rect, _)| rect.contains(position));
                let hovering = hitbox.is_hovered(window) && over(&window.mouse_position());
                if hovering {
                    window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
                }
                let rects: Vec<_> = swatches.iter().map(|(rect, _)| *rect).collect();
                let move_hitbox = hitbox.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
                    let now = move_hitbox.is_hovered(window) && rects.iter().any(|rect| rect.contains(&event.position));
                    if phase == DispatchPhase::Bubble && now != hovering {
                        window.refresh();
                    }
                });
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble || event.button != MouseButton::Left || !hitbox.is_hovered(window) {
                        return;
                    }
                    let Some((_, value)) = swatches.iter().find(|(rect, _)| rect.contains(&event.position)) else {
                        return;
                    };
                    cx.stop_propagation();
                    let value = value.clone();
                    _ = this.update(cx, |workspace, cx| workspace.open_color_picker(id, value, window, cx));
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        div()
            .relative()
            .size_full()
            .child(editor)
            .child(overlay)
            .into_any_element()
    }
}

/// Where each visible line's swatches go: just past the line's last character,
/// on its last wrapped row, vertically centered.
pub(super) fn layout_swatches(
    state: &gpui_kit::component::input::EditorState,
    lines: &[ColorLine],
    bounds: Bounds<Pixels>,
) -> Vec<Swatch> {
    let Some(line_height) = state.line_height() else {
        return Vec::new();
    };
    let side = px((f32::from(line_height) * SWATCH_SCALE).round());
    let scroll_x = state.scroll_offset().x;
    let mut swatches = Vec::new();
    let mut seen_visible = false;

    for line in lines {
        // Kit maps offsets above the viewport (or inside a fold) onto the start
        // of the first row it drew, so a line whose start and end land on the
        // same spot isn't on screen. Offsets below the viewport map to nothing.
        let (Some(start), Some(end)) = (
            state.range_to_bounds(&(line.start..line.start)),
            state.range_to_bounds(&(line.end..line.end)),
        ) else {
            if seen_visible {
                break;
            }
            continue;
        };
        if start.origin == end.origin {
            continue;
        }
        seen_visible = true;

        // Swatches scrolled left past the text's edge would sit on the line numbers.
        let text_left = start.origin.x - scroll_x;
        let y = end.origin.y + (line_height - side) / 2.;
        let mut x = end.origin.x + px(LINE_GAP);
        for value in &line.colors {
            let rect = Bounds::new(point(x, y), size(side, side));
            if x >= text_left && bounds.contains(&rect.center()) {
                swatches.push((rect, value.clone()));
            }
            x += side + px(SWATCH_GAP);
        }
    }
    swatches
}

fn to_hsla([r, g, b, a]: color_values::Rgba) -> Hsla {
    Rgba { r, g, b, a }.into()
}
