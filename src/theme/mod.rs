mod fonts;
mod watcher;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeConfig, ThemeSet};
use gpui_kit::*;

use crate::storage::Settings;
pub use fonts::FontLists;
pub use watcher::ThemeWatcher;

pub const DEFAULT_THEME_NAME: &str = "Slate Dark";

/// Slate's own theme plus Kit's theme set (from gpui-kit v0.7.1, Apache-2.0),
/// embedded so every theme works with no files on disk.
const BUNDLED: &[&str] = &[
    include_str!("../../themes/slate.json"),
    include_str!("../../themes/kit/adventure.json"),
    include_str!("../../themes/kit/alduin.json"),
    include_str!("../../themes/kit/asciinema.json"),
    include_str!("../../themes/kit/aurora.json"),
    include_str!("../../themes/kit/ayu.json"),
    include_str!("../../themes/kit/catppuccin.json"),
    include_str!("../../themes/kit/everforest.json"),
    include_str!("../../themes/kit/fahrenheit.json"),
    include_str!("../../themes/kit/flexoki.json"),
    include_str!("../../themes/kit/gruvbox.json"),
    include_str!("../../themes/kit/harper.json"),
    include_str!("../../themes/kit/hybrid.json"),
    include_str!("../../themes/kit/jellybeans.json"),
    include_str!("../../themes/kit/kibble.json"),
    include_str!("../../themes/kit/macos-classic.json"),
    include_str!("../../themes/kit/mellifluous.json"),
    include_str!("../../themes/kit/molokai.json"),
    include_str!("../../themes/kit/solarized.json"),
    include_str!("../../themes/kit/spaceduck.json"),
    include_str!("../../themes/kit/tokyonight.json"),
    include_str!("../../themes/kit/twilight.json"),
];

/// Theme values a theme file may or may not set. Every switch starts from
/// these, so a theme without a radius doesn't inherit the last theme's.
#[derive(Clone)]
struct Baseline {
    font_family: SharedString,
    font_size: Pixels,
    mono_font_family: SharedString,
    mono_font_size: Pixels,
    radius: Pixels,
    radius_lg: Pixels,
    shadow: bool,
}

/// Every theme Slate knows about: the bundled set and the user's themes folder.
///
/// Slate keeps this instead of using Kit's `ThemeRegistry::watch_dir`, because
/// a registry reload drops any theme that isn't a file in the watched folder.
pub struct ThemeCatalog {
    bundled: Vec<Rc<ThemeConfig>>,
    custom: Vec<Rc<ThemeConfig>>,
    custom_dir: PathBuf,
    baseline: Baseline,
}

impl Global for ThemeCatalog {}

impl ThemeCatalog {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn custom_dir(&self) -> &Path {
        &self.custom_dir
    }

    /// Looks a theme up by name. A custom theme wins over a bundled one with
    /// the same name, so copying a bundled theme into the folder overrides it.
    pub fn find(&self, name: &str) -> Option<Rc<ThemeConfig>> {
        self.custom
            .iter()
            .chain(&self.bundled)
            .find(|theme| theme.name == name)
            .cloned()
    }

    /// Every theme name once, sorted alphabetically.
    pub fn names(&self) -> Vec<SharedString> {
        let mut names: Vec<SharedString> = self
            .custom
            .iter()
            .chain(&self.bundled)
            .map(|theme| theme.name.clone())
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names.dedup();
        names
    }
}

/// Builds the theme catalog. The custom folder is created so it can be watched
/// and opened from the settings panel.
pub fn init(custom_dir: PathBuf, cx: &mut App) {
    let mut bundled = Vec::new();
    for source in BUNDLED {
        match parse_set(source) {
            Ok(themes) => bundled.extend(themes),
            Err(err) => eprintln!("slate: a bundled theme failed to parse: {err}"),
        }
    }
    if let Err(err) = std::fs::create_dir_all(&custom_dir) {
        eprintln!("slate: couldn't create {}: {err}", custom_dir.display());
    }
    let (custom, errors) = load_dir(&custom_dir);
    for error in errors {
        eprintln!("slate: {error}");
    }

    let theme = Theme::global(cx);
    let baseline = Baseline {
        font_family: theme.font_family.clone(),
        font_size: theme.font_size,
        mono_font_family: theme.mono_font_family.clone(),
        mono_font_size: theme.mono_font_size,
        radius: theme.radius,
        radius_lg: theme.radius_lg,
        shadow: theme.shadow,
    };
    cx.set_global(ThemeCatalog {
        bundled,
        custom,
        custom_dir,
        baseline,
    });
}

/// Re-reads the custom themes folder. Returns a message for each file that
/// couldn't be loaded.
pub fn reload_custom(cx: &mut App) -> Vec<String> {
    let dir = ThemeCatalog::global(cx).custom_dir.clone();
    let (custom, errors) = load_dir(&dir);
    cx.global_mut::<ThemeCatalog>().custom = custom;
    errors
}

/// Applies the theme and font settings. An unknown theme name falls back to Slate Dark.
/// The editor font size zoom stays within.
pub const MIN_EDITOR_FONT_SIZE: f32 = 6.;
pub const MAX_EDITOR_FONT_SIZE: f32 = 48.;

pub fn apply(settings: &Settings, cx: &mut App) {
    let catalog = ThemeCatalog::global(cx);
    let Some(config) = settings
        .theme
        .as_deref()
        .and_then(|name| catalog.find(name))
        .or_else(|| catalog.find(DEFAULT_THEME_NAME))
    else {
        eprintln!("slate: bundled theme \"{DEFAULT_THEME_NAME}\" is missing");
        return;
    };
    let base = catalog.baseline.clone();

    Theme::update(cx, |theme| {
        theme.font_family = base.font_family;
        theme.font_size = base.font_size;
        theme.mono_font_family = base.mono_font_family;
        theme.mono_font_size = base.mono_font_size;
        theme.radius = base.radius;
        theme.radius_lg = base.radius_lg;
        theme.shadow = base.shadow;

        theme.apply_config(&config);

        if let Some(font) = &settings.ui_font {
            theme.font_family = font.clone().into();
        }
        if let Some(size) = settings.ui_font_size {
            theme.font_size = px(size);
        }
        if let Some(font) = &settings.editor_font {
            theme.mono_font_family = font.clone().into();
        }
        if let Some(size) = settings.editor_font_size {
            theme.mono_font_size = px(size);
        }
        if settings.editor_zoom != 0. {
            let zoomed = f32::from(theme.mono_font_size) + settings.editor_zoom;
            theme.mono_font_size = px(zoomed.clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE));
        }
    });
}

fn parse_set(json: &str) -> serde_json::Result<Vec<Rc<ThemeConfig>>> {
    let set: ThemeSet = serde_json::from_str(json)?;
    Ok(set.themes.into_iter().map(Rc::new).collect())
}

/// Loads every `*.json` theme file in `dir`, in file name order.
fn load_dir(dir: &Path) -> (Vec<Rc<ThemeConfig>>, Vec<String>) {
    let mut themes = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (themes, errors);
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    for path in paths {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_set(&text) {
                Ok(set) => themes.extend(set),
                Err(err) => errors.push(format!("Theme {name} has an error: {err}")),
            },
            Err(err) => errors.push(format!("Couldn't read theme {name}: {err}")),
        }
    }
    (themes, errors)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{BUNDLED, DEFAULT_THEME_NAME, load_dir, parse_set};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-themes-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_bundled_theme_parses() {
        let mut names = Vec::new();
        for source in BUNDLED {
            names.extend(parse_set(source).unwrap().into_iter().map(|theme| theme.name.clone()));
        }
        assert!(names.iter().any(|name| name == DEFAULT_THEME_NAME));
        assert!(names.iter().any(|name| name == "Catppuccin Mocha"));
        assert!(names.len() > 30);
    }

    #[test]
    fn custom_folder_loads_good_files_and_reports_bad_ones() {
        let dir = temp_dir("custom");
        std::fs::write(
            dir.join("mine.json"),
            r#"{"name": "Mine", "themes": [{"name": "My Dark", "mode": "dark"}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let (themes, errors) = load_dir(&dir);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name, "My Dark");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("broken.json"));
    }
}
