use std::rc::Rc;

use gpui_kit::component::input::{EditorState, InputEvent, TabSize};
use gpui_kit::component::text::TextViewState;
use gpui_kit::*;

use super::Workspace;
use super::minimap::MinimapState;
use crate::color_values::{self, ColorLine};
use crate::document::Document;
use crate::tab_color::TabColor;
use crate::text_format::Indent;

/// Stable identity for an open tab, so async work can find it after tabs move or close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

/// One open tab: its editor state, the file behind it, and its accent color.
pub struct Buffer {
    pub id: BufferId,
    pub editor: Entity<EditorState>,
    pub document: Document,
    /// A hand-picked color. In "color by language" mode this still wins.
    pub color: Option<TabColor>,
    /// The file changed on disk while this tab had unsaved edits.
    pub disk_conflict: bool,
    /// Saves still writing. Disk checks wait for them, so our own writes
    /// don't look like outside changes.
    pub saves_in_flight: u32,
    /// The rendered Markdown shown next to the editor, while it's open.
    pub preview: Option<Entity<TextViewState>>,
    pub minimap: MinimapState,
    /// The color values in the text, for the swatches after each line.
    pub colors: Rc<Vec<ColorLine>>,
    /// What Tab inserts, guessed from the file and changeable from the status bar.
    pub indent: Indent,
    _subscriptions: Vec<Subscription>,
}

impl Buffer {
    pub fn new(
        id: BufferId,
        document: Document,
        text: String,
        show_whitespace: bool,
        word_wrap: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let language = document.language().id;
        let indent = Indent::detect(&text).unwrap_or_default();
        let colors = Rc::new(color_values::scan(&text));
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language)
                .line_number(true)
                .folding(true)
                .soft_wrap(word_wrap)
                // Slate draws its own find bar (`find_bar.rs`), so Ctrl+F goes up to the workspace.
                .searchable(false)
                .show_whitespaces(show_whitespace)
                .tab_size(tab_size(indent))
        });
        if !text.is_empty() {
            // `set_value` doesn't emit change events, so loading isn't an edit.
            editor.update(cx, |state, cx| state.set_value(text, window, cx));
        }

        let subscriptions = vec![
            // The status bar reads the cursor position, so redraw whenever the editor does.
            cx.observe(&editor, |_, _, cx| cx.notify()),
            cx.subscribe_in(&editor, window, move |this, _, event, window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Some(buffer) = this.buffer_mut(id) else {
                    return;
                };
                let was_dirty = buffer.document.is_dirty();
                buffer.document.mark_edited();
                this.schedule_session_save(cx);
                this.text_changed(id, cx);
                if !was_dirty {
                    this.sync_window_title(window);
                    cx.notify();
                }
            }),
        ];

        Self {
            id,
            editor,
            document,
            color: None,
            disk_conflict: false,
            saves_in_flight: 0,
            preview: None,
            minimap: MinimapState::default(),
            colors,
            indent,
            _subscriptions: subscriptions,
        }
    }

    pub fn set_indent(&mut self, indent: Indent, cx: &mut App) {
        self.indent = indent;
        self.editor.update(cx, |state, cx| state.set_tab_size(tab_size(indent), cx));
    }

    /// Picks up the indentation of newly loaded text, when it has any.
    pub fn detect_indent(&mut self, text: &str, cx: &mut App) {
        if let Some(indent) = Indent::detect(text) {
            self.set_indent(indent, cx);
        }
    }

    /// An empty, untouched untitled tab that opening a file can reuse.
    pub fn is_pristine(&self, cx: &App) -> bool {
        self.document.path().is_none()
            && !self.document.is_dirty()
            && self.editor.read(cx).text().len() == 0
    }
}

fn tab_size(indent: Indent) -> TabSize {
    TabSize {
        tab_size: indent.width,
        hard_tabs: indent.hard_tabs,
    }
}
