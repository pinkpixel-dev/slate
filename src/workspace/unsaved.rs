use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{WindowExt as _, h_flex};
use gpui_kit::*;

use super::buffer::BufferId;
use super::{PendingAction, Workspace};

impl Workspace {
    /// Closes a tab, asking first if it has unsaved edits.
    pub(super) fn close_buffer(&mut self, id: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        if self.buffers[index].document.is_dirty() {
            self.activate(index, window, cx);
            self.ask_to_save(id, PendingAction::CloseBuffer(id), window, cx);
        } else {
            self.remove_buffer(id, window, cx);
        }
    }

    /// Closes the window once every tab is saved or discarded, asking about
    /// each unsaved tab in turn.
    pub(super) fn close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_keeps_edits(cx) {
            window.remove_window();
            return;
        }
        match self.buffers.iter().position(|buffer| buffer.document.is_dirty()) {
            Some(index) => {
                let id = self.buffers[index].id;
                self.activate(index, window, cx);
                self.ask_to_save(id, PendingAction::CloseWindow, window, cx);
            }
            None => window.remove_window(),
        }
    }

    /// Returns `true` when the window can close right away.
    pub(super) fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.close_keeps_edits(cx) {
            return true;
        }
        if !self.buffers.iter().any(|buffer| buffer.document.is_dirty()) {
            return true;
        }
        self.close_window(window, cx);
        false
    }

    /// Continues an action after its buffer was saved.
    pub(super) fn run_pending(&mut self, action: PendingAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            PendingAction::CloseBuffer(id) => self.remove_buffer(id, window, cx),
            PendingAction::CloseWindow => self.close_window(window, cx),
        }
    }

    fn discard_then(&mut self, id: BufferId, action: PendingAction, window: &mut Window, cx: &mut Context<Self>) {
        self.remove_buffer(id, window, cx);
        if action == PendingAction::CloseWindow {
            self.close_window(window, cx);
        }
    }

    fn ask_to_save(&mut self, id: BufferId, action: PendingAction, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(index) = self.index_of(id) else {
            return;
        };

        let this = cx.entity().downgrade();
        let title: SharedString =
            format!("Save changes to {}?", self.buffers[index].document.display_name()).into();

        window.open_dialog(cx, move |dialog, _, _| {
            let discard = this.clone();
            let save = this.clone();

            dialog
                .w(px(420.))
                .title(title.clone())
                .child("Your changes will be lost if you don't save them.")
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("dialog-discard")
                                .ghost()
                                .label("Don't Save")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    _ = discard.update(cx, |workspace, cx| {
                                        workspace.discard_then(id, action, window, cx)
                                    });
                                }),
                        )
                        .child(
                            Button::new("dialog-cancel")
                                .outline()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("dialog-save")
                                .primary()
                                .label("Save")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    _ = save.update(cx, |workspace, cx| {
                                        workspace.save_then(id, Some(action), window, cx)
                                    });
                                }),
                        ),
                )
        });
    }
}
