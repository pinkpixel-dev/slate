use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::text::{TextView, TextViewState};
use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::*;

use super::buffer::BufferId;
use super::{KEY_CONTEXT, Workspace};

actions!(slate, [TogglePreview]);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-v", TogglePreview, Some(KEY_CONTEXT))]);
}

impl Workspace {
    /// Shows or hides the active tab's rendered Markdown, next to its editor.
    pub(super) fn toggle_preview(&mut self, _: &TogglePreview, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        let open = self.active_buffer().preview.is_none();
        self.set_preview(id, open, cx);
    }

    pub(super) fn set_preview(&mut self, id: BufferId, open: bool, cx: &mut Context<Self>) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        let buffer = &self.buffers[index];
        let preview = open.then(|| {
            let text = buffer.editor.read(cx).value();
            cx.new(|cx| TextViewState::markdown(&text, cx))
        });
        self.buffers[index].preview = preview;
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// Sends the tab's current text to its preview. Kit parses it off the UI thread.
    pub(super) fn sync_preview(&mut self, id: BufferId, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.iter().find(|buffer| buffer.id == id) else {
            return;
        };
        let Some(preview) = buffer.preview.clone() else {
            return;
        };
        let text = buffer.editor.read(cx).value();
        preview.update(cx, |state, cx| state.set_text(&text, cx));
    }

    /// The editor alone, or split with the active tab's preview on the right.
    pub(super) fn render_with_preview(&self, editor: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        let Some(preview) = self.active_buffer().preview.clone() else {
            return editor;
        };
        let theme = cx.theme();
        h_resizable("preview-split")
            .child(resizable_panel().child(editor))
            .child(
                resizable_panel().child(
                    v_flex()
                        .id("markdown-preview")
                        .size_full()
                        .bg(theme.background)
                        .border_l_1()
                        .border_color(theme.border)
                        .px_6()
                        .py_4()
                        .child(TextView::new(&preview).scrollable(true)),
                ),
            )
            .into_any_element()
    }
}
