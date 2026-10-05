mod buffer;
mod chrome;
mod files;
mod folders;
mod prefs;
mod settings_panel;
mod tab_menu;
mod tab_strip;
mod unsaved;

#[cfg(test)]
mod tests;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::Editor;
use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::document::Document;
use crate::sidebar::Sidebar;
use crate::storage::{AppState, Settings, Storage};
use crate::theme::{ThemeCatalog, ThemeWatcher};
use buffer::{Buffer, BufferId};
use settings_panel::FontPickers;

actions!(
    slate,
    [
        NewFile,
        Open,
        Save,
        SaveAs,
        CloseTab,
        NextTab,
        PreviousTab,
        Quit,
        ToggleWhitespace,
        ToggleSidebar,
        OpenFolder,
        OpenSettings,
    ]
);

const KEY_CONTEXT: &str = "Workspace";
const TAB_SIZE: usize = 4;

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewFile, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-o", Open, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-s", Save, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-w", CloseTab, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-tab", NextTab, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-pagedown", NextTab, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-pageup", PreviousTab, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-q", Quit, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-b", ToggleSidebar, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-o", OpenFolder, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-,", OpenSettings, Some(KEY_CONTEXT)),
    ]);
}

/// What to do once a buffer's unsaved changes have been saved or discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    CloseBuffer(BufferId),
    CloseWindow,
}

/// The editor window: title bar, tab strip, the active buffer's editor, and status bar.
pub struct Workspace {
    focus_handle: FocusHandle,
    buffers: Vec<Buffer>,
    active: usize,
    next_buffer_id: u64,
    storage: Storage,
    settings: Settings,
    state: AppState,
    show_whitespace: bool,
    tab_scroll: ScrollHandle,
    sidebar: Entity<Sidebar>,
    sidebar_open: bool,
    font_pickers: Option<FontPickers>,
    _theme_watcher: Option<ThemeWatcher>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(storage: Storage, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Window-manager closes (Alt+F4, taskbar) ask first when there are unsaved edits.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |workspace, cx| workspace.should_close(window, cx))
                .unwrap_or(true)
        });

        let settings = storage.load_settings();
        crate::theme::apply(&settings, cx);
        let themes_dir = ThemeCatalog::global(cx).custom_dir().to_path_buf();
        let theme_watcher = ThemeWatcher::new(&themes_dir, window, cx, Self::on_themes_changed)
            .inspect_err(|err| eprintln!("slate: theme hot reload is off: {err}"))
            .ok();
        let sidebar = cx.new(|cx| Sidebar::new(settings.show_hidden_files, cx));
        let subscriptions = vec![cx.subscribe_in(&sidebar, window, Self::on_sidebar_event)];

        let mut workspace = Self {
            focus_handle: cx.focus_handle(),
            buffers: Vec::new(),
            active: 0,
            next_buffer_id: 0,
            settings,
            state: storage.load_state(),
            storage,
            show_whitespace: false,
            tab_scroll: ScrollHandle::new(),
            sidebar,
            sidebar_open: false,
            font_pickers: None,
            _theme_watcher: theme_watcher,
            _subscriptions: subscriptions,
        };
        workspace.new_untitled(window, cx);
        workspace
    }

    fn active_buffer(&self) -> &Buffer {
        &self.buffers[self.active]
    }

    fn index_of(&self, id: BufferId) -> Option<usize> {
        self.buffers.iter().position(|buffer| buffer.id == id)
    }

    fn buffer_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.iter_mut().find(|buffer| buffer.id == id)
    }

    /// Adds a buffer after the active one and switches to it.
    fn push_buffer(&mut self, document: Document, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let id = BufferId(self.next_buffer_id);
        self.next_buffer_id += 1;
        let buffer = Buffer::new(id, document, text, self.show_whitespace, window, cx);
        let index = if self.buffers.is_empty() { 0 } else { self.active + 1 };
        self.buffers.insert(index, buffer);
        self.activate(index, window, cx);
    }

    fn new_untitled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let taken: Vec<usize> = self
            .buffers
            .iter()
            .filter_map(|buffer| buffer.document.untitled_number())
            .collect();
        let number = (1..).find(|n| !taken.contains(n)).unwrap_or(1);
        self.push_buffer(Document::untitled(number), String::new(), window, cx);
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        self.active = index;
        buffer.editor.update(cx, |state, cx| state.focus(window, cx));
        self.tab_scroll.scroll_to_item(index);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Removes a buffer without asking. The window always keeps at least one tab.
    fn remove_buffer(&mut self, id: BufferId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        self.buffers.remove(index);
        if self.buffers.is_empty() {
            self.active = 0;
            self.new_untitled(window, cx);
            return;
        }
        if index < self.active || self.active >= self.buffers.len() {
            self.active = self.active.saturating_sub(1);
        }
        self.activate(self.active, window, cx);
    }

    /// Moves a dragged tab so it lands at `target`.
    fn move_buffer(&mut self, id: BufferId, target: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.index_of(id) else {
            return;
        };
        let active_id = self.active_buffer().id;
        let buffer = self.buffers.remove(from);
        let target = target.min(self.buffers.len());
        self.buffers.insert(target, buffer);
        let active = self.index_of(active_id).unwrap_or(0);
        self.activate(active, window, cx);
    }

    fn sync_window_title(&self, window: &mut Window) {
        let document = &self.active_buffer().document;
        let marker = if document.is_dirty() { "• " } else { "" };
        window.set_window_title(&format!("{marker}{} - Slate", document.display_name()));
    }

    fn new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.new_untitled(window, cx);
    }

    fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.active_buffer().id;
        self.close_buffer(id, window, cx);
    }

    fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        let index = (self.active + 1) % self.buffers.len();
        self.activate(index, window, cx);
    }

    fn previous_tab(&mut self, _: &PreviousTab, window: &mut Window, cx: &mut Context<Self>) {
        let index = (self.active + self.buffers.len() - 1) % self.buffers.len();
        self.activate(index, window, cx);
    }

    fn toggle_whitespace(&mut self, _: &ToggleWhitespace, window: &mut Window, cx: &mut Context<Self>) {
        self.show_whitespace = !self.show_whitespace;
        let show = self.show_whitespace;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_show_whitespaces(show, window, cx));
        }
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, window: &mut Window, cx: &mut Context<Self>) {
        self.close_window(window, cx);
    }
}

impl Workspace {
    /// The tab strip and the active editor.
    fn render_editor_column(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .child(self.render_tab_strip(window, cx))
            .child(
                div().flex_1().min_h_0().child(
                    Editor::new(&self.active_buffer().editor)
                        .h_full()
                        .bordered(false)
                        .aria_label("Editor"),
                ),
            )
            .into_any_element()
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
            .on_action(cx.listener(Self::close_tab))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::previous_tab))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::toggle_whitespace))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::open_settings))
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(window, cx))
            .child(div().flex_1().min_h_0().child(self.render_body(window, cx)))
            .child(self.render_status_bar(cx))
    }
}
