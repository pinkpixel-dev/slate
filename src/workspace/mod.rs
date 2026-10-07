mod buffer;
mod chrome;
mod color_swatches;
mod disk_watch;
mod editing;
mod files;
mod find_bar;
mod folders;
mod format_menus;
mod go_to_line;
mod minimap;
mod palette;
mod preview;
mod prefs;
mod quick_open;
mod session;
mod settings_panel;
mod tab_menu;
mod tab_strip;
mod unsaved;

#[cfg(test)]
mod find_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wrap_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod disk_tests;
#[cfg(test)]
mod preview_tests;
#[cfg(test)]
mod minimap_tests;
#[cfg(test)]
mod editing_tests;
#[cfg(test)]
mod color_swatch_tests;

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::command::CommandState;
use gpui_kit::component::input::{Editor, InputState};
use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::document::Document;
use crate::sidebar::Sidebar;
use crate::storage::{AppState, Settings, Storage};
use crate::theme::{ThemeCatalog, ThemeWatcher};
use buffer::{Buffer, BufferId};
use disk_watch::FileWatcher;
use find_bar::FindBar;
use minimap::MinimapDrag;
use preview::TogglePreview;
use quick_open::QuickOpenState;
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
        ToggleWordWrap,
        ToggleSidebar,
        OpenFolder,
        OpenSettings,
        FindNext,
        FindPrevious,
    ]
);

const KEY_CONTEXT: &str = "Workspace";

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
        KeyBinding::new("alt-z", ToggleWordWrap, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-b", ToggleSidebar, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-shift-o", OpenFolder, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-,", OpenSettings, Some(KEY_CONTEXT)),
    ]);
    find_bar::init(cx);
    palette::init(cx);
    preview::init(cx);
    minimap::init(cx);
    editing::init(cx);
    go_to_line::init(cx);
    quick_open::init(cx);
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
    window: AnyWindowHandle,
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
    find: FindBar,
    palette: Entity<CommandState>,
    go_to_line: Entity<InputState>,
    quick_open: QuickOpenState,
    file_watcher: Option<FileWatcher>,
    /// Where the minimap was last painted, for its mouse handlers.
    minimap_bounds: Rc<Cell<Bounds<Pixels>>>,
    minimap_drag: Option<MinimapDrag>,
    /// This window restores and saves the session: launched without
    /// arguments, with `restore_session` on.
    session_active: bool,
    pending_session_save: Option<Task<()>>,
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
        let mut subscriptions = vec![cx.subscribe_in(&sidebar, window, Self::on_sidebar_event)];
        let (find, find_subscriptions) = FindBar::new(window, cx);
        subscriptions.extend(find_subscriptions);
        let go_to_line = go_to_line::new_input(window, cx);

        let mut workspace = Self {
            focus_handle: cx.focus_handle(),
            window: window.window_handle(),
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
            find,
            palette: cx.new(|cx| CommandState::new(window, cx)),
            go_to_line,
            quick_open: QuickOpenState::new(window, cx),
            // Tests drive `on_files_changed` directly: real inotify events would
            // wake GPUI's deterministic test scheduler from another thread.
            file_watcher: if cfg!(test) {
                None
            } else {
                FileWatcher::new(window, cx)
                    .inspect_err(|err| eprintln!("slate: not watching open files for changes: {err}"))
                    .ok()
            },
            minimap_bounds: Rc::default(),
            minimap_drag: None,
            session_active: false,
            pending_session_save: None,
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
        let buffer = Buffer::new(
            id,
            document,
            text,
            self.show_whitespace,
            self.settings.word_wrap,
            window,
            cx,
        );
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
        self.sync_find(window, cx);
        self.sync_file_watches();
        self.ensure_minimap(cx);
        self.schedule_session_save(cx);
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

    /// The tab's text changed, by an edit or by loading new text: update what mirrors it.
    fn text_changed(&mut self, id: BufferId, cx: &mut Context<Self>) {
        self.sync_preview(id, cx);
        self.mark_minimap_stale(id, cx);
        self.refresh_colors(id, cx);
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
        let editor = Editor::new(&self.active_buffer().editor)
            .h_full()
            .bordered(false)
            .aria_label("Editor")
            .into_any_element();
        let editor = self.render_with_swatches(editor, cx);
        let editor = self.render_with_minimap(editor, cx);
        v_flex()
            .size_full()
            .child(self.render_tab_strip(window, cx))
            .children(self.render_disk_bar(cx))
            .children(self.render_find_bar(cx))
            .child(div().flex_1().min_h_0().child(self.render_with_preview(editor, cx)))
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
            .on_action(cx.listener(Self::toggle_word_wrap))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::open_settings))
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::find_replace))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_action(cx.listener(Self::on_escape))
            .on_action(cx.listener(Self::toggle_command_palette))
            .on_action(cx.listener(Self::toggle_preview))
            .on_action(cx.listener(Self::toggle_minimap))
            .on_action(cx.listener(Self::duplicate_line))
            .on_action(cx.listener(Self::move_line_up))
            .on_action(cx.listener(Self::move_line_down))
            .on_action(cx.listener(Self::toggle_comment))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::reset_zoom))
            .on_action(cx.listener(Self::go_to_line))
            .on_action(cx.listener(Self::quick_open))
            .on_action(cx.listener(Self::edit_color))
            .on_action(cx.listener(Self::toggle_color_swatches))
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(window, cx))
            .child(div().flex_1().min_h_0().child(self.render_body(window, cx)))
            .child(self.render_status_bar(cx))
    }
}
