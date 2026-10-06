//! The file list behind Quick Open: every file under a folder, and a fuzzy
//! matcher to rank them against what's typed.

use std::path::{Path, PathBuf};

use crate::file_tree::is_hidden_name;

/// Stop walking past this many files, so a huge folder can't stall Quick Open.
pub const MAX_FILES: usize = 50_000;

/// Files under `root` as `/`-separated paths relative to it, sorted. Hidden
/// files and folders are skipped unless `show_hidden`; `.git` always is.
/// Symlinked folders aren't followed, so a link loop can't trap the walk.
pub fn list_files(root: &Path, show_hidden: bool) -> Vec<String> {
    let mut files = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            if name == ".git" || (!show_hidden && is_hidden_name(&name, kind.is_dir())) {
                continue;
            }
            let path = relative.join(&name);
            if kind.is_dir() {
                pending.push(path);
            } else {
                files.push(path.to_string_lossy().into_owned());
                if files.len() >= MAX_FILES {
                    files.sort_unstable();
                    return files;
                }
            }
        }
    }
    files.sort_unstable();
    files
}

/// Scores `candidate` against `query`: every query character has to show up
/// in order (ignoring case). Higher is better. `None` when it doesn't match.
pub fn score(query: &str, candidate: &str) -> Option<i64> {
    let file_start = candidate.rfind('/').map_or(0, |i| i + 1);
    let lower: Vec<char> = candidate.chars().map(fold_case).collect();
    let chars: Vec<(usize, char)> = candidate.char_indices().collect();
    let mut total = 0i64;
    let mut index = 0;
    let mut previous: Option<usize> = None;
    for q in query.chars().map(fold_case) {
        if q.is_whitespace() {
            continue;
        }
        let found = (index..lower.len()).find(|&i| lower[i] == q)?;
        let byte = chars[found].0;
        let mut points = 1;
        if previous == Some(found.wrapping_sub(1)) {
            points += 6;
        }
        let at_word_start = found == 0 || matches!(chars[found - 1].1, '/' | '_' | '-' | '.' | ' ');
        if at_word_start {
            points += 4;
        }
        if byte >= file_start {
            points += 3;
        }
        total += points;
        previous = Some(found);
        index = found + 1;
    }
    // Shorter paths win ties.
    Some(total * 1000 - candidate.len() as i64)
}

/// One character in, one out, so positions in the folded text match the original.
fn fold_case(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// The best `limit` matches for `query`, best first. An empty query keeps the list's order.
pub fn rank<'a>(query: &str, files: &'a [String], limit: usize) -> Vec<&'a str> {
    if query.trim().is_empty() {
        return files.iter().take(limit).map(String::as_str).collect();
    }
    let mut scored: Vec<(i64, &str)> = files
        .iter()
        .filter_map(|file| score(query, file).map(|points| (points, file.as_str())))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored.into_iter().take(limit).map(|(_, file)| file).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn matches_in_order_ignoring_case() {
        assert!(score("wsmod", "src/workspace/mod.rs").is_some());
        assert!(score("WSMOD", "src/workspace/mod.rs").is_some());
        assert!(score("domws", "src/workspace/mod.rs").is_none());
    }

    #[test]
    fn file_names_and_word_starts_rank_first() {
        let list = files(&["src/main.rs", "docs/maintenance/notes.md", "src/domain.rs"]);
        assert_eq!(rank("main", &list, 10)[0], "src/main.rs");

        let list = files(&["src/workspace/tab_strip.rs", "src/tab_color.rs", "src/storage.rs"]);
        assert_eq!(rank("tabc", &list, 10)[0], "src/tab_color.rs");
    }

    #[test]
    fn empty_query_keeps_order_and_limit_caps_results() {
        let list = files(&["a", "b", "c"]);
        assert_eq!(rank("", &list, 2), vec!["a", "b"]);
        assert!(rank("zzz", &list, 10).is_empty());
    }

    #[test]
    fn walks_folders_skipping_hidden_and_git() {
        let root = std::env::temp_dir().join(format!("slate-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["src/nested", ".git", "node_modules/pkg", ".config"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in ["README.md", "src/main.rs", "src/nested/deep.rs", ".git/HEAD", "node_modules/pkg/index.js", ".config/x", ".env"] {
            std::fs::write(root.join(file), "").unwrap();
        }

        assert_eq!(list_files(&root, false), vec!["README.md", "src/main.rs", "src/nested/deep.rs"]);
        let all = list_files(&root, true);
        assert!(all.contains(&".env".to_string()) && all.contains(&"node_modules/pkg/index.js".to_string()));
        assert!(!all.iter().any(|file| file.starts_with(".git/")));
    }
}
