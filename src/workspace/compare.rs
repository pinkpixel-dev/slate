use std::path::{Path, PathBuf};

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::Workspace;
use super::files::notify_error;
use crate::diff;
use crate::document::{self, Document};

actions!(slate, [CompareWithDisk]);

impl Workspace {
    pub(super) fn compare_with_disk(&mut self, _: &CompareWithDisk, window: &mut Window, cx: &mut Context<Self>) {
        let buffer = self.active_buffer();
        let Some(path) = buffer.document.path().map(Path::to_path_buf) else {
            window.push_notification(Notification::info("This tab isn't saved to a file yet."), cx);
            return;
        };
        let name = buffer.document.display_name();
        let mine = buffer.editor.read(cx).value().to_string();

        let work = cx.background_spawn({
            let (path, name) = (path.clone(), name.clone());
            async move {
                let disk = document::load(&path)?;
                let labels = (format!("{name} (on disk)"), format!("{name} (in Slate)"));
                std::io::Result::Ok(diff::unified(&disk.text, &mine, &labels.0, &labels.1))
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = work.await;
            _ = this.update_in(cx, |workspace, window, cx| match result {
                Ok(Some(text)) => workspace.show_comparison(format!("{name} (changes)"), text, window, cx),
                Ok(None) => window.push_notification(Notification::info(format!("{name} matches the file on disk.")), cx),
                Err(err) => notify_error(format!("Couldn't read {} to compare: {err}", path.display()), window, cx),
            });
        })
        .detach();
    }

    /// Shows a diff in a read-only tab, reusing the one from an earlier compare of the same file.
    fn show_comparison(&mut self, title: String, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let existing = self
            .buffers
            .iter()
            .position(|buffer| buffer.document.is_scratch() && buffer.document.display_name() == title);
        match existing {
            Some(index) => {
                let editor = self.buffers[index].editor.clone();
                editor.update(cx, |state, cx| state.set_value(text, window, cx));
                self.activate(index, window, cx);
            }
            None => {
                let document = Document::scratch(title, &PathBuf::from("changes.diff"));
                // Read-only comes from the editor element (`render_editor_column`).
                self.push_buffer(document, text, window, cx);
            }
        }
        cx.notify();
    }
}
