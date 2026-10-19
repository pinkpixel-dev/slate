use std::rc::Rc;

use gpui_kit::component::input::{CompletionProvider, EditorState, Rope, RopeExt as _};
use gpui_kit::*;
use lsp_types::{
    CompletionContext, CompletionResponse, InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse,
};

use super::Workspace;
use crate::word_complete;

/// Lines searched on each side of the cursor, so a huge file doesn't get
/// copied on every pause in typing.
const SEARCH_LINES: usize = 2000;

/// Kit's completion hook, used only for its inline suggestion: Kit shows the
/// ghost text after a short pause and Tab accepts it. The popup menu stays off.
struct WordCompleter {
    workspace: WeakEntity<Workspace>,
}

impl CompletionProvider for WordCompleter {
    fn completions(
        &self,
        _: &Rope,
        _: usize,
        _: CompletionContext,
        _: &mut Window,
        _: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        Task::ready(Ok(CompletionResponse::Array(Vec::new())))
    }

    fn inline_completion(
        &self,
        text: &Rope,
        offset: usize,
        _: InlineCompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<InlineCompletionResponse>> {
        let enabled = self
            .workspace
            .upgrade()
            .is_some_and(|workspace| workspace.read(cx).settings.word_complete);
        if !enabled {
            return Task::ready(Ok(InlineCompletionResponse::Array(Vec::new())));
        }

        let text = text.clone();
        cx.background_spawn(async move {
            let row = text.offset_to_point(offset).row;
            let first = row.saturating_sub(SEARCH_LINES);
            let last = (row + SEARCH_LINES + 1).min(text.lines_len());
            let window = text.slice_lines(first..last).to_string();
            let cursor = offset - text.line_start_offset(first);
            let items = word_complete::suggest(&window, cursor)
                .map(|insert_text| InlineCompletionItem {
                    insert_text,
                    filter_text: None,
                    range: None,
                    command: None,
                    insert_text_format: None,
                })
                .into_iter()
                .collect();
            Ok(InlineCompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(&self, _: usize, _: &str, _: &mut App) -> bool {
        false
    }
}

/// Hooks word completion up on a new tab's editor.
pub(super) fn enable(editor: &mut EditorState, workspace: WeakEntity<Workspace>) {
    editor.lsp_mut().completion_provider = Some(Rc::new(WordCompleter { workspace }));
}
