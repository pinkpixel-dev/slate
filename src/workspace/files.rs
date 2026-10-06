use std::io;
use std::path::PathBuf;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::buffer::BufferId;
use super::{Open, PendingAction, Save, SaveAs, Workspace};
use crate::document::{self, Document, Loaded};

impl Workspace {
    pub(super) fn open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Open".into()),
        });

        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some(paths))) => {
                _ = this.update_in(cx, |workspace, window, cx| {
                    for path in paths {
                        workspace.open_path(path, false, window, cx);
                    }
                });
            }
            Ok(Err(err)) => {
                _ = this.update_in(cx, |_, window, cx| {
                    notify_error(format!("Couldn't open the file picker: {err}"), window, cx);
                });
            }
            _ => {}
        })
        .detach();
    }

    /// Opens `path` in a tab, or switches to it if it's already open. A folder
    /// opens in the sidebar. With `missing_ok`, a path that doesn't exist yet
    /// opens as an empty file that Save will create.
    pub fn open_path(&mut self, path: PathBuf, missing_ok: bool, window: &mut Window, cx: &mut Context<Self>) {
        let path = std::path::absolute(&path).unwrap_or(path);
        if path.is_dir() {
            self.show_folder(path, cx);
            return;
        }
        if self.focus_open_path(&path, window, cx) {
            return;
        }

        let read = cx.background_spawn({
            let path = path.clone();
            async move { document::load(&path) }
        });

        cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            _ = this.update_in(cx, |workspace, window, cx| match result {
                Ok(loaded) => workspace.show_file(path, loaded, window, cx),
                Err(err) if missing_ok && err.kind() == io::ErrorKind::NotFound => {
                    let empty = Loaded {
                        text: String::new(),
                        line_ending: Default::default(),
                        modified: None,
                    };
                    workspace.show_file(path, empty, window, cx)
                }
                Err(err) => notify_error(format!("Couldn't open {}: {err}", path.display()), window, cx),
            });
        })
        .detach();
    }

    fn focus_open_path(&mut self, path: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let open = self
            .buffers
            .iter()
            .position(|buffer| buffer.document.path() == Some(path));
        if let Some(index) = open {
            self.activate(index, window, cx);
        }
        open.is_some()
    }

    fn show_file(&mut self, path: PathBuf, loaded: Loaded, window: &mut Window, cx: &mut Context<Self>) {
        let Loaded {
            text,
            line_ending,
            modified,
        } = loaded;
        // Two opens of the same file can race; the second one just switches to it.
        if self.focus_open_path(&path, window, cx) {
            return;
        }
        let color = self.state.tab_colors.get(&path).cloned();
        let mut document = Document::from_path(path);
        document.set_disk_modified(modified);
        document.set_line_ending(line_ending);

        if self.active_buffer().is_pristine(cx) {
            let language = document.language().id;
            let buffer = &mut self.buffers[self.active];
            buffer.document = document;
            buffer.color = color;
            buffer.detect_indent(&text, cx);
            buffer.editor.update(cx, |state, cx| {
                state.set_highlighter(language, cx);
                state.set_value(text, window, cx);
            });
            let id = self.active_buffer().id;
            self.text_changed(id, cx);
            self.activate(self.active, window, cx);
        } else {
            self.push_buffer(document, text, window, cx);
            self.buffers[self.active].color = color;
        }

        let id = self.active_buffer().id;
        self.remember_file(id);
    }

    pub(super) fn save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        self.save_then(id, None, window, cx);
    }

    pub(super) fn save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        self.prompt_save_as(id, None, window, cx);
    }

    /// Saves a buffer to its current path, or asks for one first. Runs `after`
    /// once the write succeeds.
    pub(super) fn save_then(
        &mut self,
        id: BufferId,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        match self.buffers[index].document.path() {
            Some(path) => self.write(id, path.to_path_buf(), after, window, cx),
            None => self.prompt_save_as(id, after, window, cx),
        }
    }

    fn prompt_save_as(
        &mut self,
        id: BufferId,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        let document = &self.buffers[index].document;
        let picked = cx.prompt_for_new_path(&document.directory(), Some(&document.display_name()));

        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some(path))) => {
                _ = this.update_in(cx, |workspace, window, cx| workspace.write(id, path, after, window, cx));
            }
            Ok(Err(err)) => {
                _ = this.update_in(cx, |_, window, cx| {
                    notify_error(format!("Couldn't open the save dialog: {err}"), window, cx);
                });
            }
            _ => {}
        })
        .detach();
    }

    fn write(
        &mut self,
        id: BufferId,
        path: PathBuf,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        let buffer = &mut self.buffers[index];
        buffer.saves_in_flight += 1;
        let revision = buffer.document.revision();
        let line_ending = buffer.document.line_ending();
        let text = buffer.editor.read(cx).value();
        let write = cx.background_spawn({
            let path = path.clone();
            async move { document::write_text(&path, &text, line_ending).map(|()| document::modified(&path)) }
        });

        cx.spawn_in(window, async move |this, cx| {
            let result = write.await;
            _ = this.update_in(cx, |workspace, window, cx| {
                let Some(buffer) = workspace.buffer_mut(id) else {
                    return;
                };
                buffer.saves_in_flight -= 1;
                match result {
                    Ok(modified) => {
                        buffer.document.set_disk_modified(modified);
                        // Saving answers any "changed on disk" question.
                        buffer.disk_conflict = false;
                        workspace.finish_save(id, path, revision, after, window, cx)
                    }
                    Err(err) => notify_error(format!("Couldn't save {}: {err}", path.display()), window, cx),
                }
            });
        })
        .detach();
    }

    fn finish_save(
        &mut self,
        id: BufferId,
        path: PathBuf,
        revision: u64,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The tab may have been closed while the write was in flight.
        let Some(buffer) = self.buffer_mut(id) else {
            return;
        };
        let language_before = buffer.document.language();
        buffer.document.mark_saved(path, revision);
        let language = buffer.document.language();
        if language != language_before {
            buffer
                .editor
                .update(cx, |state, cx| state.set_highlighter(language.id, cx));
        }
        self.remember_file(id);
        self.sync_window_title(window);
        self.sync_file_watches();
        self.schedule_session_save(cx);
        cx.notify();
        if let Some(action) = after {
            self.run_pending(action, window, cx);
        }
    }
}

pub(super) fn notify_error(message: String, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::error(message), cx);
}
