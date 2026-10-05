use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::Workspace;
use super::buffer::{Buffer, BufferId};
use crate::storage::{self, TabColorMode};
use crate::tab_color::TabColor;

impl Workspace {
    /// The color a tab's top line is drawn in, if any.
    pub(super) fn tab_color(&self, buffer: &Buffer) -> Option<TabColor> {
        buffer.color.clone().or_else(|| match self.settings.tab_color_mode {
            TabColorMode::Manual => None,
            TabColorMode::Language => TabColor::for_language(buffer.document.language().id),
        })
    }

    pub(super) fn tab_hsla(&self, buffer: &Buffer, cx: &App) -> Option<Hsla> {
        self.tab_color(buffer).map(|color| color.resolve(cx.theme()))
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
        write_now(&self.storage.settings_path(), &storage::to_json(&self.settings));
        cx.notify();
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
