use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{self as unix_fs, MetadataExt as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Writes editor text with the document's line ending. The text goes to a temp file
/// that's flushed to disk and renamed over the original, so a crash or power cut
/// leaves the old file or the new one, never half of one.
pub fn write_text(path: &Path, text: &str, line_ending: LineEnding) -> io::Result<()> {
    let text = line_ending.apply(text);
    let bytes = text.as_bytes();
    // Save through a symlink to the file it points at, so the link stays a link.
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let existing = match fs::metadata(&target) {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    if let Some(meta) = &existing {
        // A rename would happily replace a read-only file, so ask the file itself first.
        OpenOptions::new().write(true).open(&target)?;
        // A rename gives the path a new inode, which would cut its other hard links loose.
        if meta.nlink() > 1 {
            return write_in_place(&target, bytes);
        }
    }
    match write_replacing(&target, bytes, existing.as_ref()) {
        Ok(true) => Ok(()),
        // The swap would change the file's owner or group.
        Ok(false) => write_in_place(&target, bytes),
        // The folder doesn't let us create the temp file, but the file itself is writable.
        Err(err) if existing.is_some() && err.kind() == io::ErrorKind::PermissionDenied => {
            write_in_place(&target, bytes)
        }
        Err(err) => Err(err),
    }
}

/// Writes `bytes` to a hidden temp file beside `target` and renames it into place.
/// Returns `Ok(false)` without touching `target` if the new file can't keep its owner.
fn write_replacing(target: &Path, bytes: &[u8], existing: Option<&fs::Metadata>) -> io::Result<bool> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = match target.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    // Unique per save, since two saves of one file can be in flight at once.
    let tmp = dir.join(format!(
        ".{}.slate-{}-{}",
        name.to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));

    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        if let Some(meta) = existing {
            let ours = file.metadata()?;
            if ours.uid() != meta.uid() {
                return Ok(false);
            }
            if ours.gid() != meta.gid() && unix_fs::fchown(&file, None, Some(meta.gid())).is_err() {
                return Ok(false);
            }
            file.set_permissions(meta.permissions())?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, target)?;
        // Makes the rename itself survive a power cut. Some filesystems can't sync
        // a folder, and the data is already on disk, so a failure here is fine.
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
        Ok(true)
    })();
    if !matches!(result, Ok(true)) {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Overwrites a file where it is, for the cases a rename can't handle.
fn write_in_place(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).truncate(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
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

    /// An empty folder of its own, so one test's temp files can't show up in another's.
    fn save_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slate-save-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn saves_replace_the_file_and_leave_no_temp_files() {
        let dir = save_dir("replace");
        let path = dir.join("notes.md");
        fs::write(&path, "old text that is longer than the new one\n").unwrap();

        write_text(&path, "new\n", LineEnding::Lf).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");

        write_text(&dir.join("fresh.md"), "made\n", LineEnding::Lf).unwrap();
        assert_eq!(entries(&dir), ["fresh.md", "notes.md"]);
    }

    #[test]
    fn saves_keep_permissions() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = save_dir("perms");
        let path = dir.join("run.sh");
        fs::write(&path, "echo hi\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();

        write_text(&path, "echo bye\n", LineEnding::Lf).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o750);
    }

    #[test]
    fn saves_through_a_symlink_keep_the_link() {
        let dir = save_dir("symlink");
        let real = dir.join("real.txt");
        let link = dir.join("link.txt");
        fs::write(&real, "before\n").unwrap();
        unix_fs::symlink(&real, &link).unwrap();

        write_text(&link, "after\n", LineEnding::Lf).unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "after\n");
        assert_eq!(entries(&dir), ["link.txt", "real.txt"]);
    }

    #[test]
    fn saves_keep_hard_links_together() {
        let dir = save_dir("hardlink");
        let one = dir.join("one.txt");
        let two = dir.join("two.txt");
        fs::write(&one, "before\n").unwrap();
        fs::hard_link(&one, &two).unwrap();

        write_text(&one, "after\n", LineEnding::Lf).unwrap();
        assert_eq!(fs::read_to_string(&two).unwrap(), "after\n");
        assert_eq!(fs::metadata(&one).unwrap().ino(), fs::metadata(&two).unwrap().ino());
    }

    #[test]
    fn read_only_files_refuse_the_save() {
        let dir = save_dir("readonly");
        let path = dir.join("locked.txt");
        fs::write(&path, "keep me\n").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms).unwrap();

        let err = write_text(&path, "overwritten\n", LineEnding::Lf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep me\n");
        assert_eq!(entries(&dir), ["locked.txt"]);
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
