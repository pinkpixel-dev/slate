use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry};

/// Slate's own theme, embedded so the app looks right with no files on disk.
const SLATE_THEME: &str = include_str!("../themes/slate.json");
const DEFAULT_THEME_NAME: &str = "Slate Dark";

/// Registers the bundled Slate theme and makes it the active dark theme.
pub fn init(cx: &mut App) {
    if let Err(err) = ThemeRegistry::global_mut(cx).load_themes_from_str(SLATE_THEME) {
        eprintln!("slate: bundled theme failed to parse: {err}");
        return;
    }

    let Some(theme) = ThemeRegistry::global(cx)
        .themes()
        .get(DEFAULT_THEME_NAME)
        .cloned()
    else {
        eprintln!("slate: bundled theme \"{DEFAULT_THEME_NAME}\" is missing");
        return;
    };

    Theme::update(cx, |current| current.apply_config(&theme));
    Theme::change(ThemeMode::Dark, None, cx);
}
