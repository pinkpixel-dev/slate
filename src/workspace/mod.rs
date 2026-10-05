mod chrome;
mod files;
mod unsaved;

#[cfg(test)]
mod tests;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{Editor, EditorState, InputEvent, TabSize};
use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::document::Document;

actions!(slate, [NewFile, Open, Save, SaveAs, Quit, ToggleWhitespace]);

const KEY_CONTEXT: &str = "Workspace";
const TAB_SIZE: usize = 4;

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewFile, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-o", Open, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-s", Save, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-q", Quit, Some(KEY_CONTEXT)),
    ]);
}

/// What to do once unsaved changes have been saved or discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    NewFile,
    Open,
    Close,
}

/// The single editor window: title bar, editor surface and status bar.
pub struct Workspace {
    focus_handle: FocusHandle,
    editor: Entity<EditorState>,
    document: Document,
    show_whitespace: bool,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(crate::language::Language::PLAIN.id)
                .line_number(true)
                .folding(true)
                .soft_wrap(false)
                .tab_size(TabSize {
                    tab_size: TAB_SIZE,
                    hard_tabs: false,
                })
        });

        let subscriptions = vec![
            // The status bar reads the cursor position, so redraw whenever the editor does.
            cx.observe(&editor, |_, _, cx| cx.notify()),
            cx.subscribe_in(&editor, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let was_dirty = this.document.is_dirty();
                    this.document.mark_edited();
                    if !was_dirty {
                        this.sync_window_title(window);
                        cx.notify();
                    }
                }
            }),
        ];

        // Window-manager closes (Alt+F4, taskbar) ask first when there are unsaved edits.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |workspace, cx| workspace.should_close(window, cx))
                .unwrap_or(true)
        });

        editor.update(cx, |state, cx| state.focus(window, cx));

        let workspace = Self {
            focus_handle: cx.focus_handle(),
            editor,
            document: Document::untitled(),
            show_whitespace: false,
            _subscriptions: subscriptions,
        };
        workspace.sync_window_title(window);
        workspace
    }

    fn sync_window_title(&self, window: &mut Window) {
        let marker = if self.document.is_dirty() { "• " } else { "" };
        window.set_window_title(&format!("{marker}{} - Slate", self.document.display_name()));
    }

    fn toggle_whitespace(&mut self, _: &ToggleWhitespace, window: &mut Window, cx: &mut Context<Self>) {
        self.show_whitespace = !self.show_whitespace;
        let show = self.show_whitespace;
        self.editor
            .update(cx, |state, cx| state.set_show_whitespaces(show, window, cx));
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, window: &mut Window, cx: &mut Context<Self>) {
        self.guard_unsaved(PendingAction::Close, window, cx);
    }

    /// Returns `true` when the window can close right away.
    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.document.is_dirty() {
            return true;
        }
        self.guard_unsaved(PendingAction::Close, window, cx);
        false
    }

    fn run_pending(&mut self, action: PendingAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            PendingAction::NewFile => self.reset_to_untitled(window, cx),
            PendingAction::Open => self.prompt_open(window, cx),
            PendingAction::Close => window.remove_window(),
        }
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::new_file))
            .on_action(cx.listener(Self::open))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::save_as))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::toggle_whitespace))
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Editor::new(&self.editor).h_full().bordered(false).aria_label("Editor")),
            )
            .child(self.render_status_bar(cx))
    }
}
