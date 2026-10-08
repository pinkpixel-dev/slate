//! A line diff in unified format, for Compare with Disk.

/// Lines of unchanged text kept around each change.
const CONTEXT: usize = 3;
/// Past this many cells (old lines × new lines after trimming), skip the
/// line matching and show the changed block as removed then added.
const MAX_CELLS: usize = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Same,
    Removed,
    Added,
}

/// `old` turned into `new` as a unified diff, or `None` when they're the same.
pub fn unified(old: &str, new: &str, old_label: &str, new_label: &str) -> Option<String> {
    if old == new {
        return None;
    }
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let ops = diff_lines(&a, &b);

    let mut out = format!("--- {old_label}\n+++ {new_label}\n");
    let changed: Vec<usize> = (0..ops.len()).filter(|&i| ops[i] != Op::Same).collect();
    if changed.is_empty() {
        // Same lines, different line breaks (usually the final newline).
        out.push_str("@@ Only the final newline differs @@\n");
        return Some(out);
    }

    // Group changes whose context would touch into one hunk.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for &i in &changed {
        match groups.last_mut() {
            Some((_, end)) if i <= *end + 2 * CONTEXT + 1 => *end = i,
            _ => groups.push((i, i)),
        }
    }

    // Line numbers on each side before every op.
    let mut positions = Vec::with_capacity(ops.len() + 1);
    let (mut old_line, mut new_line) = (0, 0);
    for op in &ops {
        positions.push((old_line, new_line));
        match op {
            Op::Same => {
                old_line += 1;
                new_line += 1;
            }
            Op::Removed => old_line += 1,
            Op::Added => new_line += 1,
        }
    }
    positions.push((old_line, new_line));

    for (first, last) in groups {
        let start = first.saturating_sub(CONTEXT);
        let end = (last + CONTEXT + 1).min(ops.len());
        let (old_start, new_start) = positions[start];
        let (old_end, new_end) = positions[end];
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(old_start, old_end - old_start),
            range(new_start, new_end - new_start)
        ));
        for i in start..end {
            let (o, n) = positions[i];
            match ops[i] {
                Op::Same => push_line(&mut out, ' ', a[o]),
                Op::Removed => push_line(&mut out, '-', a[o]),
                Op::Added => push_line(&mut out, '+', b[n]),
            }
        }
    }
    Some(out)
}

/// `start,count` the way unified diffs write it: 1-based, and an empty range
/// names the line before it.
fn range(start: usize, count: usize) -> String {
    let first = if count == 0 { start } else { start + 1 };
    format!("{first},{count}")
}

fn push_line(out: &mut String, prefix: char, line: &str) {
    out.push(prefix);
    out.push_str(line);
    out.push('\n');
}

/// The edit script from `a` to `b`, removals before additions in each change.
fn diff_lines(a: &[&str], b: &[&str]) -> Vec<Op> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (mid_a, mid_b) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);

    let mut ops = vec![Op::Same; prefix];
    if mid_a.len().saturating_mul(mid_b.len()) > MAX_CELLS {
        ops.extend(std::iter::repeat_n(Op::Removed, mid_a.len()));
        ops.extend(std::iter::repeat_n(Op::Added, mid_b.len()));
    } else {
        ops.extend(lcs_ops(mid_a, mid_b));
    }
    ops.extend(std::iter::repeat_n(Op::Same, suffix));
    ops
}

/// Longest-common-subsequence diff. Fine for the middle part left after
/// trimming, which is small for ordinary edits.
fn lcs_ops(a: &[&str], b: &[&str]) -> Vec<Op> {
    let width = b.len() + 1;
    // table[i][j]: common lines between a[i..] and b[j..].
    let mut table = vec![0u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let (mut i, mut j) = (0, 0);
    let mut ops = Vec::with_capacity(a.len() + b.len());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            ops.push(Op::Same);
            i += 1;
            j += 1;
        } else if j == b.len() || (i < a.len() && table[(i + 1) * width + j] >= table[i * width + j + 1]) {
            ops.push(Op::Removed);
            i += 1;
        } else {
            ops.push(Op::Added);
            j += 1;
        }
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_text_has_no_diff() {
        assert_eq!(unified("a\nb\n", "a\nb\n", "old", "new"), None);
    }

    #[test]
    fn a_changed_line_gets_context_and_a_header() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
        let new = "1\n2\n3\n4\nfive\n6\n7\n8\n9\n";
        let diff = unified(old, new, "disk", "mine").unwrap();
        assert_eq!(
            diff,
            "--- disk\n+++ mine\n@@ -2,7 +2,7 @@\n 2\n 3\n 4\n-5\n+five\n 6\n 7\n 8\n"
        );
    }

    #[test]
    fn far_apart_changes_get_separate_hunks() {
        let old: String = (1..=30).map(|n| format!("{n}\n")).collect();
        let new: String = (1..=30)
            .filter(|&n| n != 28)
            .map(|n| if n == 3 { "three\n".to_string() } else { format!("{n}\n") })
            .collect();
        let diff = unified(&old, &new, "a", "b").unwrap();
        assert_eq!(diff.matches("@@ -").count(), 2);
        assert!(diff.contains("@@ -1,6 +1,6 @@\n 1\n 2\n-3\n+three\n"));
        assert!(diff.contains("@@ -25,6 +25,5 @@\n 25\n 26\n 27\n-28\n 29\n 30\n"));
    }

    #[test]
    fn insertions_into_empty_and_only_newline_changes() {
        assert_eq!(unified("", "hi\n", "a", "b").unwrap(), "--- a\n+++ b\n@@ -0,0 +1,1 @@\n+hi\n");
        assert!(unified("x", "x\n", "a", "b").unwrap().contains("final newline"));
    }

    #[test]
    fn lcs_keeps_moved_blocks_minimal() {
        let ops = diff_lines(&["a", "b", "c", "d"], &["a", "c", "d", "b"]);
        let changes = ops.iter().filter(|op| **op != Op::Same).count();
        assert_eq!(changes, 2);
    }
}
