use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::Workspace;
use super::buffer::{Buffer, BufferId};
use super::files::notify_error;
use crate::storage::{self, Settings, TabColorMode};
use crate::tab_color::{self, TabColor};

impl Workspace {
    /// The color of the tab at `index`'s top line, if any. A color picked from
    /// the tab menu always wins over the automatic mode.
    pub(super) fn tab_hsla(&self, index: usize, buffer: &Buffer, cx: &App) -> Option<Hsla> {
        let theme = cx.theme();
        if let Some(color) = &buffer.color {
            return Some(color.resolve(theme));
        }
        match self.settings.tab_color_mode {
            TabColorMode::Theme => {
                let cycle = tab_color::theme_cycle(theme);
                cycle.get(index % cycle.len()).copied()
            }
            TabColorMode::Language => {
                TabColor::for_language(buffer.document.language().id).map(|color| color.resolve(theme))
            }
            TabColorMode::Off => None,
        }
    }

    pub(super) fn set_buffer_color(&mut self, id: BufferId, color: Option<TabColor>, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffer_mut(id) else {
            return;
        };
        buffer.color = color.clone();
        if let Some(path) = buffer.document.path().map(Path::to_path_buf) {
            self.state.set_tab_color(&path, color);
            self.save_state();
        }
        cx.notify();
    }

    pub(super) fn set_tab_color_mode(&mut self, mode: TabColorMode, cx: &mut Context<Self>) {
        self.settings.tab_color_mode = mode;
        self.save_settings();
        cx.notify();
    }

    /// Changes theme or font settings, saves them, and restyles every window.
    pub(super) fn update_appearance(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings);
        self.save_settings();
        crate::theme::apply(&self.settings, cx);
        cx.notify();
    }

    /// Goes through the sidebar's own toggle, which saves the setting.
    pub(super) fn set_show_hidden(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.settings.show_hidden_files != show {
            self.sidebar.update(cx, |sidebar, cx| sidebar.toggle_hidden(cx));
            cx.notify();
        }
    }

    /// Something in the custom themes folder changed: reload it and reapply,
    /// so edits to the active theme show up right away.
    pub(super) fn on_themes_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for error in crate::theme::reload_custom(cx) {
            notify_error(error, window, cx);
        }
        crate::theme::apply(&self.settings, cx);
        cx.notify();
    }

    pub(super) fn save_settings(&self) {
        write_now(&self.storage.settings_path(), &storage::to_json(&self.settings));
    }

    /// Records a file as recently used and remembers its tab color under that path.
    pub(super) fn remember_file(&mut self, id: BufferId) {
        let Some(buffer) = self.buffers.iter().find(|buffer| buffer.id == id) else {
            return;
        };
        let Some(path) = buffer.document.path().map(Path::to_path_buf) else {
            return;
        };
        let color = buffer.color.clone();
        self.state.add_recent(&path);
        if color.is_some() {
            self.state.set_tab_color(&path, color);
        }
        self.save_state();
    }

    pub(super) fn clear_recent(&mut self, cx: &mut Context<Self>) {
        self.state.recent_files.clear();
        self.save_state();
        cx.notify();
    }

    fn save_state(&self) {
        write_now(&self.storage.state_path(), &storage::to_json(&self.state));
    }
}

/// Writes right away, in order. These files are a few KB, and writing them
/// from background tasks could let an older write land after a newer one.
fn write_now(path: &Path, json: &str) {
    if let Err(err) = storage::write_atomic(path, json) {
        eprintln!("slate: couldn't write {}: {err}", path.display());
    }
}
