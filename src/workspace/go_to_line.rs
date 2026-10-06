use gpui_kit::component::input::{Input, InputState, Position, RopeExt as _};
use gpui_kit::component::{WindowExt as _, v_flex};
use gpui_kit::*;

use super::{KEY_CONTEXT, Workspace};

actions!(slate, [GoToLine]);

const DIALOG_WIDTH: f32 = 340.;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-g", GoToLine, Some(KEY_CONTEXT))]);
}

/// The field shown in the Go to Line dialog. It lives on the workspace so
/// the dialog can be reopened with it.
pub(super) fn new_input(window: &mut Window, cx: &mut Context<Workspace>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx))
}

/// Reads `12`, `12:5`, or `12,5` as a 1-based line and optional column.
pub(super) fn parse_target(text: &str) -> Option<(u32, Option<u32>)> {
    let text = text.trim();
    let (line, column) = match text.split_once([':', ',']) {
        Some((line, column)) => (line, Some(column.trim())),
        None => (text, None),
    };
    let line = line.trim().parse::<u32>().ok().filter(|&line| line > 0)?;
    let column = match column {
        Some("") | None => None,
        Some(column) => Some(column.parse::<u32>().ok().filter(|&column| column > 0)?),
    };
    Some((line, column))
}

impl Workspace {
    pub(super) fn go_to_line(&mut self, _: &GoToLine, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let editor = self.active_buffer().editor.read(cx);
        let lines = editor.text().lines_len().max(1);
        let current = editor.cursor_position().line + 1;
        let input = self.go_to_line.clone();
        input.update(cx, |input, cx| {
            input.set_placeholder(format!("Line 1 to {lines}, or line:column"), window, cx);
            input.set_value(current.to_string(), window, cx);
            input.select_all(window, cx);
        });

        let this = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let input = input.clone();
            let this = this.clone();
            dialog
                .close_button(false)
                // Enter confirms the dialog. Returning false keeps it open.
                .on_ok(move |_, window, cx| {
                    this.update(cx, |workspace, cx| workspace.confirm_go_to_line(window, cx))
                        .unwrap_or(true)
                })
                .p_2()
                .w(px(DIALOG_WIDTH))
                .margin_top(px(72.))
                .content(move |content, _, _| {
                    content.child(v_flex().child(Input::new(&input).aria_label("Go to line")))
                })
        });
        self.go_to_line.update(cx, |input, cx| input.focus(window, cx));
    }

    /// Jumps to the typed line. Returns false for anything that isn't a line
    /// number, which keeps the dialog open.
    fn confirm_go_to_line(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((line, column)) = parse_target(&self.go_to_line.read(cx).value()) else {
            return false;
        };
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| {
            let last = state.text().lines_len().saturating_sub(1) as u32;
            let position = Position::new((line - 1).min(last), column.map_or(0, |column| column - 1));
            state.unfold_at(position, cx);
            state.set_cursor_position(position, window, cx);
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn reads_lines_and_columns() {
        assert_eq!(parse_target("12"), Some((12, None)));
        assert_eq!(parse_target(" 12:5 "), Some((12, Some(5))));
        assert_eq!(parse_target("12,5"), Some((12, Some(5))));
        assert_eq!(parse_target("12:"), Some((12, None)));
    }

    #[test]
    fn refuses_what_isnt_a_line() {
        assert_eq!(parse_target(""), None);
        assert_eq!(parse_target("0"), None);
        assert_eq!(parse_target("abc"), None);
        assert_eq!(parse_target("3:x"), None);
        assert_eq!(parse_target("-4"), None);
    }
}
