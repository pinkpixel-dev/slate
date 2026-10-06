use std::path::PathBuf;

use gpui_kit::component::button::Button;
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_kit::component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings as SettingsView,
};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, WindowExt as _};
use gpui_kit::*;

use super::chrome::display_dir;
use super::{OpenSettings, Workspace};
use crate::storage::{Settings, TabColorMode};
use crate::theme::{DEFAULT_THEME_NAME, FontLists, ThemeCatalog};

type FontSelect = Entity<SelectState<SearchableVec<SharedString>>>;

const PANEL_WIDTH: f32 = 680.;
const MIN_FONT_SIZE: f64 = 8.;
const MAX_FONT_SIZE: f64 = 32.;

/// The searchable font pickers. Built the first time the panel opens, since
/// listing fonts asks fontconfig.
pub(super) struct FontPickers {
    pub(super) ui: FontSelect,
    pub(super) editor: FontSelect,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    /// Opens the settings sheet, or closes it if it's already open.
    pub(super) fn open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_sheet(cx) {
            window.close_sheet(cx);
            return;
        }
        if self.font_pickers.is_none() {
            self.font_pickers = Some(self.build_font_pickers(window, cx));
        }
        let Some(pickers) = self.font_pickers.as_ref() else {
            return;
        };
        let (ui_fonts, editor_fonts) = (pickers.ui.clone(), pickers.editor.clone());
        let this = cx.entity().downgrade();

        window.open_sheet(cx, move |sheet, _, cx| {
            sheet
                .title("Settings")
                .size(px(PANEL_WIDTH))
                .child(settings_view(&this, &ui_fonts, &editor_fonts, cx))
        });
    }

    fn build_font_pickers(&mut self, window: &mut Window, cx: &mut Context<Self>) -> FontPickers {
        let fonts = FontLists::installed(cx);
        let ui = font_select(fonts.all, self.settings.ui_font.clone(), window, cx);
        let editor = font_select(fonts.mono, self.settings.editor_font.clone(), window, cx);

        let subscriptions = vec![
            cx.subscribe_in(&ui, window, |this, _, event: &SelectEvent<_>, _, cx| {
                let SelectEvent::Confirm(font) = event;
                let font = font.as_ref().map(ToString::to_string);
                this.update_appearance(cx, |settings| settings.ui_font = font);
            }),
            cx.subscribe_in(&editor, window, |this, _, event: &SelectEvent<_>, _, cx| {
                let SelectEvent::Confirm(font) = event;
                let font = font.as_ref().map(ToString::to_string);
                this.update_appearance(cx, |settings| settings.editor_font = font);
            }),
        ];
        FontPickers {
            ui,
            editor,
            _subscriptions: subscriptions,
        }
    }
}

fn font_select(
    fonts: Vec<SharedString>,
    current: Option<String>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> FontSelect {
    cx.new(|cx| {
        let mut state = SelectState::new(SearchableVec::new(fonts), None, window, cx).searchable(true);
        if let Some(font) = current {
            state.set_selected_value(&SharedString::from(font), window, cx);
        }
        state
    })
}

/// A copy of the workspace's settings, for the panel's getters.
fn current(this: &WeakEntity<Workspace>, cx: &App) -> Settings {
    this.upgrade()
        .map(|workspace| workspace.read(cx).settings.clone())
        .unwrap_or_default()
}

fn edit(this: &WeakEntity<Workspace>, cx: &mut App, change: impl FnOnce(&mut Settings)) {
    _ = this.update(cx, |workspace, cx| workspace.update_appearance(cx, change));
}

fn settings_view(this: &WeakEntity<Workspace>, ui_fonts: &FontSelect, editor_fonts: &FontSelect, cx: &App) -> SettingsView {
    SettingsView::new("slate-settings")
        .sidebar_width(px(150.))
        .pages([
            SettingPage::new("Appearance")
                .default_open(true)
                .groups([theme_group(this, cx), font_group(this, ui_fonts, editor_fonts)]),
            SettingPage::new("General").groups(general_groups(this)),
        ])
}

fn theme_group(this: &WeakEntity<Workspace>, cx: &App) -> SettingGroup {
    let catalog = ThemeCatalog::global(cx);
    let themes = catalog.names().into_iter().map(|name| (name.clone(), name)).collect();
    let dir = catalog.custom_dir().to_path_buf();

    let get = this.clone();
    let set = this.clone();
    let theme = SettingField::scrollable_dropdown(
        themes,
        move |cx| {
            let theme = current(&get, cx).theme;
            theme.unwrap_or_else(|| DEFAULT_THEME_NAME.into()).into()
        },
        move |name: SharedString, cx| edit(&set, cx, |settings| settings.theme = Some(name.to_string())),
    )
    .default_value(DEFAULT_THEME_NAME);

    SettingGroup::new().title("Theme").items([
        SettingItem::new("Theme", theme),
        SettingItem::new("Custom themes", open_folder_field(dir.clone())).description(display_dir(&dir)),
    ])
}

fn open_folder_field(dir: PathBuf) -> SettingField<SharedString> {
    SettingField::render(move |_, _, _| {
        let dir = dir.clone();
        Button::new("open-themes-folder")
            .outline()
            .small()
            .label("Open Folder")
            .on_click(move |_, _, cx| cx.open_with_system(&dir))
    })
}

fn font_group(this: &WeakEntity<Workspace>, ui_fonts: &FontSelect, editor_fonts: &FontSelect) -> SettingGroup {
    SettingGroup::new().title("Fonts").items([
        SettingItem::new(
            "Interface font",
            font_field(this, ui_fonts, |settings| &mut settings.ui_font),
        ),
        SettingItem::new(
            "Interface size",
            size_field(this, |theme| theme.font_size, |settings| &mut settings.ui_font_size),
        ),
        SettingItem::new(
            "Editor font",
            font_field(this, editor_fonts, |settings| &mut settings.editor_font),
        ),
        SettingItem::new(
            "Editor size",
            size_field(this, |theme| theme.mono_font_size, |settings| &mut settings.editor_font_size),
        ),
    ])
}

fn font_field(
    this: &WeakEntity<Workspace>,
    picker: &FontSelect,
    slot: fn(&mut Settings) -> &mut Option<String>,
) -> SettingField<SharedString> {
    let render_picker = picker.clone();
    let reset_picker = picker.clone();
    let get = this.clone();
    let set = this.clone();

    SettingField::render(move |_, _, _| {
        Select::new(&render_picker)
            .w(px(220.))
            .menu_max_h(px(320.))
            .placeholder("Theme default")
            .search_placeholder("Search fonts")
    })
    .on_reset(
        move |cx| slot(&mut current(&get, cx)).is_some(),
        move |window, cx| {
            reset_picker.update(cx, |picker, cx| picker.set_selected_index(None, window, cx));
            edit(&set, cx, |settings| *slot(settings) = None);
        },
    )
}

fn size_field(
    this: &WeakEntity<Workspace>,
    effective: fn(&gpui_kit::component::Theme) -> Pixels,
    slot: fn(&mut Settings) -> &mut Option<f32>,
) -> SettingField<f64> {
    let get = this.clone();
    let set = this.clone();
    let dirty = this.clone();
    let reset = this.clone();
    let options = NumberFieldOptions {
        min: MIN_FONT_SIZE,
        max: MAX_FONT_SIZE,
        step: 1.,
    };

    SettingField::number_input(
        options,
        move |cx| slot(&mut current(&get, cx)).map_or_else(|| f32::from(effective(cx.theme())), |size| size) as f64,
        move |size, cx| edit(&set, cx, |settings| *slot(settings) = Some(size as f32)),
    )
    .on_reset(
        move |cx| slot(&mut current(&dirty, cx)).is_some(),
        move |_, cx| edit(&reset, cx, |settings| *slot(settings) = None),
    )
}

fn general_groups(this: &WeakEntity<Workspace>) -> [SettingGroup; 3] {
    let (get_mode, set_mode) = (this.clone(), this.clone());
    let tab_colors = SettingField::dropdown(
        vec![
            ("theme".into(), "Theme".into()),
            ("language".into(), "By language".into()),
            ("off".into(), "Off".into()),
        ],
        move |cx| match current(&get_mode, cx).tab_color_mode {
            TabColorMode::Theme => "theme".into(),
            TabColorMode::Language => "language".into(),
            TabColorMode::Off => "off".into(),
        },
        move |mode: SharedString, cx| {
            let mode = match mode.as_ref() {
                "language" => TabColorMode::Language,
                "off" => TabColorMode::Off,
                _ => TabColorMode::Theme,
            };
            _ = set_mode.update(cx, |workspace, cx| workspace.set_tab_color_mode(mode, cx));
        },
    )
    .default_value("theme");

    let (get_hidden, set_hidden) = (this.clone(), this.clone());
    let hidden = SettingField::switch(
        move |cx| current(&get_hidden, cx).show_hidden_files,
        move |show, cx| {
            _ = set_hidden.update(cx, |workspace, cx| workspace.set_show_hidden(show, cx));
        },
    )
    .default_value(false);

    let (get_wrap, set_wrap) = (this.clone(), this.clone());
    let wrap = SettingField::switch(
        move |cx| current(&get_wrap, cx).word_wrap,
        move |wrap, cx| {
            _ = set_wrap.update(cx, |workspace, cx| workspace.set_word_wrap(wrap, cx));
        },
    )
    .default_value(false);

    [
        SettingGroup::new().title("Editor").item(SettingItem::new("Word wrap", wrap)),
        SettingGroup::new().title("Tabs").item(SettingItem::new("Tab colors", tab_colors)),
        SettingGroup::new()
            .title("Sidebar")
            .item(SettingItem::new("Show hidden files", hidden)),
    ]
}
