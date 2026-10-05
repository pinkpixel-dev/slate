mod view;
mod watcher;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui_kit::component::tree::{TreeEvent, TreeState};
use gpui_kit::*;

use crate::file_tree::{self, FileTree, RowKind};
use watcher::DirWatcher;

actions!(sidebar, [OpenSelected]);

pub fn init(cx: &mut App) {
    // Enter opens a selected file (or toggles a folder) in the file tree.
    cx.bind_keys([KeyBinding::new("enter", OpenSelected, Some("Tree"))]);
}

/// Events the workspace reacts to.
pub enum SidebarEvent {
    OpenFile(PathBuf),
    ShowHiddenChanged(bool),
}

impl EventEmitter<SidebarEvent> for Sidebar {}

/// The file sidebar: one folder's tree, loaded lazily and kept current by a watcher.
pub struct Sidebar {
    pub(crate) tree_state: Entity<TreeState>,
    tree: Option<FileTree>,
    show_hidden: bool,
    watcher: Option<DirWatcher>,
    _subscriptions: Vec<Subscription>,
}

impl Sidebar {
    pub fn new(show_hidden: bool, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let subscriptions = vec![cx.subscribe(&tree_state, |this, _, event, cx| match event {
            TreeEvent::Expanded(id) => this.expand_row(id, true, cx),
            TreeEvent::Collapsed(id) => this.expand_row(id, false, cx),
        })];
        Self {
            tree_state,
            tree: None,
            show_hidden,
            watcher: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.tree.as_ref().map(FileTree::root)
    }

    pub fn open_folder(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.tree = Some(FileTree::new(root.clone()));
        self.watcher = DirWatcher::new(cx)
            .inspect_err(|err| eprintln!("slate: file watching is off: {err}"))
            .ok();
        self.refresh(cx);
        self.load_dir(root, cx);
    }

    pub fn close_folder(&mut self, cx: &mut Context<Self>) {
        self.tree = None;
        self.watcher = None;
        self.refresh(cx);
    }

    pub fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        self.show_hidden = !self.show_hidden;
        cx.emit(SidebarEvent::ShowHiddenChanged(self.show_hidden));
        self.refresh(cx);
    }

    fn expand_row(&mut self, id: &SharedString, expanded: bool, cx: &mut Context<Self>) {
        let Some(tree) = self.tree.as_mut() else { return };
        let Some(RowKind::Folder(dir)) = tree.row(id).cloned() else { return };
        tree.set_expanded(&dir, expanded);
        if expanded && !tree.is_loaded(&dir) {
            self.load_dir(dir, cx);
        }
    }

    /// Reads a folder in the background, then shows and watches it.
    fn load_dir(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let read = cx.background_spawn({
            let dir = dir.clone();
            async move { file_tree::list_dir(&dir) }
        });
        cx.spawn(async move |this, cx| {
            let result = read.await;
            _ = this.update(cx, |sidebar, cx| sidebar.apply_listing(dir, result, cx));
        })
        .detach();
    }

    fn apply_listing(&mut self, dir: PathBuf, result: std::io::Result<Vec<file_tree::Node>>, cx: &mut Context<Self>) {
        // The folder may have been closed or swapped while the read ran.
        let Some(tree) = self.tree.as_mut() else { return };
        if !dir.starts_with(tree.root()) {
            return;
        }
        match result {
            Ok(nodes) => {
                tree.set_loaded(dir.clone(), nodes);
                if let Some(watcher) = self.watcher.as_mut() {
                    watcher.watch(&dir);
                }
            }
            Err(_) => {
                tree.forget(&dir);
                if let Some(watcher) = self.watcher.as_mut() {
                    watcher.unwatch(&dir);
                }
            }
        }
        self.refresh(cx);
    }

    /// Rescans loaded folders touched by a batch of file events.
    fn rescan_changed(&mut self, changed: HashSet<PathBuf>, cx: &mut Context<Self>) {
        let Some(tree) = self.tree.as_ref() else { return };
        let dirs: HashSet<PathBuf> = changed
            .iter()
            .flat_map(|path| [Some(path.clone()), path.parent().map(Path::to_path_buf)])
            .flatten()
            .filter(|dir| tree.is_loaded(dir))
            .collect();
        for dir in dirs {
            self.load_dir(dir, cx);
        }
    }

    /// Rebuilds the tree rows from the model, keeping the selection.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let show_hidden = self.show_hidden;
        let items = self.tree.as_mut().map(|tree| tree.items(show_hidden)).unwrap_or_default();
        self.tree_state.update(cx, |state, cx| {
            let selected = state.selected_item().map(|item| item.id.clone());
            state.set_items(items, cx);
            if let Some(ix) = selected.and_then(|id| state.index_of(&id)) {
                state.set_selected_index(Some(ix), cx);
            }
        });
        cx.notify();
    }

    fn open_selected(&mut self, _: &OpenSelected, _: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.tree_state.read(cx).selected_item().map(|item| item.id.clone()) else {
            return;
        };
        let Some(tree) = self.tree.as_mut() else { return };
        match tree.row(&id).cloned() {
            Some(RowKind::File(path)) => cx.emit(SidebarEvent::OpenFile(path)),
            Some(RowKind::Folder(dir)) => {
                let expanded = !tree.is_expanded(&dir);
                tree.set_expanded(&dir, expanded);
                if expanded && !tree.is_loaded(&dir) {
                    self.load_dir(dir, cx);
                }
                self.refresh(cx);
            }
            _ => {}
        }
    }

    fn row_kind(&self, id: &str) -> Option<RowKind> {
        self.tree.as_ref().and_then(|tree| tree.row(id).cloned())
    }
}
