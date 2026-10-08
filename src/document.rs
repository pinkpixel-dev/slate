use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::language::{self, Language};
use crate::text_format::{self, LineEnding};

/// The file behind the editor: where it lives, what language it is, and whether
/// it has unsaved edits.
pub struct Document {
    path: Option<PathBuf>,
    /// Which "Untitled N" this is while it has no path.
    untitled_number: usize,
    language: Language,
    /// Bumped on every edit.
    revision: u64,
    /// The revision that was last written to (or read from) disk.
    saved_revision: u64,
    /// The file's modified time when it was last read or saved.
    disk_modified: Option<SystemTime>,
    /// The line ending Save writes. The editor itself only holds `\n`.
    line_ending: LineEnding,
    /// Set for read-only views like Compare with Disk, which are never saved in the session.
    title: Option<String>,
}

impl Document {
    pub fn untitled(number: usize) -> Self {
        Self {
            path: None,
            untitled_number: number,
            language: Language::PLAIN,
            revision: 0,
            saved_revision: 0,
            disk_modified: None,
            line_ending: LineEnding::default(),
            title: None,
        }
    }

    /// A view with its own tab name, highlighted as the language of `kind_path`.
    pub fn scratch(title: String, kind_path: &Path) -> Self {
        Self {
            language: language::detect(kind_path),
            title: Some(title),
            ..Self::untitled(0)
        }
    }

    pub fn is_scratch(&self) -> bool {
        self.title.is_some()
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self {
            language: language::detect(&path),
            path: Some(path),
            untitled_number: 0,
            revision: 0,
            saved_revision: 0,
            disk_modified: None,
            line_ending: LineEnding::default(),
            title: None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn display_name(&self) -> String {
        if let (None, Some(title)) = (&self.path, &self.title) {
            return title.clone();
        }
        self.path
            .as_deref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| match self.untitled_number {
                0 | 1 => "Untitled".into(),
                n => format!("Untitled {n}"),
            })
    }

    /// `Some(n)` for an "Untitled n" document that has never been saved.
    pub fn untitled_number(&self) -> Option<usize> {
        (self.path.is_none() && self.title.is_none()).then_some(self.untitled_number)
    }

    /// Folder to start a Save As dialog in.
    pub fn directory(&self) -> PathBuf {
        self.path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn disk_modified(&self) -> Option<SystemTime> {
        self.disk_modified
    }

    pub fn set_disk_modified(&mut self, modified: Option<SystemTime>) {
        self.disk_modified = modified;
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn set_line_ending(&mut self, line_ending: LineEnding) {
        self.line_ending = line_ending;
    }

    pub fn mark_edited(&mut self) {
        self.revision += 1;
    }

    /// Records that `revision` was written to `path`. Edits made while the write
    /// was in flight keep the document dirty.
    pub fn mark_saved(&mut self, path: PathBuf, revision: u64) {
        if self.path.as_deref() != Some(path.as_path()) {
            self.language = language::detect(&path);
            self.path = Some(path);
            self.title = None;
        }
        self.saved_revision = revision;
    }
}

/// Reads a file as UTF-8 text. Binary files and other encodings are refused
/// rather than loaded lossily, so saving can't silently corrupt them.
pub fn read_text(path: &Path) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    if bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "looks like a binary file",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "isn't valid UTF-8 text"))
}

/// A file as read from disk, ready for the editor.
pub struct Loaded {
    /// The text with `\r\n` turned into `\n`.
    pub text: String,
    pub line_ending: LineEnding,
    pub modified: Option<SystemTime>,
}

/// Reads a file for the editor: checks it's text, notes its line ending, and normalizes it.
pub fn load(path: &Path) -> io::Result<Loaded> {
    let text = read_text(path)?;
    Ok(Loaded {
        line_ending: LineEnding::detect(&text),
        text: text_format::normalize(text),
        modified: modified(path),
    })
}

/// A file's modified time, or `None` if it's missing or the filesystem doesn't say.
pub fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

/// Writes editor text with the document's line ending.
pub fn write_text(path: &Path, text: &str, line_ending: LineEnding) -> io::Result<()> {
    std::fs::write(path, line_ending.apply(text).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn reads_utf8_and_refuses_binary_or_other_encodings() {
        let text = temp_file("ok.txt", "héllo\r\nworld\n".as_bytes());
        assert_eq!(read_text(&text).unwrap(), "héllo\r\nworld\n");

        let binary = temp_file("bin.dat", &[0x50, 0x00, 0x51]);
        assert_eq!(read_text(&binary).unwrap_err().kind(), io::ErrorKind::InvalidData);

        let latin1 = temp_file("latin1.txt", &[0x63, 0x61, 0x66, 0xe9]);
        assert_eq!(read_text(&latin1).unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn crlf_files_load_normalized_and_save_as_crlf() {
        let path = temp_file("crlf.txt", b"a\r\nb\r\n");
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.text, "a\nb\n");
        assert_eq!(loaded.line_ending, LineEnding::Crlf);

        write_text(&path, "a\nb\nc\n", loaded.line_ending).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"a\r\nb\r\nc\r\n");
    }

    #[test]
    fn edits_during_a_save_keep_the_document_dirty() {
        let mut doc = Document::untitled(1);
        doc.mark_edited();
        let saving = doc.revision();
        doc.mark_edited();
        doc.mark_saved(PathBuf::from("/tmp/notes.md"), saving);
        assert!(doc.is_dirty());
        assert_eq!(doc.language().label, "Markdown");

        doc.mark_saved(PathBuf::from("/tmp/notes.md"), doc.revision());
        assert!(!doc.is_dirty());
    }

    #[test]
    fn untitled_documents_are_numbered_after_the_first() {
        assert_eq!(Document::untitled(1).display_name(), "Untitled");
        assert_eq!(Document::untitled(3).display_name(), "Untitled 3");
        assert_eq!(Document::from_path(PathBuf::from("/a/b.rs")).untitled_number(), None);
    }
}
