use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// How many lines to look at when guessing a file's indentation.
const INDENT_SAMPLE_LINES: usize = 2000;
pub const DEFAULT_INDENT_WIDTH: usize = 4;

/// The line ending a file is saved with. The editor always holds `\n`; Save
/// converts back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

impl LineEnding {
    /// CRLF when at least half of the file's line breaks are `\r\n`.
    pub fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let lf = text.matches('\n').count() - crlf;
        if crlf > 0 && crlf >= lf { Self::Crlf } else { Self::Lf }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
        }
    }

    /// Turns editor text (`\n` only) into what goes on disk.
    pub fn apply(self, text: &str) -> Cow<'_, str> {
        match self {
            Self::Lf => Cow::Borrowed(text),
            Self::Crlf => Cow::Owned(text.replace('\n', "\r\n")),
        }
    }
}

/// Turns every `\r\n` into `\n`, so the editor only ever sees one kind of break.
pub fn normalize(text: String) -> String {
    if text.contains("\r\n") { text.replace("\r\n", "\n") } else { text }
}

/// What the Tab key inserts: a tab character, or `width` spaces. A tab also
/// draws `width` columns wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indent {
    pub hard_tabs: bool,
    pub width: usize,
}

impl Default for Indent {
    fn default() -> Self {
        Self {
            hard_tabs: false,
            width: DEFAULT_INDENT_WIDTH,
        }
    }
}

impl Indent {
    pub fn label(self) -> String {
        if self.hard_tabs {
            format!("Tabs: {}", self.width)
        } else {
            format!("Spaces: {}", self.width)
        }
    }

    /// Guesses from the leading whitespace of the first lines. `None` when
    /// nothing is indented, so the default stays.
    pub fn detect(text: &str) -> Option<Self> {
        let mut tab_lines = 0;
        let mut space_lines = 0;
        // How often each step up in space indentation shows up, for widths 1..=8.
        let mut steps = [0usize; 9];
        let mut previous = 0;

        for line in text.lines().take(INDENT_SAMPLE_LINES) {
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with('\t') {
                tab_lines += 1;
                continue;
            }
            let spaces = line.len() - line.trim_start_matches(' ').len();
            if spaces > 0 {
                space_lines += 1;
            }
            if spaces > previous && spaces - previous < steps.len() {
                steps[spaces - previous] += 1;
            }
            previous = spaces;
        }

        if tab_lines == 0 && space_lines == 0 {
            return None;
        }
        if tab_lines > space_lines {
            return Some(Self {
                hard_tabs: true,
                width: DEFAULT_INDENT_WIDTH,
            });
        }
        // One-space steps are mostly the ` * ` of block comments, so they only
        // count when nothing else shows up.
        let width = (2..steps.len())
            .filter(|&width| steps[width] > 0)
            .max_by_key(|&width| (steps[width], std::cmp::Reverse(width)))
            .or_else(|| (steps[1] > 0).then_some(1))
            .unwrap_or(DEFAULT_INDENT_WIDTH);
        Some(Self { hard_tabs: false, width })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_endings_follow_the_majority() {
        assert_eq!(LineEnding::detect("a\nb\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\r\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("a\r\nb\nc\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("no breaks"), LineEnding::Lf);
    }

    #[test]
    fn crlf_round_trips_through_the_editor() {
        let disk = "one\r\ntwo\r\n";
        let editor = normalize(disk.to_string());
        assert_eq!(editor, "one\ntwo\n");
        assert_eq!(LineEnding::Crlf.apply(&editor), disk);
        assert_eq!(LineEnding::Lf.apply(&editor), editor);
    }

    #[test]
    fn detects_tabs_and_space_widths() {
        let tabs = "fn a() {\n\tlet x = 1;\n\tif x {\n\t\ty();\n\t}\n}\n";
        assert_eq!(Indent::detect(tabs), Some(Indent { hard_tabs: true, width: 4 }));

        let two = "a:\n  b:\n    c: 1\n  d: 2\n";
        assert_eq!(Indent::detect(two), Some(Indent { hard_tabs: false, width: 2 }));

        let four = "def a():\n    if b:\n        c()\n    return\n";
        assert_eq!(Indent::detect(four), Some(Indent { hard_tabs: false, width: 4 }));
    }

    #[test]
    fn block_comment_stars_dont_make_one_space_indents() {
        let text = "/**\n * Docs.\n */\nfn a() {\n    b();\n}\n";
        assert_eq!(Indent::detect(text), Some(Indent { hard_tabs: false, width: 4 }));
    }

    #[test]
    fn unindented_text_keeps_the_default() {
        assert_eq!(Indent::detect("just\nsome\nlines\n"), None);
        assert_eq!(Indent::detect(""), None);
    }
}
