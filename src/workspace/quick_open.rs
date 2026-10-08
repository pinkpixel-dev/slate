use std::path::{Path, PathBuf};

use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, IndexPath, WindowExt as _, h_flex};
use gpui_kit::*;

use super::{KEY_CONTEXT, Workspace};
use crate::file_index;

actions!(slate, [QuickOpen]);

const DIALOG_WIDTH: f32 = 560.;
/// Rows shown at once. Typing narrows the list, so more would just be noise.
const MAX_RESULTS: usize = 100;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-p", QuickOpen, Some(KEY_CONTEXT))]);
}

/// The file list Quick Open searches, and what currently matches.
pub(super) struct QuickOpenState {
    pub command: Entity<CommandState>,
    root: PathBuf,
    files: Vec<String>,
    pub matches: Vec<String>,
    query: String,
    /// Listing recent files (absolute paths) because there's no folder to search.
    recent: bool,
    _scan: Option<Task<()>>,
}

impl QuickOpenState {
    pub fn new(window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        Self {
            command: cx.new(|cx| CommandState::new(window, cx)),
            root: PathBuf::new(),
            files: Vec::new(),
            matches: Vec::new(),
            query: String::new(),
            recent: false,
            _scan: None,
        }
    }

    fn refresh_matches(&mut self) {
        self.matches = file_index::rank(&self.query, &self.files, MAX_RESULTS)
            .into_iter()
            .map(String::from)
            .collect();
    }
}

impl Workspace {
    /// Searches the sidebar folder, or the active file's folder when the
    /// sidebar has none (the same fallback as Ctrl+B).
    fn quick_open_root(&self, cx: &App) -> Option<PathBuf> {
        self.sidebar
            .read(cx)
            .root()
            .map(Path::to_path_buf)
            .or_else(|| self.active_buffer().document.path()?.parent().map(Path::to_path_buf))
    }

    pub(super) fn quick_open(&mut self, _: &QuickOpen, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let root = self.quick_open_root(cx);
        // Nothing to search, so offer the files opened lately instead.
        let recent: Vec<String> = self
            .state
            .recent_files
            .iter()
            .filter(|path| path.is_file())
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        if root.is_none() && recent.is_empty() {
            window.push_notification(Notification::info("Open a folder or a file to search its files."), cx);
            return;
        }

        let command = self.quick_open.command.clone();
        command.update(cx, |state, cx| {
            state.set_query("", window, cx);
            state.set_loading(root.is_some(), window, cx);
        });
        self.quick_open.query.clear();
        self.quick_open.files.clear();
        self.quick_open.matches.clear();
        self.quick_open.recent = root.is_none();

        if let Some(root) = root {
            self.quick_open.root = root.clone();
            let show_hidden = self.settings.show_hidden_files;
            let scan = cx.background_spawn({
                let root = root.clone();
                async move { file_index::list_files(&root, show_hidden) }
            });
            self.quick_open._scan = Some(cx.spawn_in(window, async move |this, cx| {
                let files = scan.await;
                _ = this.update_in(cx, |workspace, window, cx| {
                    let quick_open = &mut workspace.quick_open;
                    if quick_open.recent || quick_open.root != root {
                        return;
                    }
                    quick_open.files = files;
                    quick_open.refresh_matches();
                    quick_open.command.update(cx, |state, cx| state.set_loading(false, window, cx));
                    cx.notify();
                });
            }));
        } else {
            // Absolute paths, which `root.join` passes through unchanged.
            self.quick_open.root = PathBuf::new();
            self.quick_open.files = recent;
            self.quick_open.refresh_matches();
            self.quick_open._scan = None;
        }

        let this = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let command = command.clone();
            let this = this.clone();
            dialog
                .close_button(false)
                .p_0()
                .w(px(DIALOG_WIDTH))
                .margin_top(px(72.))
                .content(move |content, _, cx| content.child(render_quick_open(&command, this.clone(), cx)))
        });
        self.quick_open.command.update(cx, |state, cx| state.focus(window, cx));
    }

    fn filter_quick_open(&mut self, query: &str, cx: &mut Context<Self>) {
        self.quick_open.query = query.to_string();
        self.quick_open.refresh_matches();
        cx.notify();
    }

    fn confirm_quick_open(&mut self, index: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = self.quick_open.matches.get(index.row) else {
            return;
        };
        let path = self.quick_open.root.join(file);
        window.close_dialog(cx);
        self.open_path(path, false, window, cx);
    }
}

fn render_quick_open(state: &Entity<CommandState>, this: WeakEntity<Workspace>, cx: &App) -> Command {
    let (matches, recent) = this
        .upgrade()
        .map(|workspace| {
            let quick_open = &workspace.read(cx).quick_open;
            (quick_open.matches.clone(), quick_open.recent)
        })
        .unwrap_or_default();
    let items = matches.into_iter().map(|file| {
        let (dir, name) = match file.rsplit_once('/') {
            Some((dir, name)) => (dir.to_string(), name.to_string()),
            None => (String::new(), file.clone()),
        };
        CommandItem::new().label(file).child(move |_, cx| {
            h_flex()
                .w_full()
                .gap_3()
                .items_center()
                .child(div().flex_none().child(name.clone()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(dir.clone()),
                )
                .into_any_element()
        })
    });

    let (query, confirm) = (this.clone(), this);
    Command::new(state)
        .bordered(false)
        .filterable(false)
        .placeholder(if recent { "Search recent files" } else { "Search files by name" })
        .max_h(px(400.))
        .items(items)
        .empty(|state, _, cx| {
            let message = if state.is_loading() { "Looking for files" } else { "No matching files" };
            div()
                .p_3()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message)
        })
        .on_query(move |text, _, cx| {
            _ = query.update(cx, |workspace, cx| workspace.filter_quick_open(text, cx));
        })
        .on_confirm(move |index, window, cx| {
            _ = confirm.update(cx, |workspace, cx| workspace.confirm_quick_open(index, window, cx));
        })
}
