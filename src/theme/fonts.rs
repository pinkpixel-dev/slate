use std::collections::HashSet;
use std::process::Command;

use gpui_kit::*;

/// Installed font families for the font pickers.
#[derive(Clone)]
pub struct FontLists {
    pub all: Vec<SharedString>,
    /// Monospace families, for the editor font. Falls back to `all` when
    /// fontconfig can't be asked.
    pub mono: Vec<SharedString>,
}

impl FontLists {
    pub fn installed(cx: &App) -> Self {
        let mut all: Vec<SharedString> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .filter(|name| !name.starts_with('.'))
            .map(SharedString::from)
            .collect();
        all.sort_by_key(|name| name.to_lowercase());
        all.dedup();

        let mono = match monospace_families() {
            Some(families) => all.iter().filter(|name| families.contains(name.as_ref())).cloned().collect(),
            None => Vec::new(),
        };
        let mono = if mono.is_empty() { all.clone() } else { mono };
        Self { all, mono }
    }
}

/// Asks fontconfig which families are fixed-width (spacing 90 "dual" and up).
fn monospace_families() -> Option<HashSet<String>> {
    let output = Command::new("fc-list")
        .arg("--format=%{spacing}\t%{family}\n")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_fc_list(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_fc_list(output: &str) -> HashSet<String> {
    output
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(spacing, _)| spacing.parse::<u32>().is_ok_and(|spacing| spacing >= 90))
        .flat_map(|(_, families)| families.split(','))
        .map(|family| family.trim().replace('\\', ""))
        .filter(|family| !family.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_fc_list;

    #[test]
    fn fc_list_output_keeps_only_fixed_width_families() {
        let output = "100\tJetBrains Mono,JetBrains Mono NL\n\tInter\n90\tNoto Sans Mono CJK SC\n0\tDejaVu Sans\n";
        let families = parse_fc_list(output);
        assert!(families.contains("JetBrains Mono"));
        assert!(families.contains("JetBrains Mono NL"));
        assert!(families.contains("Noto Sans Mono CJK SC"));
        assert!(!families.contains("Inter"));
        assert!(!families.contains("DejaVu Sans"));
    }
}
