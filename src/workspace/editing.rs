use std::ops::Range;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::{KEY_CONTEXT, Workspace};
use crate::line_ops::{self, Case, LineEdit};
use crate::theme::{MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE};

actions!(
    slate,
    [
        DuplicateLine,
        MoveLineUp,
        MoveLineDown,
        ToggleComment,
        SortLines,
        UpperCase,
        LowerCase,
        TitleCase,
        ZoomIn,
        ZoomOut,
        ResetZoom
    ]
);

const ZOOM_STEP: f32 = 1.;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-d", DuplicateLine, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-up", MoveLineUp, Some(KEY_CONTEXT)),
        KeyBinding::new("alt-down", MoveLineDown, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-/", ToggleComment, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-=", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-+", ZoomIn, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl--", ZoomOut, Some(KEY_CONTEXT)),
        KeyBinding::new("ctrl-0", ResetZoom, Some(KEY_CONTEXT)),
    ]);
}

impl Workspace {
    /// Applies a line edit to the active editor as one undo step. Only while
    /// the editor has focus, so these keys don't reach into it from the find bar.
    fn edit_lines(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&str, Range<usize>) -> Option<LineEdit>,
    ) {
        let editor = self.active_buffer().editor.clone();
        if !editor.read(cx).focus_handle(cx).is_focused(window) {
            cx.propagate();
            return;
        }
        let (text, selection) = {
            let state = editor.read(cx);
            (state.value(), state.selected_range())
        };
        let Some(edit) = edit(&text, selection) else {
            return;
        };
        editor.update(cx, |state, cx| {
            state.set_selected_range(edit.range, cx);
            state.replace(edit.text, window, cx);
            state.set_selected_range(edit.selection, cx);
        });
    }

    pub(super) fn duplicate_line(&mut self, _: &DuplicateLine, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, |text, selection| Some(line_ops::duplicate(text, selection)));
    }

    pub(super) fn move_line_up(&mut self, _: &MoveLineUp, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, line_ops::move_up);
    }

    pub(super) fn move_line_down(&mut self, _: &MoveLineDown, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, line_ops::move_down);
    }

    pub(super) fn toggle_comment(&mut self, _: &ToggleComment, window: &mut Window, cx: &mut Context<Self>) {
        let Some(comment) = self.active_buffer().document.language().comment() else {
            return;
        };
        self.edit_lines(window, cx, |text, selection| line_ops::toggle_comment(text, selection, comment));
    }

    pub(super) fn sort_lines(&mut self, _: &SortLines, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, line_ops::sort_lines);
    }

    pub(super) fn upper_case(&mut self, _: &UpperCase, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, |text, selection| line_ops::change_case(text, selection, Case::Upper));
    }

    pub(super) fn lower_case(&mut self, _: &LowerCase, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, |text, selection| line_ops::change_case(text, selection, Case::Lower));
    }

    pub(super) fn title_case(&mut self, _: &TitleCase, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_lines(window, cx, |text, selection| line_ops::change_case(text, selection, Case::Title));
    }

    pub(super) fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_by(ZOOM_STEP, cx);
    }

    pub(super) fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_by(-ZOOM_STEP, cx);
    }

    pub(super) fn reset_zoom(&mut self, _: &ResetZoom, _: &mut Window, cx: &mut Context<Self>) {
        if self.settings.editor_zoom != 0. {
            self.settings.editor_zoom = 0.;
            self.apply_zoom(cx);
        }
    }

    /// Zoom only lasts until Slate closes; Settings holds the lasting font size.
    fn zoom_by(&mut self, step: f32, cx: &mut Context<Self>) {
        let size = f32::from(cx.theme().mono_font_size);
        if (step > 0. && size >= MAX_EDITOR_FONT_SIZE) || (step < 0. && size <= MIN_EDITOR_FONT_SIZE) {
            return;
        }
        self.settings.editor_zoom += step;
        self.apply_zoom(cx);
    }

    fn apply_zoom(&mut self, cx: &mut Context<Self>) {
        crate::theme::apply(&self.settings, cx);
        self.ensure_minimap(cx);
        cx.notify();
    }
}
