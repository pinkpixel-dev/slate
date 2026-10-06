use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::tab_color::TabColor;
use crate::text_format::LineEnding;

/// The open tabs and sidebar folder, saved to `~/.local/state/slate/session.json`
/// so the next launch without arguments picks up where this one stopped.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    /// Index into `tabs`.
    pub active: usize,
    pub folder: Option<PathBuf>,
    pub sidebar_open: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTab {
    /// `None` for an untitled tab.
    pub path: Option<PathBuf>,
    /// Which "Untitled N" an untitled tab was.
    pub untitled_number: usize,
    /// Unsaved text. `None` means the tab was clean and reopens from disk.
    pub text: Option<String>,
    /// The file's modified time when its unsaved edits started, to spot
    /// changes made on disk in the meantime.
    pub disk_modified: Option<SystemTime>,
    pub color: Option<TabColor>,
    /// Cursor byte offset.
    pub cursor: usize,
    /// The Markdown preview was open.
    pub preview: bool,
    /// What Save writes, kept so stashed edits to a CRLF file stay CRLF.
    pub line_ending: LineEnding,
}

impl SessionTab {
    /// Whether restoring this tab should warn that the file changed under its unsaved edits.
    pub fn disk_changed(&self) -> bool {
        match (&self.path, &self.text) {
            (Some(path), Some(_)) => crate::document::modified(path) != self.disk_modified,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_with_unsaved_text_round_trips() {
        let session = Session {
            tabs: vec![
                SessionTab {
                    untitled_number: 2,
                    text: Some("draft\n".into()),
                    color: Some(TabColor::Teal),
                    cursor: 3,
                    ..SessionTab::default()
                },
                SessionTab {
                    path: Some("/tmp/notes.md".into()),
                    disk_modified: Some(SystemTime::UNIX_EPOCH),
                    text: Some("edited".into()),
                    line_ending: LineEnding::Crlf,
                    ..SessionTab::default()
                },
            ],
            active: 1,
            folder: Some("/tmp".into()),
            sidebar_open: true,
        };
        let json = crate::storage::to_json(&session);
        assert_eq!(serde_json::from_str::<Session>(&json).unwrap(), session);
    }

    #[test]
    fn disk_changes_are_only_checked_for_edited_files() {
        let dir = std::env::temp_dir().join(format!("slate-session-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("watched.txt");
        std::fs::write(&path, "one").unwrap();
        let now = crate::document::modified(&path);

        let mut tab = SessionTab {
            path: Some(path.clone()),
            disk_modified: now,
            text: Some("edited".into()),
            ..SessionTab::default()
        };
        assert!(!tab.disk_changed());

        tab.disk_modified = Some(SystemTime::UNIX_EPOCH);
        assert!(tab.disk_changed());

        tab.text = None;
        assert!(!tab.disk_changed(), "a clean tab just reloads from disk");
    }
}
