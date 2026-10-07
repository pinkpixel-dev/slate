//! Editing commands that Kit's editor doesn't have: duplicate, move up and
//! down, toggle comment, sort lines, change case, and the save cleanup. Each
//! returns a single replacement, so the editor can apply it as one undoable edit.

use std::ops::Range;

use crate::language::Comment;

/// Replace `range` with `text`, then select `selection` (offsets in the new text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEdit {
    pub range: Range<usize>,
    pub text: String,
    pub selection: Range<usize>,
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map_or(text.len(), |i| offset + i)
}

/// The lines a selection touches, without the last line break. A selection
/// that ends at the very start of a line leaves that line out, the way
/// selecting whole lines with Shift+Down does.
fn line_block(text: &str, selection: &Range<usize>) -> Range<usize> {
    let start = line_start(text, selection.start);
    let mut end = selection.end;
    if end > selection.start && end > start && text.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    start..line_end(text, end)
}

fn shift(range: &Range<usize>, by: isize, len: usize) -> Range<usize> {
    let move_one = |offset: usize| (offset as isize + by).clamp(0, len as isize) as usize;
    move_one(range.start)..move_one(range.end)
}

pub fn duplicate(text: &str, selection: Range<usize>) -> LineEdit {
    let block = line_block(text, &selection);
    let lines = &text[block.clone()];
    LineEdit {
        range: block.end..block.end,
        text: format!("\n{lines}"),
        selection: shift(&selection, lines.len() as isize + 1, text.len() + lines.len() + 1),
    }
}

/// `None` when the lines are already at the top.
pub fn move_up(text: &str, selection: Range<usize>) -> Option<LineEdit> {
    let block = line_block(text, &selection);
    if block.start == 0 {
        return None;
    }
    let previous_start = line_start(text, block.start - 1);
    let previous = &text[previous_start..block.start - 1];
    let lines = &text[block.clone()];
    Some(LineEdit {
        range: previous_start..block.end,
        text: format!("{lines}\n{previous}"),
        selection: shift(&selection, -(previous.len() as isize + 1), text.len()),
    })
}

/// `None` when the lines are already at the bottom.
pub fn move_down(text: &str, selection: Range<usize>) -> Option<LineEdit> {
    let block = line_block(text, &selection);
    if block.end == text.len() {
        return None;
    }
    let next_end = line_end(text, block.end + 1);
    let next = &text[block.end + 1..next_end];
    let lines = &text[block.clone()];
    Some(LineEdit {
        range: block.start..next_end,
        text: format!("{next}\n{lines}"),
        selection: shift(&selection, next.len() as isize + 1, text.len()),
    })
}

/// One change inside the block: at `at` (in the old text), `removed` bytes
/// went away and `inserted` bytes came in.
struct Change {
    at: usize,
    removed: usize,
    inserted: usize,
}

/// Where an old offset lands after `changes` (sorted by position). With
/// `stay_left`, an offset sitting exactly where text was inserted stays
/// before it, which keeps a selection's start ahead of a new comment marker.
fn map_offset(offset: usize, changes: &[Change], stay_left: bool) -> usize {
    let mut moved = offset as isize;
    for change in changes {
        if offset < change.at || (offset == change.at && change.removed == 0 && stay_left) {
            break;
        }
        if offset >= change.at + change.removed {
            moved += change.inserted as isize - change.removed as isize;
        } else {
            // Inside removed text: land where it was.
            moved -= (offset - change.at) as isize;
            break;
        }
    }
    moved.max(0) as usize
}

/// Sorts the selected lines, A to Z, ignoring case (ties keep their order).
/// With nothing selected, sorts the whole document. The trailing newline
/// stays put. `None` when the lines are already sorted.
pub fn sort_lines(text: &str, selection: Range<usize>) -> Option<LineEdit> {
    let range = if selection.is_empty() {
        0..text.strip_suffix('\n').unwrap_or(text).len()
    } else {
        line_block(text, &selection)
    };
    let mut lines: Vec<&str> = text[range.clone()].split('\n').collect();
    lines.sort_by_cached_key(|line| line.to_lowercase());
    let sorted = lines.join("\n");
    if sorted == text[range.clone()] {
        return None;
    }
    let selection = if selection.is_empty() { selection } else { range.start..range.start + sorted.len() };
    Some(LineEdit { range, text: sorted, selection })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Upper,
    Lower,
    Title,
}

/// Changes the case of the selection, or of the word under the cursor when
/// nothing is selected. `None` when there's no word or nothing changes.
pub fn change_case(text: &str, selection: Range<usize>, case: Case) -> Option<LineEdit> {
    let range = if selection.is_empty() { word_at(text, selection.start) } else { selection.clone() };
    if range.is_empty() {
        return None;
    }
    let original = &text[range.clone()];
    let changed = match case {
        Case::Upper => original.to_uppercase(),
        Case::Lower => original.to_lowercase(),
        Case::Title => title_case(original),
    };
    if changed == original {
        return None;
    }
    let end = range.start + changed.len();
    let selection = if selection.is_empty() {
        selection.start.min(end)..selection.start.min(end)
    } else {
        range.start..end
    };
    Some(LineEdit { range, text: changed, selection })
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn word_at(text: &str, offset: usize) -> Range<usize> {
    let start = text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(offset, |(i, _)| i);
    let end = text[offset..].find(|c: char| !is_word_char(c)).map_or(text.len(), |i| offset + i);
    start..end
}

/// Upper-cases the first letter of each word and lower-cases the rest.
fn title_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_word_start = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if at_word_start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            at_word_start = false;
        } else {
            out.push(c);
            at_word_start = c != '\'';
        }
    }
    out
}

/// Cleanup for saving: strips spaces and tabs from line ends (unless
/// `trim_trailing` is off) and adds a final newline when the text doesn't end
/// with one. One edit spanning the first to the last change, so it's one undo
/// step. `None` when there's nothing to fix.
pub fn tidy_whitespace(text: &str, selection: Range<usize>, trim_trailing: bool) -> Option<LineEdit> {
    let mut changes = Vec::new();
    let mut start = 0;
    let mut last_kept = 0;
    for line in text.split('\n') {
        let kept = if trim_trailing { line.trim_end_matches([' ', '\t']).len() } else { line.len() };
        if kept < line.len() {
            changes.push(Change { at: start + kept, removed: line.len() - kept, inserted: 0 });
        }
        last_kept = kept;
        start += line.len() + 1;
    }
    // A last line that's empty (or only whitespace being trimmed) already
    // leaves the text ending in a newline.
    if last_kept > 0 {
        changes.push(Change { at: text.len(), removed: 0, inserted: 1 });
    }

    let range = changes.first()?.at..changes.last().map(|change| change.at + change.removed)?;
    let mut replacement = String::new();
    let mut pos = range.start;
    for change in &changes {
        replacement.push_str(&text[pos..change.at]);
        if change.inserted > 0 {
            replacement.push('\n');
        }
        pos = change.at + change.removed;
    }
    Some(LineEdit {
        range,
        text: replacement,
        selection: map_offset(selection.start, &changes, true)..map_offset(selection.end, &changes, true),
    })
}

fn is_commented(line: &str, comment: Comment) -> bool {
    let body = line.trim();
    match comment {
        Comment::Line(prefix) => body.starts_with(prefix),
        Comment::Block(open, close) => {
            body.len() >= open.len() + close.len() && body.starts_with(open) && body.ends_with(close)
        }
    }
}

/// Comments out the selected lines, or uncomments them when every non-blank
/// one is already commented. Blank lines are left alone. `None` when there's
/// nothing but blank lines.
pub fn toggle_comment(text: &str, selection: Range<usize>, comment: Comment) -> Option<LineEdit> {
    let block = line_block(text, &selection);
    let lines: Vec<&str> = text[block.clone()].split('\n').collect();
    let blank = |line: &&str| line.trim().is_empty();
    let filled: Vec<&&str> = lines.iter().filter(|line| !blank(line)).collect();
    if filled.is_empty() {
        return None;
    }
    let uncomment = filled.iter().all(|line| is_commented(line, comment));
    let indent = filled
        .iter()
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let mut out = String::with_capacity(block.len() + lines.len() * 4);
    let mut changes = Vec::new();
    let mut pos = block.start;
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push('\n');
            pos += 1;
        }
        if blank(line) {
            out.push_str(line);
        } else if uncomment {
            uncomment_line(line, pos, comment, &mut out, &mut changes);
        } else {
            comment_line(line, pos, indent, comment, &mut out, &mut changes);
        }
        pos += line.len();
    }

    let new_selection = if selection.is_empty() {
        let cursor = map_offset(selection.start, &changes, false);
        cursor..cursor
    } else {
        map_offset(selection.start, &changes, true)..map_offset(selection.end, &changes, false)
    };
    Some(LineEdit {
        range: block,
        text: out,
        selection: new_selection,
    })
}

fn comment_line(line: &str, pos: usize, indent: usize, comment: Comment, out: &mut String, changes: &mut Vec<Change>) {
    out.push_str(&line[..indent]);
    match comment {
        Comment::Line(prefix) => {
            let marker = format!("{prefix} ");
            out.push_str(&marker);
            out.push_str(&line[indent..]);
            changes.push(Change { at: pos + indent, removed: 0, inserted: marker.len() });
        }
        Comment::Block(open, close) => {
            let content_end = line.trim_end().len();
            let (open, close) = (format!("{open} "), format!(" {close}"));
            out.push_str(&open);
            out.push_str(&line[indent..content_end]);
            out.push_str(&close);
            out.push_str(&line[content_end..]);
            changes.push(Change { at: pos + indent, removed: 0, inserted: open.len() });
            changes.push(Change { at: pos + content_end, removed: 0, inserted: close.len() });
        }
    }
}

fn uncomment_line(line: &str, pos: usize, comment: Comment, out: &mut String, changes: &mut Vec<Change>) {
    let lead = line.len() - line.trim_start().len();
    let body = &line[lead..];
    out.push_str(&line[..lead]);
    match comment {
        Comment::Line(prefix) => {
            let mut cut = prefix.len();
            if body[cut..].starts_with(' ') {
                cut += 1;
            }
            out.push_str(&body[cut..]);
            changes.push(Change { at: pos + lead, removed: cut, inserted: 0 });
        }
        Comment::Block(open, close) => {
            let content = body.trim_end();
            let mut open_cut = open.len();
            if content[open_cut..].starts_with(' ') && open_cut < content.len() - close.len() {
                open_cut += 1;
            }
            let mut close_at = content.len() - close.len();
            if close_at > open_cut && content[..close_at].ends_with(' ') {
                close_at -= 1;
            }
            out.push_str(&body[open_cut..close_at]);
            out.push_str(&body[content.len()..]);
            changes.push(Change { at: pos + lead, removed: open_cut, inserted: 0 });
            changes.push(Change { at: pos + lead + close_at, removed: content.len() - close_at, inserted: 0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies an edit and returns the new text with the selection marked as `[..]`.
    fn apply(text: &str, edit: LineEdit) -> String {
        let mut result = text.to_string();
        result.replace_range(edit.range, &edit.text);
        let Range { start, end } = edit.selection;
        format!("{}[{}]{}", &result[..start], &result[start..end], &result[end..])
    }

    #[test]
    fn sorts_the_selected_lines_ignoring_case() {
        let text = "keep\npear\nApple\nbanana\nlast";
        // "pear" through "banana" selected.
        assert_eq!(apply(text, sort_lines(text, 5..22).unwrap()), "keep\n[Apple\nbanana\npear]\nlast");
    }

    #[test]
    fn sorts_the_whole_document_with_nothing_selected() {
        let text = "c\na\nb\n";
        assert_eq!(apply(text, sort_lines(text, 2..2).unwrap()), "a\n[]b\nc\n", "the cursor offset stays");
        assert!(sort_lines("a\nb\n", 0..0).is_none(), "already sorted");
    }

    #[test]
    fn changes_the_case_of_the_selection() {
        let text = "hello wide world";
        assert_eq!(apply(text, change_case(text, 6..16, Case::Upper).unwrap()), "hello [WIDE WORLD]");
        assert_eq!(apply(text, change_case(text, 0..16, Case::Title).unwrap()), "[Hello Wide World]");
        let shout = "LOUD";
        assert_eq!(apply(shout, change_case(shout, 0..4, Case::Lower).unwrap()), "[loud]");
    }

    #[test]
    fn changes_the_word_under_the_cursor_with_nothing_selected() {
        let text = "let some_name = 1;";
        assert_eq!(apply(text, change_case(text, 7..7, Case::Upper).unwrap()), "let SOM[]E_NAME = 1;");
        assert_eq!(apply(text, change_case(text, 13..13, Case::Upper).unwrap()), "let SOME_NAME[] = 1;", "touching the word's end counts");
        assert!(change_case(text, 15..15, Case::Upper).is_none(), "no word between `=` and the space");
    }

    #[test]
    fn title_case_handles_apostrophes_and_mixed_case() {
        assert_eq!(title_case("don't STOP me-now"), "Don't Stop Me-Now");
    }

    #[test]
    fn tidy_trims_line_ends_and_adds_a_final_newline() {
        let text = "a  \nb\t\nc";
        assert_eq!(apply(text, tidy_whitespace(text, 1..1, true).unwrap()), "a[]\nb\nc\n");
        // A cursor past the trimmed spaces lands at the new line end.
        assert_eq!(apply(text, tidy_whitespace(text, 3..3, true).unwrap()), "a[]\nb\nc\n");
        assert_eq!(apply(text, tidy_whitespace(text, 6..6, true).unwrap()), "a\nb[]\nc\n");
        assert_eq!(apply(text, tidy_whitespace(text, 7..7, true).unwrap()), "a\nb\n[]c\n");
    }

    #[test]
    fn tidy_leaves_clean_text_alone() {
        assert!(tidy_whitespace("a\nb\n", 0..0, true).is_none());
        assert!(tidy_whitespace("", 0..0, true).is_none());
        assert!(tidy_whitespace("a\n\n", 0..0, true).is_none(), "extra blank lines are kept");
    }

    #[test]
    fn tidy_without_trimming_only_adds_the_newline() {
        let text = "line one  \nline two";
        assert_eq!(apply(text, tidy_whitespace(text, 0..0, false).unwrap()), "[]line one  \nline two\n");
    }

    #[test]
    fn tidy_doesnt_double_the_newline_after_a_whitespace_line() {
        let text = "a\n   ";
        assert_eq!(apply(text, tidy_whitespace(text, 0..0, true).unwrap()), "[]a\n");
    }

    #[test]
    fn duplicates_the_cursor_line_and_follows_it() {
        let text = "one\ntwo\nthree";
        assert_eq!(apply(text, duplicate(text, 5..5)), "one\ntwo\nt[]wo\nthree");
        assert_eq!(apply(text, duplicate(text, 10..10)), "one\ntwo\nthree\nth[]ree");
    }

    #[test]
    fn duplicates_every_selected_line() {
        let text = "a\nb\nc\n";
        // "a\nb\n" selected from the start of a to the start of c.
        assert_eq!(apply(text, duplicate(text, 0..4)), "a\nb\n[a\nb\n]c\n");
    }

    #[test]
    fn moves_lines_up_and_down() {
        let text = "a\nb\nc";
        assert_eq!(apply(text, move_up(text, 2..2).unwrap()), "[]b\na\nc");
        assert_eq!(apply(text, move_down(text, 2..2).unwrap()), "a\nc\n[]b");
        assert_eq!(move_up(text, 0..0), None);
        assert_eq!(move_down(text, 4..4), None);
    }

    #[test]
    fn moves_a_selected_block_past_its_neighbor() {
        let text = "a\nb\nc\nd\n";
        assert_eq!(apply(text, move_down(text, 2..6).unwrap()), "a\nd\n[b\nc\n]");
        assert_eq!(apply(text, move_up(text, 2..6).unwrap()), "[b\nc\n]a\nd\n");
    }

    #[test]
    fn comments_at_the_shallowest_indent_and_skips_blank_lines() {
        let text = "fn a() {\n    if b {\n\n        c();\n    }\n}";
        let start = text.find("    if").unwrap();
        let end = text.find("    }").unwrap() + 5;
        let edit = toggle_comment(text, start..end, Comment::Line("//")).unwrap();
        assert_eq!(
            apply(text, edit),
            "fn a() {\n[    // if b {\n\n    //     c();\n    // }]\n}"
        );
    }

    #[test]
    fn toggling_twice_restores_the_text() {
        let text = "x = 1\n  y = 2\n";
        let commented = toggle_comment(text, 0..13, Comment::Line("#")).unwrap();
        let mut once = text.to_string();
        once.replace_range(commented.range.clone(), &commented.text);
        assert_eq!(once, "# x = 1\n#   y = 2\n");

        let back = toggle_comment(&once, commented.selection, Comment::Line("#")).unwrap();
        let mut twice = once.clone();
        twice.replace_range(back.range, &back.text);
        assert_eq!(twice, text);
    }

    #[test]
    fn mixed_lines_get_commented_not_uncommented() {
        let text = "// a\nb";
        let edit = toggle_comment(text, 0..6, Comment::Line("//")).unwrap();
        assert_eq!(edit.text, "// // a\n// b");
    }

    #[test]
    fn the_cursor_keeps_its_place_in_the_line() {
        let text = "let x = 1;";
        let edit = toggle_comment(text, 4..4, Comment::Line("//")).unwrap();
        assert_eq!(apply(text, edit.clone()), "// let []x = 1;");
        let mut commented = text.to_string();
        commented.replace_range(edit.range.clone(), &edit.text);
        let back = toggle_comment(&commented, edit.selection, Comment::Line("//")).unwrap();
        assert_eq!(apply(&commented, back), "let []x = 1;");
    }

    #[test]
    fn block_comments_wrap_each_line() {
        let text = "<p>hi</p>\n  <b>yo</b>";
        let edit = toggle_comment(text, 0..text.len(), Comment::Block("<!--", "-->")).unwrap();
        assert_eq!(edit.text, "<!-- <p>hi</p> -->\n<!--   <b>yo</b> -->");

        let back = toggle_comment(&edit.text, 0..edit.text.len(), Comment::Block("<!--", "-->")).unwrap();
        assert_eq!(back.text, text);
    }

    #[test]
    fn only_blank_lines_means_nothing_to_do() {
        assert_eq!(toggle_comment("\n  \n", 0..3, Comment::Line("#")), None);
    }
}
