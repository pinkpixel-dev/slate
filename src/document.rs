use std::io;
use std::path::{Path, PathBuf};

use crate::language::{self, Language};

/// The file behind the editor: where it lives, what language it is, and whether
/// it has unsaved edits.
pub struct Document {
    path: Option<PathBuf>,
    language: Language,
    /// Bumped on every edit.
    revision: u64,
    /// The revision that was last written to (or read from) disk.
    saved_revision: u64,
}

impl Document {
    pub fn untitled() -> Self {
        Self {
            path: None,
            language: Language::PLAIN,
            revision: 0,
            saved_revision: 0,
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self {
            language: language::detect(&path),
            path: Some(path),
            revision: 0,
            saved_revision: 0,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn display_name(&self) -> String {
        self.path
            .as_deref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
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

    pub fn mark_edited(&mut self) {
        self.revision += 1;
    }

    /// Records that `revision` was written to `path`. Edits made while the write
    /// was in flight keep the document dirty.
    pub fn mark_saved(&mut self, path: PathBuf, revision: u64) {
        if self.path.as_deref() != Some(path.as_path()) {
            self.language = language::detect(&path);
            self.path = Some(path);
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

pub fn write_text(path: &Path, text: &str) -> io::Result<()> {
    std::fs::write(path, text)
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
    fn edits_during_a_save_keep_the_document_dirty() {
        let mut doc = Document::untitled();
        doc.mark_edited();
        let saving = doc.revision();
        doc.mark_edited();
        doc.mark_saved(PathBuf::from("/tmp/notes.md"), saving);
        assert!(doc.is_dirty());
        assert_eq!(doc.language().label, "Markdown");

        doc.mark_saved(PathBuf::from("/tmp/notes.md"), doc.revision());
        assert!(!doc.is_dirty());
    }
}
