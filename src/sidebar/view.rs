use std::path::Path;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::tree::tree;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::breadcrumb::{Crumb, crumbs, display_path, home_dir};
use super::{Sidebar, SidebarEvent};
use crate::file_tree::RowKind;

const INDENT: f32 = 14.;

impl Sidebar {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (icon, tooltip) = if self.show_hidden {
            (IconName::EyeOff, "Hide Hidden Files")
        } else {
            (IconName::Eye, "Show Hidden Files")
        };

        h_flex()
            .h(px(34.))
            .flex_shrink_0()
            .pl_1()
            .pr_1()
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.render_path_bar(cx))
            .child(
                Button::new("header-open-folder")
                    .ghost()
                    .xsmall()
                    .icon(IconName::FolderOpen)
                    .accessibility_label("Open Folder")
                    .tooltip("Open Folder")
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(crate::workspace::OpenFolder), cx)
                    }),
            )
            .child(
                Button::new("toggle-hidden")
                    .ghost()
                    .xsmall()
                    .icon(icon)
                    .accessibility_label(tooltip)
                    .tooltip(tooltip)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
            )
            .child(
                Button::new("close-folder")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .accessibility_label("Close Folder")
                    .tooltip("Close Folder")
                    .on_click(cx.listener(|this, _, _, cx| this.close_folder(cx))),
            )
    }

    /// `… › parent › root`: the parent is a click away, and the menu behind
    /// `…` lists every folder above the root, so the keyboard can reach them too.
    fn render_path_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let home = home_dir();
        let root = self.root().map(Path::to_path_buf).unwrap_or_default();
        let mut crumbs = crumbs(&root, home.as_deref());
        let current = crumbs.pop();
        let parent = crumbs.last().cloned();
        let theme = cx.theme();
        let separator = || {
            Icon::new(IconName::ChevronRight)
                .size_3()
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
        };

        let this = cx.entity().downgrade();
        h_flex()
            .flex_1()
            .min_w_0()
            .gap_0p5()
            .items_center()
            .text_sm()
            .when(!crumbs.is_empty(), |bar| {
                let ancestors: Vec<Crumb> = crumbs.iter().rev().cloned().collect();
                let home = home.clone();
                bar.child(
                    Button::new("parent-folders")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Ellipsis)
                        .accessibility_label("Parent Folders")
                        .tooltip("Parent Folders")
                        .dropdown_menu(move |mut menu, _, _| {
                            for crumb in &ancestors {
                                let go = this.clone();
                                let dir = crumb.path.clone();
                                menu = menu.item(
                                    PopupMenuItem::new(display_path(&crumb.path, home.as_deref())).on_click(
                                        move |_, _, cx| {
                                            let dir = dir.clone();
                                            _ = go.update(cx, |sidebar, cx| sidebar.navigate_to(dir, cx));
                                        },
                                    ),
                                );
                            }
                            menu.scrollable(true)
                        }),
                )
            })
            .when_some(parent, |bar, parent| {
                let go = cx.entity().downgrade();
                let tip = display_path(&parent.path, home.as_deref());
                bar.when(crumbs.len() > 1, |bar| bar.child(separator()))
                    .child(
                        div()
                            .id("crumb-parent")
                            .min_w(px(20.))
                            .max_w(px(96.))
                            .px_0p5()
                            .rounded(theme.radius)
                            .truncate()
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .hover(|this| this.text_color(theme.foreground))
                            .child(parent.label.clone())
                            .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                            .on_click(move |_, _, cx| {
                                let dir = parent.path.clone();
                                _ = go.update(cx, |sidebar, cx| sidebar.navigate_to(dir, cx));
                            }),
                    )
                    .child(separator())
            })
            .when_some(current, |bar, current| {
                let tip = display_path(&current.path, home.as_deref());
                bar.child(
                    div()
                        .id("crumb-root")
                        .min_w_0()
                        .px_0p5()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(current.label)
                        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx)),
                )
            })
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

            let content = h_flex()
                .id(ElementId::Name(id.clone()))
                .flex_1()
                .min_w_0()
                .gap_2()
                .items_center()
                .text_sm()
                .when(muted, |this| this.text_color(theme.muted_foreground))
                .children(icon.map(|icon| {
                    Icon::new(icon)
                        .small()
                        .text_color(theme.muted_foreground)
                }))
                .child(div().truncate().child(entry.item().label.clone()));
            // Folders get "Open as Root" on right-click.
            let content = match &kind {
                Some(RowKind::Folder(dir)) => {
                    let go = sidebar.clone();
                    let dir = dir.clone();
                    content
                        .context_menu(move |menu, _, _| {
                            let go = go.clone();
                            let dir = dir.clone();
                            menu.item(PopupMenuItem::new("Open as Root").on_click(move |_, _, cx| {
                                let dir = dir.clone();
                                _ = go.update(cx, |sidebar, cx| sidebar.navigate_to(dir, cx));
                            }))
                        })
                        .into_any_element()
                }
                _ => content.into_any_element(),
            };

            let open = sidebar.clone();
            ListItem::new(ix)
                .selected(selected)
                .disabled(entry.is_disabled())
                .pl(indent)
                .py_0p5()
                .child(content)
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
