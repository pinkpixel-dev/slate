use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName as CatalogIcon;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _, TitleBar, h_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{
    KEY_CONTEXT, NewFile, TogglePreview, Open, OpenSettings, Save, ToggleSidebar, ToggleWhitespace, ToggleWordWrap,
    Workspace,
};
use crate::theme::{DEFAULT_THEME_NAME, ThemeCatalog};

/// Title bar height; gpui-kit's default is 34px.
const TITLE_BAR_HEIGHT: f32 = 42.;
/// Title bar icon buttons. gpui-kit draws the icon at 75% of this.
const TITLE_BUTTON_SIZE: f32 = 24.;
/// The small "Open Recent" chevron next to Open.
const RECENT_BUTTON_SIZE: f32 = 16.;

impl Workspace {
    pub(super) fn render_title_bar(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let folder = self
            .active_buffer()
            .document
            .path()
            .and_then(Path::parent)
            .map(display_dir);

        TitleBar::new()
            .h(px(TITLE_BAR_HEIGHT))
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
                    .min_w_0()
                    .child(
                        Button::new("toggle-sidebar")
                            .ghost()
                            .with_size(px(TITLE_BUTTON_SIZE))
                            .icon(IconName::PanelLeft)
                            .selected(self.sidebar_open)
                            .accessibility_label("Toggle Sidebar")
                            .tooltip_with_action("Toggle Sidebar", &ToggleSidebar, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleSidebar), cx)),
                    )
                    .child(
                        Button::new("new-file")
                            .ghost()
                            .with_size(px(TITLE_BUTTON_SIZE))
                            .icon(Icon::new(CatalogIcon::FilePlus))
                            .accessibility_label("New File")
                            .tooltip_with_action("New File", &NewFile, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(NewFile), cx)),
                    )
                    .child(
                        Button::new("open")
                            .ghost()
                            .with_size(px(TITLE_BUTTON_SIZE))
                            .icon(Icon::new(CatalogIcon::FolderOpen))
                            .accessibility_label("Open")
                            .tooltip_with_action("Open", &Open, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(Open), cx)),
                    )
                    .child(self.render_recent_button(cx))
                    .child(
                        Button::new("save")
                            .ghost()
                            .with_size(px(TITLE_BUTTON_SIZE))
                            .icon(Icon::new(CatalogIcon::Save))
                            .accessibility_label("Save")
                            .tooltip_with_action("Save", &Save, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(Save), cx)),
                    )
                    .child(self.render_theme_button(cx))
                    .child(
                        Button::new("settings")
                            .ghost()
                            .with_size(px(TITLE_BUTTON_SIZE))
                            .icon(IconName::Settings)
                            .accessibility_label("Settings")
                            .tooltip_with_action("Settings", &OpenSettings, Some(KEY_CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenSettings), cx)),
                    )
                    .when_some(folder, |this, folder| {
                        this.child(
                            div()
                                .ml_2()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(folder),
                        )
                    }),
            )
    }

    fn render_recent_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let recent: Vec<PathBuf> = self.state.recent_files.clone();

        Button::new("recent")
            .ghost()
            .with_size(px(RECENT_BUTTON_SIZE))
            .icon(IconName::ChevronDown)
            .accessibility_label("Open Recent")
            .tooltip("Open Recent")
            .dropdown_menu(move |mut menu, _, _| {
                if recent.is_empty() {
                    return menu.item(PopupMenuItem::new("No Recent Files").disabled(true));
                }
                for path in &recent {
                    let open = this.clone();
                    let target = path.clone();
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    let dir = path.parent().map(display_dir).unwrap_or_default();
                    menu = menu.item(
                        PopupMenuItem::element(move |_, cx| {
                            h_flex()
                                .gap_3()
                                .justify_between()
                                .w_full()
                                .child(name.clone())
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(dir.clone()),
                                )
                        })
                        .on_click(move |_, window, cx| {
                            let path = target.clone();
                            _ = open.update(cx, |workspace, cx| workspace.open_path(path, false, window, cx));
                        }),
                    );
                }
                let clear = this.clone();
                menu.separator().item(PopupMenuItem::new("Clear Recent").on_click(move |_, _, cx| {
                    _ = clear.update(cx, |workspace, cx| workspace.clear_recent(cx));
                }))
            })
    }

    /// A quick theme switcher; the full list also lives in Settings.
    fn render_theme_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let active = self.settings.theme.clone().unwrap_or_else(|| DEFAULT_THEME_NAME.into());
        let themes = ThemeCatalog::global(cx).names();

        Button::new("theme")
            .ghost()
            .with_size(px(TITLE_BUTTON_SIZE))
            .icon(IconName::Palette)
            .accessibility_label("Theme")
            .tooltip("Theme")
            .dropdown_menu(move |mut menu, _, _| {
                for name in &themes {
                    let pick = this.clone();
                    let theme = name.to_string();
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(*name == active)
                            .on_click(move |_, _, cx| {
                                let theme = theme.clone();
                                _ = pick.update(cx, |workspace, cx| {
                                    workspace.update_appearance(cx, |settings| settings.theme = Some(theme));
                                });
                            }),
                    );
                }
                menu.scrollable(true)
            })
    }

    pub(super) fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let buffer = self.active_buffer();
        let position = buffer.editor.read(cx).cursor_position();
        let cursor = format!("Ln {}, Col {}", position.line + 1, position.character + 1);

        StatusBar::new()
            .text_color(cx.theme().muted_foreground)
            .left(buffer.document.language().label)
            .when(buffer.document.language().id == "markdown" || buffer.preview.is_some(), |bar| {
                bar.right(
                    Button::new("toggle-preview")
                        .ghost()
                        .xsmall()
                        .icon(IconName::BookOpen)
                        .selected(buffer.preview.is_some())
                        .accessibility_label("Markdown Preview")
                        .tooltip_with_action("Markdown Preview", &TogglePreview, Some(KEY_CONTEXT))
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(TogglePreview), cx)),
                )
            })
            .right(
                Button::new("toggle-word-wrap")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(CatalogIcon::TextWrap))
                    .selected(self.settings.word_wrap)
                    .accessibility_label("Word Wrap")
                    .tooltip_with_action("Word Wrap", &ToggleWordWrap, Some(KEY_CONTEXT))
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleWordWrap), cx)),
            )
            .right(
                Button::new("toggle-whitespace")
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(CatalogIcon::Pilcrow))
                    .selected(self.show_whitespace)
                    .accessibility_label("Show Whitespace")
                    .tooltip("Show Whitespace")
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleWhitespace), cx)),
            )
            .right(cursor)
            .right(self.render_indent_button(cx))
            .right(self.render_line_ending_button(cx))
            .right("UTF-8")
    }
}

/// A folder path for display, with the home directory shortened to `~`.
pub(super) fn display_dir(dir: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home.as_deref().and_then(|home| dir.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => dir.display().to_string(),
    }
}
