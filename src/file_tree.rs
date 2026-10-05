use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use gpui_kit::SharedString;
use gpui_kit::component::tree::TreeItem;

/// Folders hidden along with dotfiles unless "show hidden files" is on.
const HIDDEN_NAMES: &[&str] = &["node_modules", "target", "__pycache__"];

/// One entry inside a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

impl Node {
    fn is_hidden(&self) -> bool {
        self.name.starts_with('.') || (self.is_dir && HIDDEN_NAMES.contains(&self.name.as_str()))
    }
}

/// What a tree row id stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    File(PathBuf),
    Folder(PathBuf),
    /// "Loading…" or "Empty" under a folder.
    Placeholder,
}

/// Lists a folder: folders first, then files, each sorted case-insensitively.
/// Hidden entries are kept here and filtered when building tree items.
pub fn list_dir(dir: &Path) -> io::Result<Vec<Node>> {
    let mut nodes = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        // `is_dir` follows symlinks, so a link to a folder behaves like one.
        let is_dir = path.is_dir();
        let name = entry.file_name().to_string_lossy().into_owned();
        nodes.push(Node { path, name, is_dir });
    }
    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(nodes)
}

/// The folder shown in the sidebar: what's been loaded, what's expanded, and
/// how tree row ids map back to paths.
pub struct FileTree {
    root: PathBuf,
    children: HashMap<PathBuf, Vec<Node>>,
    expanded: HashSet<PathBuf>,
    rows: HashMap<SharedString, RowKind>,
}

impl FileTree {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            children: HashMap::new(),
            expanded: HashSet::new(),
            rows: HashMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_loaded(&self, dir: &Path) -> bool {
        self.children.contains_key(dir)
    }

    pub fn set_loaded(&mut self, dir: PathBuf, nodes: Vec<Node>) {
        self.children.insert(dir, nodes);
    }

    /// Forgets a folder that no longer exists, along with everything under it.
    pub fn forget(&mut self, dir: &Path) {
        self.children.retain(|path, _| !path.starts_with(dir));
        self.expanded.retain(|path| !path.starts_with(dir));
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.expanded.contains(dir)
    }

    pub fn set_expanded(&mut self, dir: &Path, expanded: bool) {
        if expanded {
            self.expanded.insert(dir.to_path_buf());
        } else {
            self.expanded.remove(dir);
        }
    }

    pub fn row(&self, id: &str) -> Option<&RowKind> {
        self.rows.get(id)
    }

    pub fn row_id(path: &Path) -> SharedString {
        path.to_string_lossy().into_owned().into()
    }

    /// Builds the tree items for the root's contents. Unloaded folders get a
    /// "Loading…" child so Kit's tree still treats them as folders.
    pub fn items(&mut self, show_hidden: bool) -> Vec<TreeItem> {
        self.rows.clear();
        let root = self.root.clone();
        self.folder_items(&root, show_hidden)
    }

    fn folder_items(&mut self, dir: &Path, show_hidden: bool) -> Vec<TreeItem> {
        let Some(nodes) = self.children.get(dir).cloned() else {
            return Vec::new();
        };
        nodes
            .iter()
            .filter(|node| show_hidden || !node.is_hidden())
            .map(|node| {
                let id = Self::row_id(&node.path);
                if !node.is_dir {
                    self.rows.insert(id.clone(), RowKind::File(node.path.clone()));
                    return TreeItem::new(id, node.name.clone());
                }
                self.rows.insert(id.clone(), RowKind::Folder(node.path.clone()));
                let mut children = self.folder_items(&node.path, show_hidden);
                if children.is_empty() {
                    let label = if self.is_loaded(&node.path) { "Empty" } else { "Loading…" };
                    let placeholder: SharedString = format!("{id}\0placeholder").into();
                    self.rows.insert(placeholder.clone(), RowKind::Placeholder);
                    children.push(TreeItem::new(placeholder, label).disabled(true));
                }
                TreeItem::new(id, node.name.clone())
                    .children(children)
                    .expanded(self.is_expanded(&node.path))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-tree-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["src", ".git", "node_modules", "docs"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        for file in ["b.txt", "A.md", ".env", "src/main.rs"] {
            std::fs::write(dir.join(file), "").unwrap();
        }
        dir
    }

    fn labels(items: &[TreeItem]) -> Vec<String> {
        items.iter().map(|item| item.label.to_string()).collect()
    }

    #[test]
    fn lists_folders_first_then_files_case_insensitively() {
        let dir = fixture("sort");
        let names: Vec<String> = list_dir(&dir).unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(names, [".git", "docs", "node_modules", "src", ".env", "A.md", "b.txt"]);
    }

    #[test]
    fn hides_dotfiles_and_junk_folders_unless_asked() {
        let dir = fixture("hidden");
        let mut tree = FileTree::new(dir.clone());
        tree.set_loaded(dir.clone(), list_dir(&dir).unwrap());

        assert_eq!(labels(&tree.items(false)), ["docs", "src", "A.md", "b.txt"]);
        assert_eq!(tree.items(true).len(), 7);
    }

    #[test]
    fn unloaded_and_empty_folders_still_show_as_folders() {
        let dir = fixture("placeholders");
        let mut tree = FileTree::new(dir.clone());
        tree.set_loaded(dir.clone(), list_dir(&dir).unwrap());
        tree.set_loaded(dir.join("docs"), Vec::new());

        let items = tree.items(false);
        let docs = &items[0];
        let src = &items[1];
        assert!(docs.is_folder() && src.is_folder());
        assert_eq!(labels(&docs.children), ["Empty"]);
        assert_eq!(labels(&src.children), ["Loading…"]);
        assert_eq!(tree.row(&docs.children[0].id), Some(&RowKind::Placeholder));
        assert_eq!(tree.row(&src.id), Some(&RowKind::Folder(dir.join("src"))));
    }

    #[test]
    fn forgetting_a_folder_drops_its_subfolders_too() {
        let dir = fixture("forget");
        let mut tree = FileTree::new(dir.clone());
        tree.set_loaded(dir.join("src"), Vec::new());
        tree.set_loaded(dir.join("src/inner"), Vec::new());
        tree.set_expanded(&dir.join("src"), true);

        tree.forget(&dir.join("src"));
        assert!(!tree.is_loaded(&dir.join("src/inner")));
        assert!(!tree.is_expanded(&dir.join("src")));
    }
}
