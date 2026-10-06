use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::Workspace;
use super::buffer::BufferId;
use super::files::notify_error;
use crate::document::{self, Document};
use crate::session::{Session, SessionTab};
use crate::storage;

/// How long typing has to pause before the session is written.
const SAVE_DELAY: Duration = Duration::from_secs(1);

impl Workspace {
    /// Reopens the last session and starts keeping it up to date, if
    /// `restore_session` is on. Only called when Slate starts without arguments.
    pub fn restore_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settings.restore_session {
            return;
        }
        self.session_active = true;
        let session = self.storage.load_session();

        if let Some(folder) = session.folder.filter(|folder| folder.is_dir()) {
            self.sidebar.update(cx, |sidebar, cx| sidebar.open_folder(folder, cx));
            self.sidebar_open = session.sidebar_open;
        }

        let placeholder = self.active_buffer().id;
        let mut restored = 0;
        let mut active = 0;
        for (index, tab) in session.tabs.iter().enumerate() {
            if self.restore_tab(tab, window, cx) {
                if index == session.active {
                    active = restored;
                }
                restored += 1;
            }
        }
        if restored > 0 {
            // Restored tabs were pushed after the placeholder, in order.
            self.buffers.retain(|buffer| buffer.id != placeholder);
            self.activate(active, window, cx);
        }
        cx.notify();
    }

    /// Adds one saved tab. Returns `false` when there was nothing to bring back.
    fn restore_tab(&mut self, tab: &SessionTab, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let document = match &tab.path {
            Some(path) => Document::from_path(path.clone()),
            None if tab.text.as_deref().is_some_and(|text| !text.is_empty()) => {
                Document::untitled(tab.untitled_number.max(1))
            }
            None => return false,
        };
        let color = tab
            .color
            .clone()
            .or_else(|| tab.path.as_ref().and_then(|path| self.state.tab_colors.get(path).cloned()));

        match (&tab.path, &tab.text) {
            // Unsaved edits come back as they were, still unsaved.
            (path, Some(text)) => {
                self.push_buffer(document, text.clone(), window, cx);
                let buffer = &mut self.buffers[self.active];
                buffer.document.mark_edited();
                buffer.document.set_disk_modified(tab.disk_modified);
                buffer.color = color;
                if let Some(path) = path
                    && tab.disk_changed()
                {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    window.push_notification(
                        Notification::warning(format!(
                            "{name} changed on disk since your unsaved edits. Saving will overwrite it."
                        )),
                        cx,
                    );
                }
                self.restore_cursor(self.buffers[self.active].id, tab.cursor, cx);
            }
            // A clean file reloads from disk, in the background.
            (Some(path), None) => {
                if !path.is_file() {
                    notify_error(format!("Couldn't reopen {}: it's gone", path.display()), window, cx);
                    return false;
                }
                self.push_buffer(document, String::new(), window, cx);
                self.buffers[self.active].color = color;
                let id = self.buffers[self.active].id;
                self.reload_tab(id, path.clone(), tab.cursor, window, cx);
            }
            (None, None) => return false,
        }
        true
    }

    fn reload_tab(&mut self, id: BufferId, path: PathBuf, cursor: usize, window: &mut Window, cx: &mut Context<Self>) {
        let read = cx.background_spawn({
            let path = path.clone();
            async move { document::read_text(&path).map(|text| (text, document::modified(&path))) }
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            _ = this.update_in(cx, |workspace, window, cx| match result {
                Ok((text, modified)) => {
                    let Some(buffer) = workspace.buffer_mut(id) else {
                        return;
                    };
                    buffer.document.set_disk_modified(modified);
                    let editor = buffer.editor.clone();
                    editor.update(cx, |state, cx| state.set_value(text, window, cx));
                    workspace.restore_cursor(id, cursor, cx);
                }
                Err(err) => {
                    notify_error(format!("Couldn't reopen {}: {err}", path.display()), window, cx);
                    workspace.remove_buffer(id, window, cx);
                }
            });
        })
        .detach();
    }

    fn restore_cursor(&mut self, id: BufferId, cursor: usize, cx: &mut Context<Self>) {
        if let Some(buffer) = self.buffer_mut(id) {
            buffer
                .editor
                .update(cx, |state, cx| state.set_selected_range(cursor..cursor, cx));
        }
    }

    /// The tabs worth keeping: files, plus untitled tabs that have text.
    fn collect_session(&self, cx: &App) -> Session {
        let mut tabs = Vec::new();
        let mut active = 0;
        for (index, buffer) in self.buffers.iter().enumerate() {
            let editor = buffer.editor.read(cx);
            let dirty = buffer.document.is_dirty();
            let path = buffer.document.path().map(PathBuf::from);
            let text = editor.value();
            if path.is_none() && text.is_empty() {
                continue;
            }
            if index == self.active {
                active = tabs.len();
            }
            tabs.push(SessionTab {
                untitled_number: buffer.document.untitled_number().unwrap_or(0),
                text: (dirty || path.is_none()).then(|| text.to_string()),
                disk_modified: buffer.document.disk_modified(),
                color: buffer.color.clone(),
                cursor: editor.selected_range().start,
                path,
            });
        }
        Session {
            tabs,
            active,
            folder: self.sidebar.read(cx).root().map(PathBuf::from),
            sidebar_open: self.sidebar_open,
        }
    }

    /// Writes the session now. Does nothing unless this window is restoring sessions.
    pub(super) fn write_session(&mut self, cx: &App) {
        self.pending_session_save = None;
        if !self.session_active {
            return;
        }
        let json = storage::to_json(&self.collect_session(cx));
        let path = self.storage.session_path();
        if let Err(err) = storage::write_atomic(&path, &json) {
            eprintln!("slate: couldn't write {}: {err}", path.display());
        }
    }

    /// Writes the session once things have been quiet for a second. Each call
    /// replaces the previous timer, so typing doesn't write on every key.
    pub(super) fn schedule_session_save(&mut self, cx: &mut Context<Self>) {
        if !self.session_active {
            return;
        }
        self.pending_session_save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            _ = this.update(cx, |workspace, cx| workspace.write_session(cx));
        }));
    }

    /// With session restore on, closing keeps unsaved edits for next time
    /// instead of asking about them.
    pub(super) fn close_keeps_edits(&mut self, cx: &App) -> bool {
        if self.session_active {
            self.write_session(cx);
        }
        self.session_active
    }

    /// Turning restore off forgets the saved session (its tabs are all open
    /// right now anyway). Turning it on starts saving this window's tabs.
    pub(super) fn set_restore_session(&mut self, restore: bool, cx: &mut Context<Self>) {
        if self.settings.restore_session == restore {
            return;
        }
        self.settings.restore_session = restore;
        self.save_settings();
        self.session_active = restore;
        if restore {
            self.write_session(cx);
        } else {
            self.pending_session_save = None;
            let path = self.storage.session_path();
            if let Err(err) = std::fs::remove_file(&path)
                && err.kind() != std::io::ErrorKind::NotFound
            {
                eprintln!("slate: couldn't remove {}: {err}", path.display());
            }
        }
        cx.notify();
    }
}
