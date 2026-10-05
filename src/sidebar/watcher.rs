use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::*;
use notify::{RecursiveMode, Watcher as _};

use super::Sidebar;

/// How long to wait for a burst of file events to settle before rescanning.
const SETTLE: Duration = Duration::from_millis(150);

/// Watches each loaded folder (not recursively), so only folders the user has
/// opened in the tree use inotify watches.
pub struct DirWatcher {
    watcher: notify::RecommendedWatcher,
    watched: HashSet<PathBuf>,
    _task: Task<()>,
}

impl DirWatcher {
    pub fn new(cx: &mut Context<Sidebar>) -> notify::Result<Self> {
        let (tx, mut rx) = mpsc::unbounded::<Vec<PathBuf>>();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result
                && !event.kind.is_access()
            {
                let _ = tx.unbounded_send(event.paths);
            }
        })?;

        let task = cx.spawn(async move |this, cx| {
            while let Some(paths) = rx.next().await {
                let mut changed: HashSet<PathBuf> = paths.into_iter().collect();
                cx.background_executor().timer(SETTLE).await;
                while let Ok(more) = rx.try_recv() {
                    changed.extend(more);
                }
                if this
                    .update(cx, |sidebar, cx| sidebar.rescan_changed(changed, cx))
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            watcher,
            watched: HashSet::new(),
            _task: task,
        })
    }

    pub fn watch(&mut self, dir: &PathBuf) {
        if self.watched.contains(dir) {
            return;
        }
        match self.watcher.watch(dir, RecursiveMode::NonRecursive) {
            Ok(()) => {
                self.watched.insert(dir.clone());
            }
            Err(err) => eprintln!("slate: not watching {}: {err}", dir.display()),
        }
    }

    pub fn unwatch(&mut self, dir: &PathBuf) {
        if self.watched.remove(dir) {
            let _ = self.watcher.unwatch(dir);
        }
    }
}
