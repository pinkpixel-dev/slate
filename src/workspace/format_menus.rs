use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::Sizable as _;
use gpui_kit::*;

use super::Workspace;
use crate::text_format::{Indent, LineEnding};

/// Widths offered in the indentation menu.
const WIDTHS: [usize; 3] = [2, 4, 8];

impl Workspace {
    /// "Spaces: 4" in the status bar. Changing it affects what Tab inserts
    /// from now on; existing indentation is left as it is.
    pub(super) fn render_indent_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let indent = self.active_buffer().indent;

        Button::new("indentation")
            .ghost()
            .xsmall()
            .label(indent.label())
            .tooltip("Indentation")
            .dropdown_menu(move |mut menu, _, _| {
                let choices = [
                    ("Indent Using Spaces", Indent { hard_tabs: false, ..indent }),
                    ("Indent Using Tabs", Indent { hard_tabs: true, ..indent }),
                ];
                for (label, choice) in choices {
                    let pick = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label)
                            .checked(choice.hard_tabs == indent.hard_tabs)
                            .on_click(move |_, _, cx| {
                                _ = pick.update(cx, |workspace, cx| workspace.set_active_indent(choice, cx));
                            }),
                    );
                }
                menu = menu.separator();
                for width in WIDTHS {
                    let pick = this.clone();
                    let choice = Indent { width, ..indent };
                    menu = menu.item(
                        PopupMenuItem::new(format!("Width: {width}"))
                            .checked(width == indent.width)
                            .on_click(move |_, _, cx| {
                                _ = pick.update(cx, |workspace, cx| workspace.set_active_indent(choice, cx));
                            }),
                    );
                }
                menu
            })
    }

    /// "LF" or "CRLF" in the status bar. Picking the other one marks the tab
    /// unsaved, and the next save writes the new line endings.
    pub(super) fn render_line_ending_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let current = self.active_buffer().document.line_ending();

        Button::new("line-ending")
            .ghost()
            .xsmall()
            .label(current.label())
            .tooltip("Line Endings")
            .dropdown_menu(move |mut menu, _, _| {
                for (label, choice) in [("LF (Linux, macOS)", LineEnding::Lf), ("CRLF (Windows)", LineEnding::Crlf)] {
                    let pick = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label)
                            .checked(choice == current)
                            .on_click(move |_, window, cx| {
                                _ = pick.update(cx, |workspace, cx| workspace.set_active_line_ending(choice, window, cx));
                            }),
                    );
                }
                menu
            })
    }

    pub(super) fn set_active_indent(&mut self, indent: Indent, cx: &mut Context<Self>) {
        self.buffers[self.active].set_indent(indent, cx);
        cx.notify();
    }

    pub(super) fn set_active_line_ending(&mut self, line_ending: LineEnding, window: &mut Window, cx: &mut Context<Self>) {
        let document = &mut self.buffers[self.active].document;
        if document.line_ending() == line_ending {
            return;
        }
        document.set_line_ending(line_ending);
        document.mark_edited();
        self.sync_window_title(window);
        self.schedule_session_save(cx);
        cx.notify();
    }
}
