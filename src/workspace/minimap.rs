use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::component::input::RopeExt as _;
use gpui_kit::*;

use super::buffer::BufferId;
use super::{KEY_CONTEXT, Workspace};
use crate::minimap::{self, CHAR_WIDTH, LINE_HEIGHT, Run, Viewport};

actions!(slate, [ToggleMinimap]);

const WIDTH: f32 = 96.;
const PADDING_LEFT: f32 = 8.;
/// How long typing has to pause before the minimap is rebuilt.
const REFRESH_DELAY: Duration = Duration::from_millis(300);
/// Past this size, the minimap skips syntax colors to stay quick.
const MAX_HIGHLIGHT_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-m", ToggleMinimap, Some(KEY_CONTEXT))]);
}

/// One tab's minimap: the lines drawn last time, and its own highlighter
/// (Kit's editor doesn't share its syntax tree).
#[derive(Default)]
pub(super) struct MinimapState {
    lines: Rc<Vec<Vec<Run>>>,
    /// The highlighter is moved into the background task while it parses.
    highlighter: Option<SyntaxHighlighter>,
    /// The text or theme changed since `lines` was built.
    stale: bool,
    /// The language and highlight theme (by address) `lines` was built with.
    built_for: Option<(SharedString, usize)>,
    refresh: Option<Task<()>>,
}

impl MinimapState {
    #[cfg(test)]
    pub fn lines(&self) -> Rc<Vec<Vec<Run>>> {
        self.lines.clone()
    }
}

/// The box being dragged, as the distance from its top edge to the pointer.
#[derive(Clone, Copy)]
pub(super) struct MinimapDrag(f32);

impl Workspace {
    /// Shown when the setting is on and word wrap is off: wrapped rows aren't
    /// exposed by Kit, so the viewport box would drift with wrap on.
    pub(super) fn minimap_visible(&self) -> bool {
        self.settings.show_minimap && !self.settings.word_wrap
    }

    pub(super) fn toggle_minimap(&mut self, _: &ToggleMinimap, _: &mut Window, cx: &mut Context<Self>) {
        self.set_show_minimap(!self.settings.show_minimap, cx);
    }

    pub(super) fn set_show_minimap(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.show_minimap = show;
        self.save_settings();
        self.ensure_minimap(cx);
        cx.notify();
    }

    /// The tab's text changed: rebuild its minimap once typing pauses.
    pub(super) fn mark_minimap_stale(&mut self, id: BufferId, cx: &mut Context<Self>) {
        if let Some(buffer) = self.buffer_mut(id) {
            buffer.minimap.stale = true;
        }
        if self.active_buffer().id == id && self.minimap_visible() {
            self.refresh_minimap(id, REFRESH_DELAY, cx);
        }
    }

    /// Builds the active tab's minimap if it's showing and out of date, for
    /// example after switching tabs or themes.
    pub(super) fn ensure_minimap(&mut self, cx: &mut Context<Self>) {
        if !self.minimap_visible() {
            return;
        }
        let key = (self.active_buffer().document.language().id.into(), theme_key(cx));
        let minimap = &self.active_buffer().minimap;
        if minimap.stale || minimap.built_for.as_ref() != Some(&key) {
            let id = self.active_buffer().id;
            self.refresh_minimap(id, Duration::ZERO, cx);
        }
    }

    fn refresh_minimap(&mut self, id: BufferId, delay: Duration, cx: &mut Context<Self>) {
        let language: SharedString = match self.buffers.iter().find(|buffer| buffer.id == id) {
            Some(buffer) => buffer.document.language().id.into(),
            None => return,
        };
        let theme = cx.theme();
        let highlight_theme = Arc::clone(&theme.highlight_theme);
        let foreground = theme.foreground.opacity(0.55);
        let key = (language.clone(), theme_key(cx));

        let task = cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                cx.background_executor().timer(delay).await;
            }
            // Take the text and the highlighter, then parse off the UI thread.
            let Ok(Some((text, highlighter))) = this.update(cx, |workspace, cx| {
                let buffer = workspace.buffer_mut(id)?;
                let highlighter = buffer
                    .minimap
                    .highlighter
                    .take()
                    .filter(|highlighter| *highlighter.language() == language);
                Some((buffer.editor.read(cx).text().clone(), highlighter))
            }) else {
                return;
            };
            let built = cx
                .background_spawn(async move {
                    let source = text.to_string();
                    if source.len() > MAX_HIGHLIGHT_BYTES {
                        return (minimap::build_lines(&source, &[], foreground), highlighter);
                    }
                    let mut highlighter = highlighter.unwrap_or_else(|| SyntaxHighlighter::new(&language));
                    highlighter.update(None, &text, None);
                    let styles = highlighter.styles(&(0..source.len()), &*highlight_theme);
                    let styles: Vec<_> = styles
                        .into_iter()
                        .map(|(range, mut style)| {
                            style.color = style.color.map(|color| color.opacity(0.8));
                            (range, style)
                        })
                        .collect();
                    (minimap::build_lines(&source, &styles, foreground), Some(highlighter))
                })
                .await;
            _ = this.update(cx, |workspace, cx| {
                let Some(buffer) = workspace.buffer_mut(id) else {
                    return;
                };
                let (lines, highlighter) = built;
                buffer.minimap.lines = Rc::new(lines);
                buffer.minimap.highlighter = highlighter;
                buffer.minimap.stale = false;
                buffer.minimap.built_for = Some(key);
                buffer.minimap.refresh = None;
                cx.notify();
            });
        });
        if let Some(buffer) = self.buffer_mut(id) {
            buffer.minimap.refresh = Some(task);
        }
    }

    /// The active editor's position in rows, or `None` before its first layout.
    fn minimap_viewport(&self, cx: &App) -> Option<(Viewport, Pixels)> {
        let buffer = self.active_buffer();
        let editor = buffer.editor.read(cx);
        let line_height = editor.line_height()?;
        let top = (-editor.scroll_offset().y / line_height).max(0.);
        let visible = editor.input_bounds().size.height / line_height;
        let total = buffer.minimap.lines.len().max(editor.text().lines_len()) as f32;
        Some((Viewport { total, top, visible }, line_height))
    }

    fn scroll_editor_to(&mut self, top: f32, cx: &mut Context<Self>) {
        let Some((_, line_height)) = self.minimap_viewport(cx) else {
            return;
        };
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            let x = state.scroll_offset().x;
            state.set_scroll_offset(point(x, -(line_height * top)), cx);
        });
    }

    pub(super) fn on_minimap_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let bounds = self.minimap_bounds.get();
        let Some((viewport, _)) = self.minimap_viewport(cx) else {
            return;
        };
        let pane = f32::from(bounds.size.height);
        let y = f32::from(event.position.y - bounds.origin.y);
        let view = viewport.view(pane);
        let grab = if (view.box_top..view.box_top + view.box_height).contains(&y) {
            y - view.box_top
        } else {
            // Clicking outside the box centers that spot, then drags from the middle.
            self.scroll_editor_to(viewport.top_centering(y, pane), cx);
            view.box_height / 2.
        };
        self.minimap_drag = Some(MinimapDrag(grab));
        cx.notify();
    }

    fn on_minimap_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(MinimapDrag(grab)) = self.minimap_drag else {
            return;
        };
        if !event.dragging() {
            self.minimap_drag = None;
            return;
        }
        let Some((viewport, _)) = self.minimap_viewport(cx) else {
            return;
        };
        let bounds = self.minimap_bounds.get();
        let y = f32::from(event.position.y - bounds.origin.y);
        let top = viewport.top_for_box(y - grab, f32::from(bounds.size.height));
        self.scroll_editor_to(top, cx);
    }

    fn on_minimap_scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some((_, line_height)) = self.minimap_viewport(cx) else {
            return;
        };
        let delta = event.delta.pixel_delta(line_height);
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            let offset = state.scroll_offset();
            state.set_scroll_offset(point(offset.x, offset.y + delta.y), cx);
        });
    }

    /// The editor with the active tab's minimap along its right edge.
    pub(super) fn render_with_minimap(&self, editor: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        if !self.minimap_visible() {
            return editor;
        }
        let lines = self.active_buffer().minimap.lines.clone();
        let viewport = self.minimap_viewport(cx);
        let theme = cx.theme();
        let slider = if self.minimap_drag.is_some() {
            theme.foreground.opacity(0.16)
        } else {
            theme.foreground.opacity(0.08)
        };
        let bounds_cell = self.minimap_bounds.clone();

        let map = div()
            .id("minimap")
            .w(px(WIDTH))
            .h_full()
            .flex_none()
            .bg(theme.background)
            .border_l_1()
            .border_color(theme.border.opacity(0.5))
            .cursor_default()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event, _, cx| this.on_minimap_down(event, cx)))
            .on_mouse_move(cx.listener(|this, event, _, cx| this.on_minimap_move(event, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.minimap_drag = None;
                cx.notify();
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.minimap_drag = None;
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, event, _, cx| this.on_minimap_scroll(event, cx)))
            .child(
                canvas(
                    move |bounds, _, _| bounds_cell.set(bounds),
                    move |bounds, (), window, _| paint_minimap(bounds, &lines, viewport, slider, window),
                )
                .size_full(),
            );

        h_flex_full(editor, map.into_any_element())
    }
}

/// Kit swaps in a new highlight theme on every theme change, so its address tells themes apart.
fn theme_key(cx: &App) -> usize {
    Arc::as_ptr(&cx.theme().highlight_theme) as usize
}

fn h_flex_full(editor: AnyElement, map: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .size_full()
        .child(div().flex_1().min_w_0().h_full().child(editor))
        .child(map)
        .into_any_element()
}

fn paint_minimap(
    bounds: Bounds<Pixels>,
    lines: &[Vec<Run>],
    viewport: Option<(Viewport, Pixels)>,
    slider: Hsla,
    window: &mut Window,
) {
    let pane = f32::from(bounds.size.height);
    let view = viewport.map(|(viewport, _)| viewport.view(pane));
    let scroll = view.map_or(0., |view| view.scroll);

    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        let first = (scroll / LINE_HEIGHT).floor() as usize;
        let count = (pane / LINE_HEIGHT).ceil() as usize + 1;
        for (row, runs) in lines.iter().enumerate().skip(first).take(count) {
            let y = bounds.origin.y + px(row as f32 * LINE_HEIGHT - scroll);
            for run in runs {
                let x = bounds.origin.x + px(PADDING_LEFT + f32::from(run.column) * CHAR_WIDTH);
                let size = size(px(f32::from(run.len) * CHAR_WIDTH), px(LINE_HEIGHT - 0.5));
                window.paint_quad(fill(Bounds::new(point(x, y), size), run.color));
            }
        }
        if let Some(view) = view {
            let top = bounds.origin.y + px(view.box_top);
            let slider_bounds = Bounds::new(point(bounds.origin.x, top), size(bounds.size.width, px(view.box_height)));
            window.paint_quad(fill(slider_bounds, slider));
        }
    });
}
