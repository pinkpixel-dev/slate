use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, WindowExt as _, h_flex};
use gpui_kit::*;
use notify::{RecursiveMode, Watcher as _};

use super::Workspace;
use super::buffer::BufferId;
use super::files::notify_error;
use crate::document;

/// How long to let a burst of events (write, rename, chmod) settle.
const SETTLE: Duration = Duration::from_millis(150);

/// Watches the folders that hold open files. Folders rather than files,
/// because many tools save by writing a temp file and renaming it over the
/// original, which ends a watch on the file itself.
pub(super) struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    watched: HashSet<PathBuf>,
    _task: Task<()>,
}

impl FileWatcher {
    pub fn new(window: &mut Window, cx: &mut Context<Workspace>) -> notify::Result<Self> {
        let (tx, mut rx) = mpsc::unbounded::<Vec<PathBuf>>();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result
                && !event.kind.is_access()
            {
                let _ = tx.unbounded_send(event.paths);
            }
        })?;
        let task = cx.spawn_in(window, async move |this, cx| {
            while let Some(paths) = rx.next().await {
                let mut changed: HashSet<PathBuf> = paths.into_iter().collect();
                cx.background_executor().timer(SETTLE).await;
                while let Ok(more) = rx.try_recv() {
                    changed.extend(more);
                }
                let update = this.update_in(cx, |workspace, window, cx| {
                    workspace.on_files_changed(&changed, window, cx)
                });
                if update.is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            watcher,
            watched: HashSet::new(),
            _task: task,
        })
    }

    /// Watches exactly `dirs`, adding and dropping watches as needed.
    fn sync(&mut self, dirs: HashSet<PathBuf>) {
        for gone in self.watched.difference(&dirs).cloned().collect::<Vec<_>>() {
            let _ = self.watcher.unwatch(&gone);
            self.watched.remove(&gone);
        }
        for dir in dirs {
            if self.watched.contains(&dir) {
                continue;
            }
            match self.watcher.watch(&dir, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.watched.insert(dir);
                }
                Err(err) => eprintln!("slate: not watching {}: {err}", dir.display()),
            }
        }
    }
}

impl Workspace {
    /// Keeps a watch on the folder of every open file.
    pub(super) fn sync_file_watches(&mut self) {
        let dirs = self
            .buffers
            .iter()
            .filter_map(|buffer| buffer.document.path()?.parent().map(Path::to_path_buf))
            .collect();
        if let Some(watcher) = self.file_watcher.as_mut() {
            watcher.sync(dirs);
        }
    }

    /// Checks every open file among `paths` against what's on disk.
    pub(super) fn on_files_changed(&mut self, paths: &HashSet<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<(BufferId, PathBuf)> = self
            .buffers
            .iter()
            .filter(|buffer| buffer.saves_in_flight == 0)
            .filter_map(|buffer| {
                let path = buffer.document.path()?;
                paths.contains(path).then(|| (buffer.id, path.to_path_buf()))
            })
            .collect();
        for (id, path) in ids {
            self.check_disk(id, path, window, cx);
        }
    }

    fn check_disk(&mut self, id: BufferId, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let read = cx.background_spawn({
            let path = path.clone();
            async move { document::read_text(&path).map(|text| (text, document::modified(&path))) }
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            _ = this.update_in(cx, |workspace, window, cx| {
                workspace.apply_disk_state(id, &path, result, window, cx)
            });
        })
        .detach();
    }

    fn apply_disk_state(
        &mut self,
        id: BufferId,
        path: &Path,
        result: io::Result<(String, Option<std::time::SystemTime>)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(buffer) = self.buffer_mut(id) else {
            return;
        };
        // A save started while the read was running; its own result wins.
        if buffer.saves_in_flight > 0 || buffer.document.path() != Some(path) {
            return;
        }
        let name = buffer.document.display_name();
        match result {
            Ok((text, modified)) => {
                if modified == buffer.document.disk_modified() {
                    return;
                }
                let editor = buffer.editor.clone();
                if editor.read(cx).value() == text {
                    // Touched, or rewritten with the same text (often our own save).
                    buffer.document.set_disk_modified(modified);
                } else if buffer.document.is_dirty() {
                    buffer.disk_conflict = true;
                } else {
                    buffer.document.set_disk_modified(modified);
                    reload_text(&editor, text, window, cx);
                    self.text_changed(id, cx);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Deleted or moved away. Keep the tab, unsaved, so Save puts it back.
                if buffer.document.disk_modified().is_none() {
                    return;
                }
                buffer.document.set_disk_modified(None);
                buffer.document.mark_edited();
                buffer.disk_conflict = false;
                self.sync_window_title(window);
                window.push_notification(
                    Notification::warning(format!("{name} was deleted or moved on disk. Save to keep it.")),
                    cx,
                );
            }
            // Now binary or not UTF-8: leave the tab alone, the error says why.
            Err(err) => notify_error(format!("{name} changed on disk and can't be reloaded: {err}"), window, cx),
        }
        cx.notify();
    }

    /// Replaces the active tab's unsaved edits with the file on disk.
    fn reload_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let buffer = self.active_buffer();
        let (id, Some(path)) = (buffer.id, buffer.document.path().map(Path::to_path_buf)) else {
            return;
        };
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
                    let revision = buffer.document.revision();
                    buffer.document.mark_saved(path, revision);
                    buffer.document.set_disk_modified(modified);
                    buffer.disk_conflict = false;
                    let editor = buffer.editor.clone();
                    reload_text(&editor, text, window, cx);
                    workspace.text_changed(id, cx);
                    workspace.sync_window_title(window);
                    workspace.schedule_session_save(cx);
                    cx.notify();
                }
                Err(err) => notify_error(format!("Couldn't reload {}: {err}", path.display()), window, cx),
            });
        })
        .detach();
    }

    /// Keeps the unsaved edits. Saving will overwrite the file on disk.
    fn keep_mine(&mut self, cx: &mut Context<Self>) {
        let buffer = &mut self.buffers[self.active];
        let modified = buffer.document.path().and_then(document::modified);
        buffer.document.set_disk_modified(modified);
        buffer.disk_conflict = false;
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// "Changed on disk" bar for the active tab, shown above the editor.
    pub(super) fn render_disk_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let buffer = self.active_buffer();
        if !buffer.disk_conflict {
            return None;
        }
        let theme = cx.theme();
        let bar = h_flex()
            .id("disk-bar")
            .w_full()
            .gap_2()
            .px_3()
            .py_1p5()
            .items_center()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.border)
            .child(Icon::new(IconName::TriangleAlert).small().text_color(theme.warning))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .child(format!("{} changed on disk.", buffer.document.display_name())),
            )
            .child(
                Button::new("disk-keep")
                    .ghost()
                    .small()
                    .label("Keep Mine")
                    .tooltip("Keep your edits. Saving overwrites the file on disk")
                    .on_click(cx.listener(|this, _, _, cx| this.keep_mine(cx))),
            )
            .child(
                Button::new("disk-reload")
                    .outline()
                    .small()
                    .label("Reload")
                    .tooltip("Throw away your edits and load the file from disk")
                    .on_click(cx.listener(|this, _, window, cx| this.reload_from_disk(window, cx))),
            );
        Some(bar.into_any_element())
    }
}

/// Swaps in new text without counting as an edit, keeping the cursor about where it was.
fn reload_text(editor: &Entity<gpui_kit::component::input::EditorState>, text: String, window: &mut Window, cx: &mut App) {
    editor.update(cx, |state, cx| {
        let cursor = state.cursor();
        state.set_value(text, window, cx);
        state.set_selected_range(cursor..cursor, cx);
    });
}
