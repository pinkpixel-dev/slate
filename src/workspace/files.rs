use std::path::PathBuf;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::{NewFile, Open, PendingAction, Save, SaveAs, Workspace};
use crate::document::{self, Document};

impl Workspace {
    pub(super) fn new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.guard_unsaved(PendingAction::NewFile, window, cx);
    }

    pub(super) fn open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.guard_unsaved(PendingAction::Open, window, cx);
    }

    pub(super) fn save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        self.save_then(None, window, cx);
    }

    pub(super) fn save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_save_as(None, window, cx);
    }

    pub(super) fn reset_to_untitled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document = Document::untitled();
        let language = self.document.language().id;
        self.editor.update(cx, |state, cx| {
            state.set_highlighter(language, cx);
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        self.sync_window_title(window);
        cx.notify();
    }

    pub(super) fn prompt_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open".into()),
        });

        cx.spawn_in(window, async move |this, cx| {
            let path = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Err(err)) => {
                    _ = this.update_in(cx, |_, window, cx| {
                        notify_error(format!("Couldn't open the file picker: {err}"), window, cx);
                    });
                    None
                }
                _ => None,
            };
            if let Some(path) = path {
                _ = this.update_in(cx, |workspace, window, cx| workspace.load(path, window, cx));
            }
        })
        .detach();
    }

    pub fn load(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let read = cx.background_spawn({
            let path = path.clone();
            async move { document::read_text(&path) }
        });

        cx.spawn_in(window, async move |this, cx| {
            let result = read.await;
            _ = this.update_in(cx, |workspace, window, cx| match result {
                Ok(text) => workspace.show_loaded(path, text, window, cx),
                Err(err) => notify_error(format!("Couldn't open {}: {err}", path.display()), window, cx),
            });
        })
        .detach();
    }

    fn show_loaded(&mut self, path: PathBuf, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.document = Document::from_path(path);
        let language = self.document.language().id;
        self.editor.update(cx, |state, cx| {
            state.set_highlighter(language, cx);
            state.set_value(text, window, cx);
            state.focus(window, cx);
        });
        self.sync_window_title(window);
        cx.notify();
    }

    /// Saves to the current path, or asks for one first. Runs `after` once the
    /// write succeeds.
    pub(super) fn save_then(
        &mut self,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.document.path() {
            Some(path) => self.write(path.to_path_buf(), after, window, cx),
            None => self.prompt_save_as(after, window, cx),
        }
    }

    fn prompt_save_as(
        &mut self,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let picked = cx.prompt_for_new_path(
            &self.document.directory(),
            Some(&self.document.display_name()),
        );

        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some(path))) => {
                _ = this.update_in(cx, |workspace, window, cx| workspace.write(path, after, window, cx));
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
        path: PathBuf,
        after: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let revision = self.document.revision();
        let text = self.editor.read(cx).value();
        let write = cx.background_spawn({
            let path = path.clone();
            async move { document::write_text(&path, &text) }
        });

        cx.spawn_in(window, async move |this, cx| {
            let result = write.await;
            _ = this.update_in(cx, |workspace, window, cx| match result {
                Ok(()) => {
                    let language_before = workspace.document.language();
                    workspace.document.mark_saved(path, revision);
                    let language = workspace.document.language();
                    if language != language_before {
                        workspace
                            .editor
                            .update(cx, |state, cx| state.set_highlighter(language.id, cx));
                    }
                    workspace.sync_window_title(window);
                    cx.notify();
                    if let Some(action) = after {
                        workspace.run_pending(action, window, cx);
                    }
                }
                Err(err) => notify_error(format!("Couldn't save {}: {err}", path.display()), window, cx),
            });
        })
        .detach();
    }
}

fn notify_error(message: String, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::error(message), cx);
}
