//! Find in Files: a plain-text search through the files Quick Open lists.

use std::path::Path;

/// Files bigger than this are skipped, so a stray log or data dump can't stall the search.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Characters of each matching line kept for the result row.
const PREVIEW_CHARS: usize = 200;

/// One matching line. `line` and `column` start at 0, and `column` counts
/// characters, the way Kit's `Position` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub preview: String,
}

/// The first match on each line of `files` (paths relative to `root`) for
/// `query`, ignoring ASCII case. Binary, non-UTF-8, and very large files are
/// skipped. Stops after `limit` matches, or as soon as `cancelled` says so.
pub fn search(root: &Path, files: &[String], query: &str, limit: usize, cancelled: impl Fn() -> bool) -> Vec<Match> {
    let mut matches = Vec::new();
    if query.is_empty() {
        return matches;
    }
    let needle = query.to_ascii_lowercase();
    for file in files {
        if cancelled() || matches.len() >= limit {
            break;
        }
        let path = root.join(file);
        if std::fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_FILE_BYTES) {
            continue;
        }
        let Ok(text) = crate::document::read_text(&path) else {
            continue;
        };
        for (line_number, line) in text.lines().enumerate() {
            let Some(byte) = line.to_ascii_lowercase().find(&needle) else {
                continue;
            };
            matches.push(Match {
                file: file.clone(),
                line: line_number as u32,
                column: line[..byte].chars().count() as u32,
                preview: line.trim().chars().take(PREVIEW_CHARS).collect(),
            });
            if matches.len() >= limit {
                break;
            }
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::{Match, search};

    fn dir_with(name: &str, files: &[(&str, &[u8])]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-file-search-{name}-{}", std::process::id()));
        _ = std::fs::remove_dir_all(&dir);
        for (file, bytes) in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        dir
    }

    fn names(files: &[(&str, &[u8])]) -> Vec<String> {
        files.iter().map(|(file, _)| file.to_string()).collect()
    }

    #[test]
    fn finds_lines_across_files_ignoring_case() {
        let files: &[(&str, &[u8])] = &[("a.txt", b"nothing\n  let Counter = 1;\n"), ("src/b.rs", b"counter()\n")];
        let dir = dir_with("case", files);
        let found = search(&dir, &names(files), "counter", 100, || false);
        assert_eq!(
            found,
            vec![
                Match { file: "a.txt".into(), line: 1, column: 6, preview: "let Counter = 1;".into() },
                Match { file: "src/b.rs".into(), line: 0, column: 0, preview: "counter()".into() },
            ]
        );
    }

    #[test]
    fn counts_columns_in_characters() {
        let files: &[(&str, &[u8])] = &[("é.txt", "café needle".as_bytes())];
        let dir = dir_with("chars", files);
        assert_eq!(search(&dir, &names(files), "needle", 100, || false)[0].column, 5);
    }

    #[test]
    fn skips_binary_files_and_stops_at_the_limit() {
        let files: &[(&str, &[u8])] = &[("bin", b"hit\0hit"), ("many.txt", b"hit\nhit\nhit\n")];
        let dir = dir_with("limit", files);
        let found = search(&dir, &names(files), "hit", 2, || false);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|found| found.file == "many.txt"));
        assert!(search(&dir, &names(files), "hit", 100, || true).is_empty(), "cancelled before the first file");
    }
}
