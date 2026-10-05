use gpui_kit::component::input::{Editor, EditorState, TabSize};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme as _, TitleBar, h_flex, v_flex};
use gpui_kit::*;

actions!(slate, [Quit]);

const TAB_SIZE: usize = 4;

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
    cx.on_action(|_: &Quit, cx| cx.quit());
}

/// The single editor window: title bar, editor surface and status bar.
pub struct Workspace {
    editor: Entity<EditorState>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(false)
                .tab_size(TabSize {
                    tab_size: TAB_SIZE,
                    hard_tabs: false,
                })
        });

        // The status bar reads the cursor position, so redraw whenever the editor does.
        let subscriptions = vec![cx.observe(&editor, |_, _, cx| cx.notify())];

        editor.update(cx, |state, cx| state.focus(window, cx));

        Self {
            editor,
            _subscriptions: subscriptions,
        }
    }

    fn render_title_bar(&self, cx: &App) -> impl IntoElement {
        TitleBar::new().child(
            h_flex()
                .gap_2()
                .items_center()
                .text_sm()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(cx.theme().foreground)
                        .child("Slate"),
                )
                .child(div().text_color(cx.theme().muted_foreground).child("Untitled")),
        )
    }

    fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        let position = self.editor.read(cx).cursor_position();
        let cursor = format!("Ln {}, Col {}", position.line + 1, position.character + 1);

        StatusBar::new()
            .text_color(cx.theme().muted_foreground)
            .left("Plain Text")
            .right(cursor)
            .right(format!("Spaces: {TAB_SIZE}"))
            .right("UTF-8")
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Editor::new(&self.editor).h_full().bordered(false).aria_label("Editor")),
            )
            .child(self.render_status_bar(cx))
    }
}
