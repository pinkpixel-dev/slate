use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};

use super::Workspace;
use super::tests::{click, open_file, open_workspace, press, test_dir, type_text};

/// Writes `text` as another program would, with a modified time clearly
/// after the last one, so fast filesystems can't hide the change.
fn write_externally(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    let later = SystemTime::now() + Duration::from_secs(5);
    std::fs::File::options().write(true).open(path).unwrap().set_modified(later).unwrap();
}

/// What the watcher does once a change settles.
fn notice(cx: &mut TestAppContext, handle: AnyWindowHandle, workspace: &Entity<Workspace>, path: &Path) {
    let changed: HashSet<PathBuf> = [path.to_path_buf()].into();
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.on_files_changed(&changed, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
}

/// (text, unsaved, changed-on-disk bar showing) for the active tab.
fn active(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> (String, bool, bool) {
    cx.update(|cx| {
        let buffer = workspace.read(cx).active_buffer();
        (
            buffer.editor.read(cx).value().to_string(),
            buffer.document.is_dirty(),
            buffer.disk_conflict,
        )
    })
}

fn setup(cx: &mut TestAppContext, name: &str, text: &str) -> (AnyWindowHandle, Entity<Workspace>, PathBuf) {
    let dir = test_dir(name);
    let path = dir.join("notes.txt");
    std::fs::write(&path, text).unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &path);
    (handle, workspace, path)
}

#[gpui_kit::test]
fn a_clean_tab_reloads_quietly(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-clean", "one\n");
    write_externally(&path, "two\n");
    notice(cx, handle, &workspace, &path);
    assert_eq!(active(cx, &workspace), ("two\n".into(), false, false));
}

#[gpui_kit::test]
fn unsaved_edits_ask_and_keep_mine_keeps_them(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-keep", "one\n");
    press(cx, handle, "ctrl-end");
    type_text(cx, handle, "mine");
    write_externally(&path, "theirs\n");
    notice(cx, handle, &workspace, &path);
    assert_eq!(active(cx, &workspace), ("one\nmine".into(), true, true));

    click(cx, handle, "disk-keep");
    assert_eq!(active(cx, &workspace), ("one\nmine".into(), true, false));

    // The same disk version doesn't ask again.
    notice(cx, handle, &workspace, &path);
    assert!(!active(cx, &workspace).2);
}

#[gpui_kit::test]
fn reload_replaces_unsaved_edits_with_the_disk_version(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-reload", "one\n");
    type_text(cx, handle, "mine");
    write_externally(&path, "theirs\n");
    notice(cx, handle, &workspace, &path);

    click(cx, handle, "disk-reload");
    assert_eq!(active(cx, &workspace), ("theirs\n".into(), false, false));
}

#[gpui_kit::test]
fn our_own_save_is_not_an_outside_change(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-own-save", "one\n");
    type_text(cx, handle, "edit ");
    press(cx, handle, "ctrl-s");
    type_text(cx, handle, "more ");
    notice(cx, handle, &workspace, &path);
    assert_eq!(active(cx, &workspace), ("edit more one\n".into(), true, false));
}

#[gpui_kit::test]
fn a_deleted_file_keeps_its_tab_as_unsaved(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-deleted", "keep me\n");
    std::fs::remove_file(&path).unwrap();
    notice(cx, handle, &workspace, &path);
    assert_eq!(active(cx, &workspace), ("keep me\n".into(), true, false));

    press(cx, handle, "ctrl-s");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me\n", "saving puts it back");
}

#[gpui_kit::test]
fn restoring_edits_to_a_changed_file_shows_the_bar(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-restore", "one\n");
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.restore_session(window, cx));
    })
    .unwrap();
    type_text(cx, handle, "mine ");
    press(cx, handle, "ctrl-q");

    write_externally(&path, "theirs\n");
    let storage = crate::storage::Storage::in_dir(path.parent().unwrap());
    let (handle, workspace) = cx.update(|cx| {
        gpui_kit::open_window(Default::default(), cx, |window, cx| {
            cx.new(|cx| Workspace::new(storage, window, cx))
        })
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.restore_session(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(active(cx, &workspace), ("mine one\n".into(), true, true));
}

#[gpui_kit::test]
fn compare_opens_a_read_only_diff_tab_that_closes_quietly(cx: &mut TestAppContext) {
    let (handle, workspace, path) = setup(cx, "disk-compare", "one\n");
    press(cx, handle, "ctrl-end");
    type_text(cx, handle, "mine");
    write_externally(&path, "theirs\n");
    notice(cx, handle, &workspace, &path);

    click(cx, handle, "disk-compare");
    let (name, text, scratch) = cx.update(|cx| {
        let buffer = workspace.read(cx).active_buffer();
        (
            buffer.document.display_name(),
            buffer.editor.read(cx).value().to_string(),
            buffer.document.is_scratch(),
        )
    });
    assert_eq!(name, "notes.txt (changes)");
    assert!(scratch);
    assert!(text.contains("-theirs\n+one\n+mine\n"), "{text}");

    assert!(!active(cx, &workspace).1, "fresh diff tab is clean");
    // Read-only, so typing doesn't make it unsaved and closing doesn't ask.
    type_text(cx, handle, "x");
    assert!(!active(cx, &workspace).1);
    press(cx, handle, "ctrl-w");
    let names: Vec<String> = cx.update(|cx| {
        workspace.read(cx).buffers.iter().map(|buffer| buffer.document.display_name()).collect()
    });
    assert_eq!(names, ["notes.txt"]);
}
