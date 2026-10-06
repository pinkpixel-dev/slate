use gpui_kit::assets::IconName as CatalogIcon;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{
    EditorState, Escape, IndentInline, Input, InputEvent, InputState, OutdentInline, Replace, Search,
};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::buffer::BufferId;
use super::{FindNext, FindPrevious, KEY_CONTEXT, Workspace};

actions!(slate, [ReplaceNext, ReplaceAll, ToggleCaseSensitive]);

const FIND_CONTEXT: &str = "FindBar";
const QUERY_WIDTH: f32 = 340.;
/// Picking the match nearest the cursor walks Kit's matcher one step at a
/// time, so give up and start at the first match past this many steps.
const MAX_ANCHOR_STEPS: usize = 512;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-f", Search, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-h", Replace, Some(KEY_CONTEXT)),
        KeyBinding::new("f3", FindNext, Some(KEY_CONTEXT)),
        KeyBinding::new("shift-f3", FindPrevious, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-c", ToggleCaseSensitive, Some(FIND_CONTEXT)),
        KeyBinding::new("ctrl-alt-enter", ReplaceAll, Some(FIND_CONTEXT)),
    ]);
}

/// Slate's find and replace bar. It draws its own fields and drives the
/// search engine built into each `EditorState`.
pub(super) struct FindBar {
    pub open: bool,
    pub replace_mode: bool,
    pub case_sensitive: bool,
    pub query: Entity<InputState>,
    pub replacement: Entity<InputState>,
    /// The tab whose editor currently shows the match highlights.
    searched: Option<BufferId>,
}

impl FindBar {
    pub fn new(window: &mut Window, cx: &mut Context<Workspace>) -> (Self, Vec<Subscription>) {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let replacement = cx.new(|cx| InputState::new(window, cx).placeholder("Replace"));
        let subscriptions = vec![
            cx.subscribe_in(&query, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.run_search(window, cx),
                InputEvent::PressEnter { shift: true, .. } => this.step_match(false, window, cx),
                InputEvent::PressEnter { .. } => this.step_match(true, window, cx),
                _ => {}
            }),
            cx.subscribe_in(&replacement, window, |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.replace_next(&ReplaceNext, window, cx);
                }
            }),
        ];
        let bar = Self {
            open: false,
            replace_mode: false,
            case_sensitive: false,
            query,
            replacement,
            searched: None,
        };
        (bar, subscriptions)
    }
}

impl Workspace {
    pub(super) fn find(&mut self, _: &Search, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(false, window, cx);
    }

    pub(super) fn find_replace(&mut self, _: &Replace, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(true, window, cx);
    }

    /// Shows the bar, seeded with the editor's selection when it's a single line.
    fn open_find(&mut self, replace_mode: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.find.open = true;
        self.find.replace_mode = replace_mode;
        let selected = self.active_buffer().editor.read(cx).selected_text().to_string();
        let seed = (!selected.is_empty() && !selected.contains('\n')).then_some(selected);
        self.find.query.update(cx, |input, cx| {
            if let Some(seed) = seed {
                input.set_value(seed, window, cx);
            }
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        // `set_value` doesn't emit a change event, so search by hand.
        self.run_search(window, cx);
        cx.notify();
    }

    pub(super) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find.open = false;
        self.clear_search_highlights(cx);
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    /// Escape closes the bar from the bar or the editor. Anything else gets the key.
    pub(super) fn on_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open {
            self.close_find(window, cx);
        } else {
            cx.propagate();
        }
    }

    /// Keeps the highlights on the active tab after switching tabs.
    pub(super) fn sync_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open && self.find.searched != Some(self.active_buffer().id) {
            self.run_search(window, cx);
        }
    }

    fn clear_search_highlights(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.find.searched.take()
            && let Some(index) = self.index_of(id)
        {
            self.buffers[index].editor.update(cx, |state, cx| state.close_search(cx));
        }
    }

    /// Searches the active editor for the query and moves to the match at or after the cursor.
    fn run_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let buffer_id = self.active_buffer().id;
        if self.find.searched != Some(buffer_id) {
            self.clear_search_highlights(cx);
        }
        let query = self.find.query.read(cx).value();
        let editor = self.active_buffer().editor.clone();
        if query.is_empty() {
            self.clear_search_highlights(cx);
            cx.notify();
            return;
        }
        self.find.searched = Some(buffer_id);
        let case_insensitive = !self.find.case_sensitive;
        editor.update(cx, |state, cx| {
            let anchor = state.selected_range().start;
            state.set_search_query(query, case_insensitive, cx);
            jump_to_anchor(state, anchor, cx);
        });
        select_current_match(&editor, window, cx);
        cx.notify();
    }

    fn step_match(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.searched != Some(self.active_buffer().id) {
            self.run_search(window, cx);
            return;
        }
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            if forward {
                state.next_search_match(cx);
            } else {
                state.previous_search_match(cx);
            }
        });
        select_current_match(&editor, window, cx);
    }

    /// F3 with the bar closed reopens it on the last query.
    pub(super) fn find_next(&mut self, _: &FindNext, window: &mut Window, cx: &mut Context<Self>) {
        self.step_or_open(true, window, cx);
    }

    pub(super) fn find_previous(&mut self, _: &FindPrevious, window: &mut Window, cx: &mut Context<Self>) {
        self.step_or_open(false, window, cx);
    }

    fn step_or_open(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.open {
            self.step_match(forward, window, cx);
        } else {
            self.open_find(false, window, cx);
        }
    }

    fn replace_next(&mut self, _: &ReplaceNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.searched != Some(self.active_buffer().id) {
            self.run_search(window, cx);
        }
        let replacement = self.find.replacement.read(cx).value();
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            state.replace_current_search_match(&replacement, window, cx);
        });
        select_current_match(&editor, window, cx);
    }

    fn replace_all(&mut self, _: &ReplaceAll, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.searched != Some(self.active_buffer().id) {
            self.run_search(window, cx);
        }
        let replacement = self.find.replacement.read(cx).value();
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            state.replace_all_search_matches(&replacement, window, cx);
        });
    }

    fn toggle_case_sensitive(&mut self, _: &ToggleCaseSensitive, window: &mut Window, cx: &mut Context<Self>) {
        self.find.case_sensitive = !self.find.case_sensitive;
        self.run_search(window, cx);
    }

    fn toggle_replace_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find.replace_mode = !self.find.replace_mode;
        let field = if self.find.replace_mode {
            &self.find.replacement
        } else {
            &self.find.query
        };
        field.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// Tab moves between the two fields instead of leaving the bar.
    fn cycle_find_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.find.replace_mode {
            return;
        }
        let in_query = self.find.query.read(cx).focus_handle(cx).is_focused(window);
        let field = if in_query { &self.find.replacement } else { &self.find.query };
        field.update(cx, |input, cx| input.focus(window, cx));
    }

    /// "3 of 12", "No results", or nothing while the query is empty.
    pub(super) fn match_label(&self, cx: &App) -> Option<String> {
        if self.find.query.read(cx).value().is_empty() {
            return None;
        }
        let matcher = &self.active_buffer().editor.read(cx).search_session().matcher;
        Some(match matcher.current() {
            Some(index) => format!("{} of {}", index + 1, matcher.len()),
            None => "No results".into(),
        })
    }

    pub(super) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.find.open {
            return None;
        }
        let theme = cx.theme();
        let has_matches = !self.active_buffer().editor.read(cx).search_session().matcher.is_empty();
        let no_results = self.match_label(cx).filter(|_| !has_matches).is_some();
        let label = self.match_label(cx).unwrap_or_default();
        let replace_mode = self.find.replace_mode;

        let bar = v_flex()
            .id("find-bar")
            .key_context(FIND_CONTEXT)
            .on_action(cx.listener(Self::toggle_case_sensitive))
            .on_action(cx.listener(Self::replace_all))
            .on_action(cx.listener(|this, _: &IndentInline, window, cx| this.cycle_find_focus(window, cx)))
            .on_action(cx.listener(|this, _: &OutdentInline, window, cx| this.cycle_find_focus(window, cx)))
            .w_full()
            .gap_1p5()
            .px_2()
            .py_1p5()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Button::new("find-toggle-replace")
                            .ghost()
                            .xsmall()
                            .icon(if replace_mode { IconName::ChevronDown } else { IconName::ChevronRight })
                            .tooltip_with_action("Toggle Replace", &Replace, Some(KEY_CONTEXT))
                            .on_click(cx.listener(|this, _, window, cx| this.toggle_replace_mode(window, cx))),
                    )
                    .child(
                        div().w(px(QUERY_WIDTH)).min_w_0().flex_shrink(1.).child(
                            Input::new(&self.find.query)
                                .small()
                                .prefix(Icon::new(IconName::Search).xsmall().text_color(theme.muted_foreground))
                                .suffix(
                                    Button::new("find-case")
                                        .ghost()
                                        .xsmall()
                                        .compact()
                                        .icon(IconName::CaseSensitive)
                                        .selected(self.find.case_sensitive)
                                        .tooltip_with_action("Match Case", &ToggleCaseSensitive, Some(FIND_CONTEXT))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.toggle_case_sensitive(&ToggleCaseSensitive, window, cx)
                                        })),
                                ),
                        ),
                    )
                    .child(
                        div()
                            .id("find-count")
                            .min_w(px(72.))
                            .px_1()
                            .text_xs()
                            .whitespace_nowrap()
                            .text_color(if no_results { theme.danger } else { theme.muted_foreground })
                            .child(label),
                    )
                    .child(
                        Button::new("find-previous")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ArrowUp)
                            .disabled(!has_matches)
                            .tooltip_with_action("Previous Match", &FindPrevious, Some(KEY_CONTEXT))
                            .on_click(cx.listener(|this, _, window, cx| this.step_match(false, window, cx))),
                    )
                    .child(
                        Button::new("find-next")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ArrowDown)
                            .disabled(!has_matches)
                            .tooltip_with_action("Next Match", &FindNext, Some(KEY_CONTEXT))
                            .on_click(cx.listener(|this, _, window, cx| this.step_match(true, window, cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("find-close")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip("Close (Escape)")
                            .on_click(cx.listener(|this, _, window, cx| this.close_find(window, cx))),
                    ),
            )
            .when(replace_mode, |bar| {
                bar.child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        // Lines the field up under the find field.
                        .pl(px(26.))
                        .child(
                            div()
                                .w(px(QUERY_WIDTH))
                                .min_w_0()
                                .flex_shrink(1.)
                                .child(Input::new(&self.find.replacement).small()),
                        )
                        .child(
                            Button::new("replace-next")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Replace)
                                .disabled(!has_matches)
                                .tooltip("Replace (Enter)")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_next(&ReplaceNext, window, cx)
                                })),
                        )
                        .child(
                            Button::new("replace-all")
                                .ghost()
                                .xsmall()
                                .icon(Icon::new(CatalogIcon::ReplaceAll))
                                .disabled(!has_matches)
                                .tooltip_with_action("Replace All", &ReplaceAll, Some(FIND_CONTEXT))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_all(&ReplaceAll, window, cx)
                                })),
                        ),
                )
            });
        Some(bar.into_any_element())
    }
}

/// Moves Kit's current match to the first one that ends at or after `anchor`.
///
/// Kit's matcher has no public way to set the current match, so this lands on
/// the first match (previous, then next, from index 0) and steps from there,
/// going whichever way round is shorter.
fn jump_to_anchor(state: &mut EditorState, anchor: usize, cx: &mut Context<EditorState>) {
    let ranges = state.search_session().matcher.matched_ranges();
    if ranges.is_empty() {
        return;
    }
    let len = ranges.len();
    let target = ranges.partition_point(|range| range.end < anchor) % len;
    state.previous_search_match(cx);
    state.next_search_match(cx);
    let (steps, forward) = if target <= len - target {
        (target, true)
    } else {
        (len - target, false)
    };
    if steps > MAX_ANCHOR_STEPS {
        return;
    }
    for _ in 0..steps {
        if forward {
            state.next_search_match(cx);
        } else {
            state.previous_search_match(cx);
        }
    }
}

/// Selects the current match in the editor, so closing the bar leaves the cursor on it.
fn select_current_match(editor: &Entity<EditorState>, _: &mut Window, cx: &mut App) {
    editor.update(cx, |state, cx| {
        let matcher = &state.search_session().matcher;
        let Some(range) = matcher.current().and_then(|index| matcher.matched_ranges().get(index).cloned()) else {
            return;
        };
        state.set_selected_range(range, cx);
    });
}
