use std::path::{Path, PathBuf};

use gpui_kit::component::resizable::{ResizablePanelEvent, ResizableState, h_resizable, resizable_panel};
use gpui_kit::*;

use super::files::notify_error;
use super::{OpenFolder, ToggleSidebar, Workspace};
use crate::sidebar::{Sidebar, SidebarEvent};

const SIDEBAR_WIDTH: f32 = 240.;

impl Workspace {
    pub(super) fn open_folder(&mut self, _: &OpenFolder, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Folder".into()),
        });

        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some(paths))) => {
                if let Some(dir) = paths.into_iter().next() {
                    _ = this.update(cx, |workspace, cx| workspace.show_folder(dir, cx));
                }
            }
            Ok(Err(err)) => {
                _ = this.update_in(cx, |_, window, cx| {
                    notify_error(format!("Couldn't open the folder picker: {err}"), window, cx);
                });
            }
            _ => {}
        })
        .detach();
    }

    /// Opens a folder in the sidebar and shows it.
    pub fn show_folder(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let dir = std::path::absolute(&dir).unwrap_or(dir);
        self.sidebar.update(cx, |sidebar, cx| sidebar.open_folder(dir, cx));
        self.sidebar_open = true;
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// With no folder open, showing the sidebar opens the active file's folder.
    pub(super) fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        if self.sidebar_open && self.sidebar.read(cx).root().is_none() {
            let folder = self
                .active_buffer()
                .document
                .path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = folder {
                self.sidebar.update(cx, |sidebar, cx| sidebar.open_folder(folder, cx));
            }
        }
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// Remembers the sidebar's width once a drag ends.
    pub(super) fn on_body_resized(
        &mut self,
        layout: Entity<ResizableState>,
        _: &ResizablePanelEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(width) = layout.read(cx).sizes().first().map(|width| f32::from(*width)) else {
            return;
        };
        if self.state.sidebar_width != Some(width) {
            self.state.sidebar_width = Some(width);
            self.save_state();
        }
    }

    pub(super) fn on_sidebar_event(
        &mut self,
        _: &Entity<Sidebar>,
        event: &SidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SidebarEvent::OpenFile(path) => self.open_path(path.clone(), false, window, cx),
            SidebarEvent::ShowHiddenChanged(show) => {
                self.settings.show_hidden_files = *show;
                self.save_settings();
            }
        }
    }

    /// The sidebar (when shown) and the editor column, side by side.
    pub(super) fn render_body(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let editor = self.render_editor_column(window, cx);
        if !self.sidebar_open {
            return editor;
        }
        // The layout state outlives the panel, so the width survives hiding the
        // sidebar; the saved width only seeds the first render.
        h_resizable("workspace-body")
            .with_state(&self.body_layout)
            .child(
                resizable_panel()
                    .size(px(self.state.sidebar_width.unwrap_or(SIDEBAR_WIDTH)))
                    .size_range(px(160.)..px(480.))
                    .child(self.sidebar.clone()),
            )
            .child(editor)
            .into_any_element()
    }
}
