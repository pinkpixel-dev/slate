use gpui_kit::component::InteractiveElementExt as _;
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::Workspace;
use super::buffer::{Buffer, BufferId};

const TAB_HEIGHT: f32 = 34.;
const TAB_MAX_WIDTH: f32 = 220.;
/// Height of the colored line along the top of a tab.
const ACCENT_HEIGHT: f32 = 2.;

/// The payload carried while a tab is being dragged, also used as its drag preview.
#[derive(Clone)]
pub(super) struct DraggedTab {
    id: BufferId,
    label: SharedString,
    accent: Option<Hsla>,
}

impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .h(px(TAB_HEIGHT))
            .px_3()
            .flex()
            .items_center()
            .text_sm()
            .bg(cx.theme().tab_active)
            .text_color(cx.theme().tab_active_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .when_some(self.accent, |this, color| this.child(accent_line(color)))
            .child(self.label.clone())
    }
}

fn accent_line(color: Hsla) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(ACCENT_HEIGHT))
        .bg(color)
}

impl Workspace {
    pub(super) fn render_tab_strip(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = Vec::with_capacity(self.buffers.len());
        for (index, buffer) in self.buffers.iter().enumerate() {
            tabs.push(self.render_tab(index, buffer, window, cx));
        }
        let theme = cx.theme();

        h_flex()
            .id("tab-strip")
            .flex_shrink_0()
            .w_full()
            .h(px(TAB_HEIGHT))
            .bg(theme.tab_bar)
            .border_b_1()
            .border_color(theme.border)
            .overflow_x_scroll()
            .track_scroll(&self.tab_scroll)
            .children(tabs)
            // The empty space after the last tab: drop here to move a tab to the end,
            // double-click for a new file.
            .child(
                div()
                    .id("tab-strip-end")
                    .flex_1()
                    .min_w(px(32.))
                    .h_full()
                    .drag_over::<DraggedTab>(|style, _, _, cx| style.bg(cx.theme().drop_target))
                    .on_drop(cx.listener(|this, dragged: &DraggedTab, window, cx| {
                        this.move_buffer(dragged.id, this.buffers.len(), window, cx);
                    }))
                    .on_double_click(cx.listener(|this, _, window, cx| this.new_untitled(window, cx))),
            )
    }

    fn render_tab(&self, index: usize, buffer: &Buffer, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let id = buffer.id;
        let selected = index == self.active;
        let dirty = buffer.document.is_dirty();
        let accent = self.tab_hsla(buffer, cx);
        let label: SharedString = buffer.document.display_name().into();
        let path: Option<SharedString> = buffer
            .document
            .path()
            .map(|path| path.display().to_string().into());
        let group: SharedString = format!("tab-{}", id.0).into();

        div()
            .id(("tab", id.0))
            .group(group.clone())
            .relative()
            .flex_shrink_0()
            .max_w(px(TAB_MAX_WIDTH))
            .h_full()
            .pl_3()
            .pr_1p5()
            .flex()
            .items_center()
            .gap_1p5()
            .text_sm()
            .border_r_1()
            .border_color(theme.border)
            .cursor_pointer()
            .when(selected, |this| {
                this.bg(theme.tab_active).text_color(theme.tab_active_foreground)
            })
            .when(!selected, |this| {
                this.text_color(theme.tab_foreground)
                    .hover(|style| style.bg(theme.secondary_hover))
            })
            .when_some(accent, |this, color| this.child(accent_line(color)))
            .child(div().truncate().child(label.clone()))
            .child(self.render_tab_close(id, selected, dirty, &group, cx))
            .when_some(path, |this, path| {
                this.tooltip(move |window, cx| Tooltip::new(path.clone()).build(window, cx))
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(index) = this.index_of(id) {
                    this.activate(index, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, window, cx| this.close_buffer(id, window, cx)),
            )
            .on_drag(
                DraggedTab { id, label, accent },
                |dragged, _, _, cx| cx.new(|_| dragged.clone()),
            )
            .drag_over::<DraggedTab>(|style, _, _, cx| style.bg(cx.theme().drop_target))
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, window, cx| {
                this.move_buffer(dragged.id, index, window, cx);
            }))
            .context_menu(self.tab_menu_builder(id, window, cx))
            .into_any_element()
    }

    /// The close button. Unsaved tabs show a dot that turns into the close
    /// button on hover; other tabs show it when active or hovered.
    fn render_tab_close(
        &self,
        id: BufferId,
        selected: bool,
        dirty: bool,
        group: &SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let show_close = selected && !dirty;

        div()
            .relative()
            .flex_shrink_0()
            .size_5()
            .when(dirty, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .group_hover(group.clone(), |style| style.opacity(0.))
                        .child(div().size_2().rounded_full().bg(theme.tab_active_foreground)),
                )
            })
            .child(
                div()
                    .id(("tab-close", id.0))
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .opacity(if show_close { 1. } else { 0. })
                    .group_hover(group.clone(), |style| style.opacity(1.))
                    .hover(|style| style.bg(theme.secondary_hover))
                    .child(Icon::new(IconName::Close).xsmall())
                    .tooltip(|window, cx| Tooltip::new("Close").build(window, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_buffer(id, window, cx);
                    })),
            )
    }
}
