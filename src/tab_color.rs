use gpui_kit::Hsla;
use gpui_kit::component::{Colorize as _, Theme};
use serde::{Deserialize, Serialize};

/// A tab's accent color. Presets follow the active theme's palette; custom
/// colors are stored as hex and stay fixed across themes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabColor {
    Red,
    Yellow,
    Green,
    Teal,
    Blue,
    Purple,
    Custom(String),
}

/// Syntax colors that tabs cycle through in Theme mode, in this order.
const CYCLE_SYNTAX: [&str; 10] = [
    "keyword",
    "string",
    "function",
    "type",
    "constant",
    "attribute",
    "tag",
    "number",
    "title",
    "constructor",
];

/// The theme's distinct syntax colors, for coloring tabs by position. Colors
/// that repeat or match the text color are skipped; a theme with fewer than
/// three left uses the preset palette instead.
pub fn theme_cycle(theme: &Theme) -> Vec<Hsla> {
    let mut colors: Vec<Hsla> = Vec::new();
    for name in CYCLE_SYNTAX {
        let Some(color) = theme.highlight_theme.style(name).and_then(|style| style.color) else {
            continue;
        };
        if !looks_same(color, theme.foreground) && !colors.iter().any(|seen| looks_same(*seen, color)) {
            colors.push(color);
        }
    }
    if colors.len() < 3 {
        colors = TabColor::PRESETS.iter().map(|preset| preset.resolve(theme)).collect();
    }
    colors
}

fn looks_same(a: Hsla, b: Hsla) -> bool {
    let hue = (a.h - b.h).abs();
    hue.min(1. - hue) < 0.025 && (a.s - b.s).abs() < 0.1 && (a.l - b.l).abs() < 0.08
}

impl TabColor {
    pub const PRESETS: [TabColor; 6] = [
        TabColor::Red,
        TabColor::Yellow,
        TabColor::Green,
        TabColor::Teal,
        TabColor::Blue,
        TabColor::Purple,
    ];

    pub fn label(&self) -> &str {
        match self {
            TabColor::Red => "Red",
            TabColor::Yellow => "Yellow",
            TabColor::Green => "Green",
            TabColor::Teal => "Teal",
            TabColor::Blue => "Blue",
            TabColor::Purple => "Purple",
            TabColor::Custom(_) => "Custom",
        }
    }

    pub fn custom(color: Hsla) -> Self {
        TabColor::Custom(color.to_hex())
    }

    pub fn resolve(&self, theme: &Theme) -> Hsla {
        match self {
            TabColor::Red => theme.red,
            TabColor::Yellow => theme.yellow,
            TabColor::Green => theme.green,
            TabColor::Teal => theme.cyan,
            TabColor::Blue => theme.blue,
            TabColor::Purple => theme.magenta,
            TabColor::Custom(hex) => Hsla::parse_hex(hex).unwrap_or(theme.muted_foreground),
        }
    }

    /// The automatic color for a language id, used in "color by language" mode.
    pub fn for_language(id: &str) -> Option<TabColor> {
        Some(match id {
            "rust" | "ruby" | "java" | "html" => TabColor::Red,
            "javascript" | "json" | "python" | "lua" => TabColor::Yellow,
            "markdown" | "bash" | "make" | "diff" => TabColor::Green,
            "go" | "css" | "toml" | "yaml" => TabColor::Teal,
            "typescript" | "tsx" | "c" | "cpp" | "sql" => TabColor::Blue,
            "php" | "zig" => TabColor::Purple,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_presets_and_custom_colors_readably() {
        assert_eq!(serde_json::to_string(&TabColor::Teal).unwrap(), r#""teal""#);
        assert_eq!(
            serde_json::to_string(&TabColor::Custom("#ff8800".into())).unwrap(),
            r##"{"custom":"#ff8800"}"##
        );
        let back: TabColor = serde_json::from_str(r##"{"custom":"#ff8800"}"##).unwrap();
        assert_eq!(back, TabColor::Custom("#ff8800".into()));
    }

    #[test]
    fn near_duplicate_colors_count_as_the_same() {
        let orange = Hsla::parse_hex("#E19773").unwrap();
        assert!(looks_same(orange, Hsla::parse_hex("#E29874").unwrap()));
        assert!(!looks_same(orange, Hsla::parse_hex("#76BA53").unwrap()));
        // Hue wraps around: deep red at both ends of the circle.
        assert!(looks_same(Hsla { h: 0.995, s: 0.7, l: 0.5, a: 1. }, Hsla { h: 0.005, s: 0.7, l: 0.5, a: 1. }));
    }

    #[test]
    fn plain_text_has_no_language_color() {
        assert_eq!(TabColor::for_language("rust"), Some(TabColor::Red));
        assert_eq!(TabColor::for_language("text"), None);
    }
}
