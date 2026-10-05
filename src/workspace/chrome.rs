use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Selectable as _, Sizable as _, TitleBar, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{KEY_CONTEXT, NewFile, Open, Save, TAB_SIZE, ToggleWhitespace, Workspace};

impl Workspace {
    pub(super) fn render_title_bar(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let dirty = self.document.is_dirty();

        TitleBar::new()
            .on_close_window(move |_, window, cx| {
                let close_now = this
                    .update(cx, |workspace, cx| workspace.should_close(window, cx))
                    .unwrap_or(true);
                if close_now {
                    window.remove_window();
                }
            })
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Button::new("new-file")
                            .ghost()
                            .small()
                            .icon(Icon::new(IconName::FilePlus))
                            .tooltip_with_action("New File", &NewFile, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(NewFile), cx)),
                    )
                    .child(
                        Button::new("open")
                            .ghost()
                            .small()
                            .icon(Icon::new(IconName::FolderOpen))
                            .tooltip_with_action("Open", &Open, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(Open), cx)),
                    )
                    .child(
                        Button::new("save")
                            .ghost()
                            .small()
                            .icon(Icon::new(IconName::Save))
                            .tooltip_with_action("Save", &Save, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(Save), cx)),
                    )
                    .child(
                        h_flex()
                            .ml_2()
                            .gap_1p5()
                            .items_center()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(self.document.display_name())
                            .when(dirty, |this| {
                                this.child(
                                    div()
                                        .id("unsaved")
                                        .size_1p5()
                                        .rounded_full()
                                        .bg(cx.theme().primary)
                                        .tooltip(|window, cx| {
                                            gpui_kit::component::tooltip::Tooltip::new("Unsaved changes")
                                                .build(window, cx)
                                        }),
                                )
                            }),
                    ),
            )
    }

    pub(super) fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        let position = self.editor.read(cx).cursor_position();
        let cursor = format!("Ln {}, Col {}", position.line + 1, position.character + 1);

        StatusBar::new()
            .text_color(cx.theme().muted_foreground)
            .left(self.document.language().label)
            .right(
                Button::new("toggle-whitespace")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::Pilcrow))
                    .selected(self.show_whitespace)
                    .tooltip("Show Whitespace")
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleWhitespace), cx)),
            )
            .right(cursor)
            .right(format!("Spaces: {TAB_SIZE}"))
            .right("UTF-8")
    }
}
