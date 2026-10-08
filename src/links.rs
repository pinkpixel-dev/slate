//! Finding the URL or file path under the mouse, for Ctrl+click.

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Something on a line that Ctrl+click can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Url(String),
    Path(String),
}

/// Characters that end a link: whitespace, quotes, and brackets.
fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | '|')
}

/// The link covering byte `column` of `line`, and its byte range in the line.
pub fn link_at(line: &str, column: usize) -> Option<(Range<usize>, Link)> {
    if column > line.len() || !line.is_char_boundary(column) {
        return None;
    }
    let start = line[..column].rfind(is_boundary).map_or(0, |i| i + line[i..].chars().next().map_or(1, char::len_utf8));
    let end = line[column..].find(is_boundary).map_or(line.len(), |i| column + i);
    let mut token = &line[start..end];

    // Sentence punctuation after a link isn't part of it.
    token = token.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    if token.is_empty() || column >= start + token.len() {
        return None;
    }

    // A URL can sit after other text in the token, like `href=https://...`.
    if let Some(at) = token.find("https://").or_else(|| token.find("http://")) {
        let url = &token[at..];
        let rest = url.split_once("://").map_or("", |(_, rest)| rest);
        if !rest.is_empty() && start + at <= column {
            return Some((start + at..start + token.len(), Link::Url(url.to_string())));
        }
        return None;
    }

    let looks_like_path = token.contains('/') || token.starts_with('~');
    if looks_like_path && !token.contains("://") {
        return Some((start..start + token.len(), Link::Path(token.to_string())));
    }
    None
}

/// The existing file or folder `path` points at. Relative paths are tried
/// against each of `bases` in order.
pub fn resolve(path: &str, bases: &[PathBuf]) -> Option<PathBuf> {
    let expanded = match path.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            let home = std::env::var_os("HOME").map(PathBuf::from)?;
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(path),
    };
    if expanded.is_absolute() {
        return expanded.exists().then_some(expanded);
    }
    bases
        .iter()
        .map(|base| base.join(&expanded))
        .find(|candidate| candidate.exists())
        .map(|candidate| normalize(&candidate))
}

/// Drops `.` and folds `..` so the tab shows a tidy path.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: &str, needle: &str) -> Option<(Range<usize>, Link)> {
        link_at(line, line.find(needle).unwrap())
    }

    #[test]
    fn finds_urls_without_trailing_punctuation_or_brackets() {
        let line = "See (https://pinkpixel.dev/docs?a=1). Thanks";
        let (range, link) = at(line, "pinkpixel").unwrap();
        assert_eq!(link, Link::Url("https://pinkpixel.dev/docs?a=1".into()));
        assert_eq!(&line[range], "https://pinkpixel.dev/docs?a=1");

        let line = r#"<a href="http://example.com/x">"#;
        assert_eq!(at(line, "example").unwrap().1, Link::Url("http://example.com/x".into()));
        assert_eq!(at("url=https://a.io/b, more", "a.io").unwrap().1, Link::Url("https://a.io/b".into()));
    }

    #[test]
    fn finds_paths_and_ignores_plain_words() {
        assert_eq!(at("open ./src/main.rs now", "main").unwrap().1, Link::Path("./src/main.rs".into()));
        assert_eq!(at("cfg in ~/.config/slate.", "config").unwrap().1, Link::Path("~/.config/slate".into()));
        assert_eq!(at("just words here", "words"), None);
        assert_eq!(at("https:// alone", "https"), None);
    }

    #[test]
    fn nothing_past_the_end_or_on_whitespace() {
        let line = "a /tmp/x b";
        assert_eq!(link_at(line, 1), None);
        assert_eq!(link_at(line, line.len() + 1), None);
        assert_eq!(link_at("é/x", 1), None, "not a char boundary");
    }

    #[test]
    fn resolves_relative_paths_against_each_base() {
        let root = std::env::temp_dir().join(format!("slate-links-{}", std::process::id()));
        std::fs::create_dir_all(root.join("one")).unwrap();
        std::fs::create_dir_all(root.join("two/sub")).unwrap();
        std::fs::write(root.join("two/sub/file.txt"), "").unwrap();

        let bases = [root.join("one"), root.join("two")];
        assert_eq!(resolve("sub/file.txt", &bases), Some(root.join("two/sub/file.txt")));
        assert_eq!(resolve("./sub/../sub/file.txt", &bases), Some(root.join("two/sub/file.txt")));
        assert_eq!(resolve("missing/file.txt", &bases), None);
        let absolute = root.join("two/sub/file.txt");
        assert_eq!(resolve(absolute.to_str().unwrap(), &[]), Some(absolute));
    }
}
