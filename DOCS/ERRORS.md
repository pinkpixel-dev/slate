# Errors

Failed approaches and difficult bugs that are worth remembering.

Log a failure when it took more than two attempts, the root cause was somewhere other than the symptom, the behavior is environment-specific and will recur, or a reasonable next approach would fail the same way. This is not a bug tracker. Ordinary bugs found and fixed quickly do not belong here.

Format:

```md
## YYYY-MM-DD

### Note: <the trap, stated as a fact>

What did not work: <the approach and the actual observed failure>

What worked instead: <the fix, specific enough to repeat>

Note for next time: <the general lesson, one sentence>
```

Title the trap, not the symptom, so someone hitting it again can find the entry. If the same trap returns, update the existing entry rather than adding a second one.

## 2026-10-05

### Note: spectacle can't capture windows on COSMIC

What did not work: `spectacle -b -n -a -o shot.png` (active-window capture) silently wrote no file under the COSMIC Wayland session.

What worked instead: `cosmic-screenshot --interactive=false --modal=false --notify=false -s <dir>` saves a full-screen PNG to `<dir>` and prints its path.

Note for next time: For visual checks on this machine, launch `target/debug/slate` in the background, wait about 4 seconds, then use `cosmic-screenshot`.

### Note: The SQL grammar needs cc 1.2.x pinned in Cargo.lock

What did not work: Enabling `tree-sitter-sql` on `gpui-kit` failed to resolve. `tree-sitter-sequel 0.3.8` requires `cc ~1.2.1`, but the lockfile already had `cc 1.6.0` (pulled in through `embed-resource` by `gpui-pre`).

What worked instead: `cargo update -p cc --precise 1.2.67`. `embed-resource` only needs `cc ^1.2`, so the newest 1.2.x satisfies both.

Note for next time: A blanket `cargo update` may try to move `cc` back to 1.6 and break the build again. Keep `cc` on 1.2.x until `tree-sitter-sequel` loosens its requirement.

### Note: `use gpui_kit::*` breaks `#[test]` when test-support is on

What did not work: A test module with `use gpui_kit::*;` and `#[gpui_kit::test]` failed with "recursion limit reached while expanding `#[test]`". With the `test-support` feature, the glob imports GPUI's own `test` macro, which shadows Rust's built-in `#[test]` that Kit's macro expands to.

What worked instead: Import Kit items by name in test modules (`use gpui_kit::{AnyWindowHandle, TestAppContext, ...}`), with no glob.

Note for next time: Never glob-import `gpui_kit` in a `#[cfg(test)]` module.

### Note: Real `notify` watchers panic GPUI's test scheduler

What did not work: Starting `FileWatcher` inside a headless test and then writing to a watched file. The test failed with "Detected activity on thread Some(\"notify-rs inotify loop\") ... Your test is not deterministic", because the inotify thread scheduled work on GPUI's test executor from outside the test thread.

What worked instead: `Workspace::new` skips `FileWatcher` under `cfg!(test)`, and the tests call `Workspace::on_files_changed` directly with the changed paths. The sidebar's `DirWatcher` only gets away with running in tests because those tests never write into a watched folder.

Note for next time: In headless tests, call a watcher's callback directly instead of letting a real OS watcher run.
