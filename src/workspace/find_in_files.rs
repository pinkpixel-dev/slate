use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::input::Position;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, IndexPath, WindowExt as _, h_flex};
use gpui_kit::*;

use super::buffer::BufferId;
use super::{KEY_CONTEXT, Workspace};
use crate::file_index;
use crate::file_search::{self, Match};

actions!(slate, [FindInFiles]);

const DIALOG_WIDTH: f32 = 720.;
/// More than this is a sign the query needs another word, not more scrolling.
const MAX_MATCHES: usize = 500;
/// Shorter queries match nearly every line.
const MIN_QUERY: usize = 2;
/// Wait for a pause in typing before reading every file again.
const DEBOUNCE: Duration = Duration::from_millis(200);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-f", FindInFiles, Some(KEY_CONTEXT))]);
}

/// The files Find in Files reads, and what the current query found.
pub(super) struct FindInFilesState {
    command: Entity<CommandState>,
    root: PathBuf,
    files: Arc<Vec<String>>,
    pub matches: Vec<Match>,
    query: String,
    /// Bumped for every new search, so a search that's been replaced stops reading files.
    generation: Arc<AtomicU64>,
    _scan: Option<Task<()>>,
    _search: Option<Task<()>>,
}

impl FindInFilesState {
    pub fn new(window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        Self {
            command: cx.new(|cx| CommandState::new(window, cx)),
            root: PathBuf::new(),
            files: Arc::default(),
            matches: Vec::new(),
            query: String::new(),
            generation: Arc::default(),
            _scan: None,
            _search: None,
        }
    }
}

impl Workspace {
    pub(super) fn find_in_files(&mut self, _: &FindInFiles, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(root) = self.quick_open_root(cx) else {
            window.push_notification(Notification::info("Open a folder or a file to search its files."), cx);
            return;
        };

        let state = &mut self.find_in_files;
        state.generation.fetch_add(1, Ordering::Relaxed);
        state.root = root.clone();
        state.files = Arc::default();
        state.matches.clear();
        state.query.clear();
        state._search = None;
        let command = state.command.clone();
        command.update(cx, |command, cx| {
            command.set_query("", window, cx);
            command.set_loading(true, window, cx);
        });

        let show_hidden = self.settings.show_hidden_files;
        let scan = cx.background_spawn({
            let root = root.clone();
            async move { file_index::list_files(&root, show_hidden) }
        });
        self.find_in_files._scan = Some(cx.spawn_in(window, async move |this, cx| {
            let files = scan.await;
            _ = this.update_in(cx, |workspace, window, cx| {
                let state = &mut workspace.find_in_files;
                if state.root != root {
                    return;
                }
                state.files = Arc::new(files);
                state.command.update(cx, |command, cx| command.set_loading(false, window, cx));
                // Typed while the folder was still being listed.
                let query = state.query.clone();
                workspace.search_files(query, window, cx);
            });
        }));

        let this = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let command = command.clone();
            let this = this.clone();
            dialog
                .close_button(false)
                .p_0()
                .w(px(DIALOG_WIDTH))
                .margin_top(px(72.))
                .content(move |content, _, cx| content.child(render_find_in_files(&command, this.clone(), cx)))
        });
        self.find_in_files.command.update(cx, |command, cx| command.focus(window, cx));
    }

    /// Starts a search for `query` after a short pause, replacing any search still running.
    fn search_files(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.find_in_files;
        let generation = state.generation.fetch_add(1, Ordering::Relaxed) + 1;
        state.query = query.clone();
        state.matches.clear();
        state._search = None;
        cx.notify();
        if query.chars().count() < MIN_QUERY || state.files.is_empty() {
            return;
        }

        let (root, files, current) = (state.root.clone(), state.files.clone(), state.generation.clone());
        state.command.update(cx, |command, cx| command.set_loading(true, window, cx));
        state._search = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let found = cx
                .background_spawn(async move {
                    file_search::search(&root, &files, &query, MAX_MATCHES, || {
                        current.load(Ordering::Relaxed) != generation
                    })
                })
                .await;
            _ = this.update_in(cx, |workspace, window, cx| {
                let state = &mut workspace.find_in_files;
                if state.generation.load(Ordering::Relaxed) != generation {
                    return;
                }
                state.matches = found;
                state.command.update(cx, |command, cx| command.set_loading(false, window, cx));
                cx.notify();
            });
        }));
    }

    fn confirm_find_in_files(&mut self, index: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.find_in_files.matches.get(index.row) else {
            return;
        };
        let path = self.find_in_files.root.join(&found.file);
        let position = Position::new(found.line, found.column);
        window.close_dialog(cx);
        self.open_at(path, position, window, cx);
    }

    /// Opens `path` with the cursor at `position`, waiting for the file to load if it isn't open yet.
    fn open_at(&mut self, path: PathBuf, position: Position, window: &mut Window, cx: &mut Context<Self>) {
        let path = std::path::absolute(&path).unwrap_or(path);
        if self.focus_open_path(&path, window, cx) {
            let id = self.active_buffer().id;
            self.move_cursor(id, position, window, cx);
            return;
        }
        self.pending_jump = Some((path.clone(), position));
        self.open_path(path, false, window, cx);
    }

    /// Called once a file has opened in tab `id`: moves to the match that opened it.
    pub(super) fn finish_pending_jump(&mut self, id: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let Some((path, position)) = self.pending_jump.take() else {
            return;
        };
        let opened = self.buffers.iter().find(|buffer| buffer.id == id).and_then(|buffer| buffer.document.path());
        if opened == Some(path.as_path()) {
            self.move_cursor(id, position, window, cx);
        }
    }

    fn move_cursor(&mut self, id: BufferId, position: Position, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(buffer) = self.buffer_mut(id) {
            buffer
                .editor
                .update(cx, |state, cx| state.set_cursor_position(position, window, cx));
        }
    }
}

fn render_find_in_files(state: &Entity<CommandState>, this: WeakEntity<Workspace>, cx: &App) -> Command {
    let (matches, query) = this
        .upgrade()
        .map(|workspace| {
            let state = &workspace.read(cx).find_in_files;
            (state.matches.clone(), state.query.clone())
        })
        .unwrap_or_default();
    let items = matches.into_iter().map(|found| {
        let location = format!("{}:{}", found.file, found.line + 1);
        CommandItem::new().label(location.clone()).child(move |_, cx| {
            h_flex()
                .w_full()
                .gap_3()
                .items_center()
                .child(div().flex_1().min_w_0().truncate().child(found.preview.clone()))
                .child(
                    div()
                        .flex_none()
                        .max_w(px(DIALOG_WIDTH * 0.4))
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(location.clone()),
                )
                .into_any_element()
        })
    });

    let (on_query, confirm) = (this.clone(), this);
    Command::new(state)
        .bordered(false)
        .filterable(false)
        .placeholder("Search text in files")
        .max_h(px(440.))
        .items(items)
        .empty(move |state, _, cx| {
            let message = if state.is_loading() {
                "Searching"
            } else if query.chars().count() < MIN_QUERY {
                "Type at least 2 characters"
            } else {
                "No matches"
            };
            div()
                .p_3()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message)
        })
        .on_query(move |text, window, cx| {
            let text = text.to_string();
            _ = on_query.update(cx, |workspace, cx| workspace.search_files(text, window, cx));
        })
        .on_confirm(move |index, window, cx| {
            _ = confirm.update(cx, |workspace, cx| workspace.confirm_find_in_files(index, window, cx));
        })
}
