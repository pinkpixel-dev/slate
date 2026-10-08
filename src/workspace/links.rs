use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr as _;

use gpui_kit::component::input::{DefinitionProvider, EditorState, Rope, RopeExt as _};
use gpui_kit::*;
use lsp_types::{LocationLink, Range as LspRange, Uri};

use super::Workspace;
use super::buffer::BufferId;
use crate::links::{self, Link};

/// Kit's Go to Definition hook, used for links: holding Ctrl over a URL or
/// path underlines it, and Ctrl+click opens it. URLs go to the browser
/// (Kit's own handling); paths open in Slate through `show_document`.
struct LinkProvider {
    workspace: WeakEntity<Workspace>,
    id: BufferId,
}

impl DefinitionProvider for LinkProvider {
    fn definitions(&self, text: &Rope, offset: usize, _: &mut Window, cx: &mut App) -> Task<Result<Vec<LocationLink>>> {
        let point = text.offset_to_point(offset);
        let line = text.slice_line(point.row).to_string();
        let column = offset - text.line_start_offset(point.row);
        let Some((range, link)) = links::link_at(&line, column) else {
            return Task::ready(Ok(Vec::new()));
        };

        let target = match link {
            Link::Url(url) => Uri::from_str(&url).ok(),
            Link::Path(path) => {
                let bases = self
                    .workspace
                    .upgrade()
                    .map(|workspace| workspace.read(cx).link_bases(self.id, cx))
                    .unwrap_or_default();
                links::resolve(&path, &bases).and_then(|path| file_uri(&path))
            }
        };
        let Some(target_uri) = target else {
            return Task::ready(Ok(Vec::new()));
        };

        let line_start = text.line_start_offset(point.row);
        let origin = LspRange::new(
            text.offset_to_position(line_start + range.start),
            text.offset_to_position(line_start + range.end),
        );
        Task::ready(Ok(vec![LocationLink {
            origin_selection_range: Some(origin),
            target_uri,
            target_range: LspRange::default(),
            target_selection_range: LspRange::default(),
        }]))
    }
}

impl Workspace {
    /// Where relative paths in a tab are looked up: its file's folder, then the sidebar folder.
    fn link_bases(&self, id: BufferId, cx: &App) -> Vec<PathBuf> {
        let file_dir = self
            .buffers
            .iter()
            .find(|buffer| buffer.id == id)
            .and_then(|buffer| buffer.document.path()?.parent().map(Path::to_path_buf));
        let root = self.sidebar.read(cx).root().map(Path::to_path_buf);
        file_dir.into_iter().chain(root).collect()
    }
}

/// Hooks Ctrl+click links up on a new tab's editor.
pub(super) fn enable(editor: &mut EditorState, id: BufferId, workspace: WeakEntity<Workspace>) {
    let lsp = editor.lsp_mut();
    lsp.definition_provider = Some(Rc::new(LinkProvider {
        workspace: workspace.clone(),
        id,
    }));
    lsp.show_document = Some(Rc::new(move |params, window, cx| {
        let Some(path) = file_path(&params.uri) else {
            return false; // A web link: Kit opens it in the browser.
        };
        // This runs inside the editor's update, and opening a tab reads editors.
        let workspace = workspace.clone();
        window.defer(cx, move |window, cx| {
            _ = workspace.update(cx, |workspace, cx| workspace.open_path(path, false, window, cx));
        });
        true
    }));
}

fn file_uri(path: &Path) -> Option<Uri> {
    let url = url::Url::from_file_path(path).ok()?;
    Uri::from_str(url.as_str()).ok()
}

fn file_path(uri: &Uri) -> Option<PathBuf> {
    let url = url::Url::parse(uri.as_str()).ok()?;
    (url.scheme() == "file").then(|| url.to_file_path().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr as _;

    use lsp_types::Uri;

    use super::{file_path, file_uri};

    #[test]
    fn file_uris_round_trip_odd_names() {
        let path = PathBuf::from("/tmp/a b/ü#1.txt");
        let uri = file_uri(&path).unwrap();
        assert!(uri.as_str().starts_with("file:///tmp/a%20b/"));
        assert_eq!(file_path(&uri), Some(path));
        assert_eq!(file_path(&Uri::from_str("https://pinkpixel.dev").unwrap()), None);
    }
}
