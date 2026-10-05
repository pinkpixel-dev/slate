use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{WindowExt as _, h_flex};
use gpui_kit::*;

use super::{PendingAction, Workspace};

impl Workspace {
    /// Runs `action` straight away, or first asks whether to save unsaved edits.
    pub(super) fn guard_unsaved(
        &mut self,
        action: PendingAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.document.is_dirty() {
            self.run_pending(action, window, cx);
            return;
        }
        if window.has_active_dialog(cx) {
            return;
        }

        let this = cx.entity().downgrade();
        let title: SharedString = format!("Save changes to {}?", self.document.display_name()).into();

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
                                        workspace.run_pending(action, window, cx)
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
                                        workspace.save_then(Some(action), window, cx)
                                    });
                                }),
                        ),
                )
        });
    }
}
