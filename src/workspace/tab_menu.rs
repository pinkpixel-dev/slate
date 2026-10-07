use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::color_picker::{ColorPickerState, ColorSelect};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;

use super::Workspace;
use super::buffer::BufferId;
use crate::tab_color::TabColor;

actions!(slate, [CopyPath, RevealInFileManager]);

type MenuBuilder = Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

impl Workspace {
    /// The right-click menu for a tab: closing, plus its color.
    pub(super) fn tab_menu_builder(&self, id: BufferId, _: &mut Window, cx: &mut Context<Self>) -> MenuBuilder {
        let this = cx.entity().downgrade();
        let current = self
            .buffers
            .iter()
            .find(|buffer| buffer.id == id)
            .and_then(|buffer| buffer.color.clone());
        let many_tabs = self.buffers.len() > 1;
        let has_path = self.path_of(id).is_some();

        Box::new(move |menu, window, cx| {
            let close = this.clone();
            let close_others = this.clone();
            let colors = this.clone();
            let copy = this.clone();
            let reveal = this.clone();
            let current = current.clone();

            menu.item(PopupMenuItem::new("Close").on_click(move |_, window, cx| {
                _ = close.update(cx, |workspace, cx| workspace.close_buffer(id, window, cx));
            }))
            .item(
                PopupMenuItem::new("Close Others")
                    .disabled(!many_tabs)
                    .on_click(move |_, window, cx| {
                        _ = close_others.update(cx, |workspace, cx| workspace.close_others(id, window, cx));
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new("Copy Path")
                    .disabled(!has_path)
                    .on_click(move |_, window, cx| {
                        _ = copy.update(cx, |workspace, cx| workspace.copy_path(id, window, cx));
                    }),
            )
            .item(
                PopupMenuItem::new("Reveal in File Manager")
                    .disabled(!has_path)
                    .on_click(move |_, _, cx| {
                        _ = reveal.update(cx, |workspace, cx| workspace.reveal(id, cx));
                    }),
            )
            .separator()
            .submenu("Tab Color", window, cx, move |menu, _, _| {
                color_menu(menu, colors.clone(), id, current.clone())
            })
        })
    }

    fn path_of(&self, id: BufferId) -> Option<std::path::PathBuf> {
        let buffer = self.buffers.iter().find(|buffer| buffer.id == id)?;
        buffer.document.path().map(|path| path.to_path_buf())
    }

    fn copy_path(&mut self, id: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.path_of(id) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string()));
        window.push_notification(Notification::info("Path copied"), cx);
    }

    /// Opens the file's folder with the file selected (through the desktop
    /// portal, falling back to `xdg-open` on the folder).
    fn reveal(&mut self, id: BufferId, cx: &mut Context<Self>) {
        if let Some(path) = self.path_of(id) {
            cx.reveal_path(&path);
        }
    }

    pub(super) fn copy_active_path(&mut self, _: &CopyPath, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        self.copy_path(id, window, cx);
    }

    pub(super) fn reveal_active(&mut self, _: &RevealInFileManager, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        self.reveal(id, cx);
    }

    fn close_others(&mut self, keep: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let (dirty, clean): (Vec<_>, Vec<_>) = self
            .buffers
            .iter()
            .filter(|buffer| buffer.id != keep)
            .map(|buffer| (buffer.id, buffer.document.is_dirty()))
            .partition(|(_, dirty)| *dirty);
        for (id, _) in clean {
            self.remove_buffer(id, window, cx);
        }
        // Unsaved tabs stay open; only the first one asks, so the dialog
        // always matches the tab it's about.
        if let Some((id, _)) = dirty.first() {
            self.close_buffer(*id, window, cx);
        } else if let Some(index) = self.index_of(keep) {
            self.activate(index, window, cx);
        }
    }

    fn open_custom_color(&mut self, id: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let start = self
            .index_of(id)
            .and_then(|index| self.tab_hsla(index, &self.buffers[index], cx))
            .unwrap_or(cx.theme().primary);
        let picker = cx.new(|cx| ColorPickerState::new(window, cx).default_value(start));
        let this = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _, cx| {
            let apply = this.clone();
            let picker_for_apply = picker.clone();
            let featured = TabColor::PRESETS.iter().map(|color| color.resolve(cx.theme())).collect();

            dialog
                .w(px(360.))
                .title("Custom Tab Color")
                .child(
                    v_flex()
                        .gap_2()
                        .child(ColorSelect::new(&picker).featured_colors(featured)),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("color-cancel")
                                .outline()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("color-apply")
                                .primary()
                                .label("Apply")
                                .on_click(move |_, window, cx| {
                                    let color = picker_for_apply.read(cx).value();
                                    window.close_dialog(cx);
                                    if let Some(color) = color {
                                        _ = apply.update(cx, |workspace, cx| {
                                            workspace.set_buffer_color(id, Some(TabColor::custom(color)), cx)
                                        });
                                    }
                                }),
                        ),
                )
        });
    }
}

fn color_menu(
    menu: PopupMenu,
    this: WeakEntity<Workspace>,
    id: BufferId,
    current: Option<TabColor>,
) -> PopupMenu {
    let none = this.clone();
    let mut menu = menu.item(
        PopupMenuItem::new("Automatic")
            .checked(current.is_none())
            .on_click(move |_, _, cx| {
                _ = none.update(cx, |workspace, cx| workspace.set_buffer_color(id, None, cx));
            }),
    );

    for preset in TabColor::PRESETS {
        let set = this.clone();
        let checked = current.as_ref() == Some(&preset);
        let swatch = preset.clone();
        menu = menu.item(
            PopupMenuItem::element(move |_, cx| {
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().size_3().rounded_full().bg(swatch.resolve(cx.theme())))
                    .child(swatch.label().to_string())
            })
            .checked(checked)
            .on_click(move |_, _, cx| {
                let color = preset.clone();
                _ = set.update(cx, |workspace, cx| workspace.set_buffer_color(id, Some(color), cx));
            }),
        );
    }

    let custom = this;
    menu.item(
        PopupMenuItem::new("Custom…")
            .checked(matches!(current, Some(TabColor::Custom(_))))
            .on_click(move |_, window, cx| {
                _ = custom.update(cx, |workspace, cx| workspace.open_custom_color(id, window, cx));
            }),
    )
}
