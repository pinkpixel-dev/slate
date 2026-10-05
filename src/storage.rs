use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::tab_color::TabColor;

const RECENT_LIMIT: usize = 10;

/// User preferences, saved to `~/.config/slate/settings.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub tab_color_mode: TabColorMode,
    /// Show dotfiles and folders like `node_modules` in the sidebar.
    pub show_hidden_files: bool,
    /// Theme name; `None` means Slate Dark.
    pub theme: Option<String>,
    /// Font overrides; `None` keeps the theme's (or Kit's) default.
    pub ui_font: Option<String>,
    pub ui_font_size: Option<f32>,
    pub editor_font: Option<String>,
    pub editor_font_size: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabColorMode {
    /// Only colors picked by hand.
    #[default]
    Manual,
    /// Tabs are colored by language; a hand-picked color still wins.
    Language,
}

/// Things Slate remembers between runs, saved to `~/.local/state/slate/state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub recent_files: Vec<PathBuf>,
    pub tab_colors: BTreeMap<PathBuf, TabColor>,
}

impl AppState {
    pub fn add_recent(&mut self, path: &Path) {
        self.recent_files.retain(|existing| existing != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(RECENT_LIMIT);
    }

    pub fn set_tab_color(&mut self, path: &Path, color: Option<TabColor>) {
        match color {
            Some(color) => self.tab_colors.insert(path.to_path_buf(), color),
            None => self.tab_colors.remove(path),
        };
    }
}

/// Where settings and state live on disk.
#[derive(Debug, Clone)]
pub struct Storage {
    config_dir: PathBuf,
    state_dir: PathBuf,
}

impl Storage {
    /// The XDG locations, e.g. `~/.config/slate` and `~/.local/state/slate`.
    pub fn from_env() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let xdg = |var: &str, fallback: &str| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.join(fallback))
                .join("slate")
        };
        Self {
            config_dir: xdg("XDG_CONFIG_HOME", ".config"),
            state_dir: xdg("XDG_STATE_HOME", ".local/state"),
        }
    }

    /// Keeps everything under one folder; used by tests.
    #[cfg(test)]
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            config_dir: dir.join("config"),
            state_dir: dir.join("state"),
        }
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    /// Custom theme files, hot-reloaded while Slate runs.
    pub fn themes_dir(&self) -> PathBuf {
        self.config_dir.join("themes")
    }

    pub fn state_path(&self) -> PathBuf {
        self.state_dir.join("state.json")
    }

    pub fn load_settings(&self) -> Settings {
        load_json(&self.settings_path())
    }

    pub fn load_state(&self) -> AppState {
        load_json(&self.state_path())
    }
}

/// Reads a JSON file, falling back to defaults when it's missing or unreadable.
fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            eprintln!("slate: ignoring {}: {err}", path.display());
            T::default()
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => T::default(),
        Err(err) => {
            eprintln!("slate: couldn't read {}: {err}", path.display());
            T::default()
        }
    }
}

/// Serializes `value` now, so the write itself can run off the UI thread.
pub fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("settings and state always serialize")
}

/// Writes through a temporary file and a rename, so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-storage-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn missing_files_load_as_defaults() {
        let storage = Storage::in_dir(&temp_dir("missing"));
        assert_eq!(storage.load_settings(), Settings::default());
        assert_eq!(storage.load_state(), AppState::default());
    }

    #[test]
    fn state_round_trips_through_disk() {
        let storage = Storage::in_dir(&temp_dir("round-trip"));
        let mut state = AppState::default();
        state.add_recent(Path::new("/tmp/a.md"));
        state.set_tab_color(Path::new("/tmp/a.md"), Some(TabColor::Green));

        write_atomic(&storage.state_path(), &to_json(&state)).unwrap();
        assert_eq!(storage.load_state(), state);
    }

    #[test]
    fn settings_keep_defaults_for_unknown_or_missing_keys() {
        let dir = temp_dir("partial");
        let storage = Storage::in_dir(&dir);
        write_atomic(&storage.settings_path(), r#"{"something_new": 1}"#).unwrap();
        assert_eq!(storage.load_settings().tab_color_mode, TabColorMode::Manual);

        write_atomic(&storage.settings_path(), r#"{"tab_color_mode": "language"}"#).unwrap();
        assert_eq!(storage.load_settings().tab_color_mode, TabColorMode::Language);
    }

    #[test]
    fn appearance_settings_round_trip() {
        let storage = Storage::in_dir(&temp_dir("appearance"));
        let settings = Settings {
            theme: Some("Gruvbox Dark".into()),
            editor_font: Some("JetBrains Mono".into()),
            editor_font_size: Some(15.0),
            ..Settings::default()
        };
        write_atomic(&storage.settings_path(), &to_json(&settings)).unwrap();
        assert_eq!(storage.load_settings(), settings);
    }

    #[test]
    fn recent_files_are_deduplicated_newest_first_and_capped() {
        let mut state = AppState::default();
        for i in 0..12 {
            state.add_recent(Path::new(&format!("/tmp/{i}.txt")));
        }
        state.add_recent(Path::new("/tmp/5.txt"));
        assert_eq!(state.recent_files.len(), RECENT_LIMIT);
        assert_eq!(state.recent_files[0], Path::new("/tmp/5.txt"));
        assert_eq!(state.recent_files.iter().filter(|p| p.ends_with("5.txt")).count(), 1);
    }
}
