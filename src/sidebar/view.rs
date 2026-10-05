use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::tree;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{Sidebar, SidebarEvent};
use crate::file_tree::RowKind;

const INDENT: f32 = 14.;

impl Sidebar {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self
            .root()
            .and_then(|root| root.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".into());
        let (icon, tooltip) = if self.show_hidden {
            (IconName::EyeOff, "Hide Hidden Files")
        } else {
            (IconName::Eye, "Show Hidden Files")
        };

        h_flex()
            .h(px(34.))
            .flex_shrink_0()
            .pl_3()
            .pr_1()
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(name),
            )
            .child(
                Button::new("toggle-hidden")
                    .ghost()
                    .xsmall()
                    .icon(icon)
                    .tooltip(tooltip)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
            )
            .child(
                Button::new("close-folder")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip("Close Folder")
                    .on_click(cx.listener(|this, _, _, cx| this.close_folder(cx))),
            )
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .px_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("No folder open"),
            )
            .child(
                Button::new("sidebar-open-folder")
                    .outline()
                    .small()
                    .icon(IconName::FolderOpen)
                    .label("Open Folder")
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(crate::workspace::OpenFolder), cx)
                    }),
            )
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = cx.entity().downgrade();

        tree(&self.tree_state, move |ix, entry, selected, _, cx| {
            let id = entry.item().id.clone();
            let kind = sidebar
                .upgrade()
                .and_then(|sidebar| sidebar.read(cx).row_kind(&id));
            let theme = cx.theme();
            let indent = px(INDENT) * entry.depth() as f32 + px(8.);

            let (icon, muted) = match &kind {
                Some(RowKind::Folder(_)) if entry.is_expanded() => (Some(IconName::FolderOpen), false),
                Some(RowKind::Folder(_)) => (Some(IconName::FolderClosed), false),
                Some(RowKind::File(_)) => (Some(IconName::File), false),
                _ => (None, true),
            };

            let open = sidebar.clone();
            ListItem::new(ix)
                .selected(selected)
                .disabled(entry.is_disabled())
                .pl(indent)
                .py_0p5()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_sm()
                        .when(muted, |this| this.text_color(theme.muted_foreground))
                        .children(icon.map(|icon| {
                            Icon::new(icon)
                                .small()
                                .text_color(theme.muted_foreground)
                        }))
                        .child(div().truncate().child(entry.item().label.clone())),
                )
                .on_click(move |_, _, cx| {
                    if let Some(RowKind::File(path)) = &kind {
                        let path = path.clone();
                        _ = open.update(cx, |_, cx| cx.emit(SidebarEvent::OpenFile(path)));
                    }
                })
        })
        .size_full()
    }
}

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .on_action(cx.listener(Self::open_selected))
            .map(|this| {
                if self.tree.is_some() {
                    this.child(self.render_header(cx))
                        .child(div().flex_1().min_h_0().py_1().child(self.render_tree(cx)))
                } else {
                    this.child(self.render_empty(cx))
                }
            })
    }
}
