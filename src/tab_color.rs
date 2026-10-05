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
    fn plain_text_has_no_language_color() {
        assert_eq!(TabColor::for_language("rust"), Some(TabColor::Red));
        assert_eq!(TabColor::for_language("text"), None);
    }
}
