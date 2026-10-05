use std::path::Path;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::*;
use notify::{RecursiveMode, Watcher as _};

/// How long to wait for an editor's save (write, rename, chmod) to settle.
const SETTLE: Duration = Duration::from_millis(150);

/// Watches the custom themes folder and calls back when anything in it changes.
pub struct ThemeWatcher {
    _watcher: notify::RecommendedWatcher,
    _task: Task<()>,
}

impl ThemeWatcher {
    pub fn new<T: 'static>(
        dir: &Path,
        window: &mut Window,
        cx: &mut Context<T>,
        on_change: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
    ) -> notify::Result<Self> {
        let (tx, mut rx) = mpsc::unbounded::<()>();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result
                && !event.kind.is_access()
            {
                let _ = tx.unbounded_send(());
            }
        })?;
        watcher.watch(dir, RecursiveMode::NonRecursive)?;

        let task = cx.spawn_in(window, async move |this, cx| {
            while rx.next().await.is_some() {
                cx.background_executor().timer(SETTLE).await;
                while rx.try_recv().is_ok() {}
                if this.update_in(cx, |view, window, cx| on_change(view, window, cx)).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            _task: task,
        })
    }
}
