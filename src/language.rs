use std::path::Path;

/// A language Slate knows how to label and, when its grammar is compiled in, highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// Name passed to the editor's highlighter, e.g. `"rust"`.
    pub id: &'static str,
    /// Human-readable name for the status bar.
    pub label: &'static str,
}

/// How a language comments out a line, for Toggle Comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comment {
    /// A prefix like `//` or `#`.
    Line(&'static str),
    /// Languages without line comments wrap each line, like `<!-- line -->`.
    Block(&'static str, &'static str),
}

impl Language {
    pub const PLAIN: Language = Language::new("text", "Plain Text");

    const fn new(id: &'static str, label: &'static str) -> Self {
        Self { id, label }
    }

    /// `None` for formats with no comments at all, like plain text and JSON.
    pub fn comment(&self) -> Option<Comment> {
        Some(match self.id {
            "bash" | "make" | "python" | "ruby" | "toml" | "yaml" => Comment::Line("#"),
            "c" | "cpp" | "go" | "java" | "javascript" | "php" | "rust" | "tsx" | "typescript" | "zig" => {
                Comment::Line("//")
            }
            "lua" | "sql" => Comment::Line("--"),
            "css" => Comment::Block("/*", "*/"),
            "html" | "markdown" => Comment::Block("<!--", "-->"),
            _ => return None,
        })
    }
}

const BASH: Language = Language::new("bash", "Shell");
const C: Language = Language::new("c", "C");
const CPP: Language = Language::new("cpp", "C++");
const CSS: Language = Language::new("css", "CSS");
const DIFF: Language = Language::new("diff", "Diff");
const GO: Language = Language::new("go", "Go");
const HTML: Language = Language::new("html", "HTML");
const JAVA: Language = Language::new("java", "Java");
const JAVASCRIPT: Language = Language::new("javascript", "JavaScript");
const JSON: Language = Language::new("json", "JSON");
const LUA: Language = Language::new("lua", "Lua");
const MAKE: Language = Language::new("make", "Makefile");
const MARKDOWN: Language = Language::new("markdown", "Markdown");
const PHP: Language = Language::new("php", "PHP");
const PYTHON: Language = Language::new("python", "Python");
const RUBY: Language = Language::new("ruby", "Ruby");
const RUST: Language = Language::new("rust", "Rust");
const SQL: Language = Language::new("sql", "SQL");
const TOML: Language = Language::new("toml", "TOML");
const TSX: Language = Language::new("tsx", "TSX");
const TYPESCRIPT: Language = Language::new("typescript", "TypeScript");
const YAML: Language = Language::new("yaml", "YAML");
const ZIG: Language = Language::new("zig", "Zig");

/// Picks a language from a file's name, falling back to plain text.
pub fn detect(path: &Path) -> Language {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if let Some(language) = by_file_name(file_name) {
        return language;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| by_extension(&ext.to_ascii_lowercase()))
        .unwrap_or(Language::PLAIN)
}

fn by_file_name(name: &str) -> Option<Language> {
    Some(match name {
        "Makefile" | "makefile" | "GNUmakefile" => MAKE,
        ".bashrc" | ".bash_profile" | ".bash_logout" | ".profile" | ".zshrc" | ".zprofile"
        | ".zshenv" | "PKGBUILD" => BASH,
        "Cargo.lock" => TOML,
        "Gemfile" | "Rakefile" => RUBY,
        _ => return None,
    })
}

fn by_extension(ext: &str) -> Option<Language> {
    Some(match ext {
        "sh" | "bash" | "zsh" => BASH,
        "c" | "h" => C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => CPP,
        "css" | "scss" => CSS,
        "diff" | "patch" => DIFF,
        "go" => GO,
        "html" | "htm" => HTML,
        "java" => JAVA,
        "js" | "mjs" | "cjs" | "jsx" => JAVASCRIPT,
        "json" | "jsonc" => JSON,
        "lua" => LUA,
        "mk" => MAKE,
        "md" | "markdown" | "mdx" => MARKDOWN,
        "php" | "phtml" => PHP,
        "py" | "pyi" => PYTHON,
        "rb" => RUBY,
        "rs" => RUST,
        "sql" => SQL,
        "toml" => TOML,
        "tsx" => TSX,
        "ts" | "mts" | "cts" => TYPESCRIPT,
        "yaml" | "yml" => YAML,
        "zig" => ZIG,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_by_extension_case_insensitively() {
        assert_eq!(detect(Path::new("src/main.rs")), RUST);
        assert_eq!(detect(Path::new("README.MD")), MARKDOWN);
        assert_eq!(detect(Path::new("app.config.mjs")), JAVASCRIPT);
    }

    #[test]
    fn detects_special_file_names() {
        assert_eq!(detect(Path::new("/home/me/.bashrc")), BASH);
        assert_eq!(detect(Path::new("Makefile")), MAKE);
        assert_eq!(detect(Path::new("Cargo.lock")), TOML);
    }

    #[test]
    fn comment_styles() {
        assert_eq!(RUST.comment(), Some(Comment::Line("//")));
        assert_eq!(PYTHON.comment(), Some(Comment::Line("#")));
        assert_eq!(SQL.comment(), Some(Comment::Line("--")));
        assert_eq!(HTML.comment(), Some(Comment::Block("<!--", "-->")));
        assert_eq!(JSON.comment(), None);
        assert_eq!(Language::PLAIN.comment(), None);
    }

    #[test]
    fn falls_back_to_plain_text() {
        assert_eq!(detect(Path::new("notes")), Language::PLAIN);
        assert_eq!(detect(Path::new("photo.xyz")), Language::PLAIN);
    }
}
