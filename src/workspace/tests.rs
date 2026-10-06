use std::path::{Path, PathBuf};

use gpui_kit::component::{Theme, WindowExt as _};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions,
    point, px, size,
};

use super::Workspace;
use crate::storage::{Storage, TabColorMode};
use crate::tab_color::TabColor;

/// A fresh folder for one test's settings, state, and files.
fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("slate-ui-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_workspace(cx: &mut TestAppContext, dir: &Path) -> (AnyWindowHandle, Entity<Workspace>) {
    let storage = Storage::in_dir(dir);
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::init(storage.themes_dir(), cx);
        super::init(cx);

        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(900.), px(600.)));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Workspace::new(storage, window, cx))
        })
        .expect("test window should open")
    })
}

/// Types into the focused editor, then lets the change events settle.
fn type_text(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.input(text, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    cx.update_window(handle, |_, window, cx| window.press(keys, cx))
        .unwrap();
    cx.run_until_parked();
}

fn click(cx: &mut TestAppContext, handle: AnyWindowHandle, id: &'static str) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click(id, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn has_dialog(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}

fn window_is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update_window(handle, |_, _, _| ()).is_ok()
}

fn tab_names(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Vec<String> {
    cx.update(|cx| {
        workspace
            .read(cx)
            .buffers
            .iter()
            .map(|buffer| buffer.document.display_name())
            .collect()
    })
}

fn open_file(cx: &mut TestAppContext, handle: AnyWindowHandle, workspace: &Entity<Workspace>, path: &Path) {
    let path = path.to_path_buf();
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.open_path(path, false, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn new_file_opens_a_numbered_tab(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("new-tab"));

    type_text(cx, handle, "first");
    press(cx, handle, "ctrl-n");

    assert!(!has_dialog(cx, handle), "a new tab never needs to ask");
    assert_eq!(tab_names(cx, &workspace), ["Untitled", "Untitled 2"]);
    assert_eq!(cx.update(|cx| workspace.read(cx).active), 1);

    press(cx, handle, "ctrl-tab");
    assert_eq!(cx.update(|cx| workspace.read(cx).active), 0);
}

#[gpui_kit::test]
fn closing_an_unsaved_tab_asks_and_cancel_keeps_it(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("close-cancel"));

    type_text(cx, handle, "hello");
    press(cx, handle, "ctrl-w");
    assert!(has_dialog(cx, handle));

    click(cx, handle, "dialog-cancel");
    assert!(!has_dialog(cx, handle));
    cx.update(|cx| {
        let buffer = workspace.read(cx).active_buffer();
        assert!(buffer.document.is_dirty());
        assert_eq!(buffer.editor.read(cx).value(), "hello");
    });
}

#[gpui_kit::test]
fn discarding_the_last_tab_leaves_a_fresh_one(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("close-discard"));

    type_text(cx, handle, "scratch");
    press(cx, handle, "ctrl-w");
    click(cx, handle, "dialog-discard");

    assert!(window_is_open(cx, handle));
    assert_eq!(tab_names(cx, &workspace), ["Untitled"]);
    cx.update(|cx| assert!(workspace.read(cx).active_buffer().is_pristine(cx)));
}

#[gpui_kit::test]
fn quitting_asks_about_each_unsaved_tab(cx: &mut TestAppContext) {
    let (handle, _workspace) = open_workspace(cx, &test_dir("quit-two"));

    type_text(cx, handle, "one");
    press(cx, handle, "ctrl-n");
    type_text(cx, handle, "two");

    press(cx, handle, "ctrl-q");
    assert!(has_dialog(cx, handle), "first unsaved tab");
    click(cx, handle, "dialog-discard");
    assert!(has_dialog(cx, handle), "second unsaved tab");
    click(cx, handle, "dialog-discard");

    assert!(!window_is_open(cx, handle));
}

#[gpui_kit::test]
fn quitting_without_edits_closes_right_away(cx: &mut TestAppContext) {
    let (handle, _workspace) = open_workspace(cx, &test_dir("quit-clean"));

    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap();
    press(cx, handle, "ctrl-q");

    assert!(!window_is_open(cx, handle));
}

#[gpui_kit::test]
fn opening_a_file_reuses_the_empty_tab_and_is_remembered(cx: &mut TestAppContext) {
    let dir = test_dir("open");
    let file = dir.join("notes.md");
    std::fs::write(&file, "# Notes\n").unwrap();
    let (handle, workspace) = open_workspace(cx, &dir);

    open_file(cx, handle, &workspace, &file);
    open_file(cx, handle, &workspace, &file);

    assert_eq!(tab_names(cx, &workspace), ["notes.md"]);
    cx.update(|cx| {
        let buffer = workspace.read(cx).active_buffer();
        assert_eq!(buffer.editor.read(cx).value(), "# Notes\n");
        assert_eq!(buffer.document.language().label, "Markdown");
    });

    let state = Storage::in_dir(&dir).load_state();
    assert_eq!(state.recent_files, [file]);
}

#[gpui_kit::test]
fn tab_colors_are_saved_per_file(cx: &mut TestAppContext) {
    let dir = test_dir("colors");
    let file = dir.join("main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();

    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &file);
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            let id = workspace.active_buffer().id;
            workspace.set_buffer_color(id, Some(TabColor::Teal), cx);
        })
    });

    // A second window with the same storage picks the color back up.
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &file);
    cx.update(|cx| assert_eq!(workspace.read(cx).active_buffer().color, Some(TabColor::Teal)));
}

#[gpui_kit::test]
fn dragging_a_tab_reorders_without_changing_the_active_one(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("reorder"));
    press(cx, handle, "ctrl-n");
    press(cx, handle, "ctrl-n");
    assert_eq!(tab_names(cx, &workspace), ["Untitled", "Untitled 2", "Untitled 3"]);

    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| {
            let first = workspace.buffers[0].id;
            workspace.move_buffer(first, 3, window, cx);
        });
    })
    .unwrap();

    assert_eq!(tab_names(cx, &workspace), ["Untitled 2", "Untitled 3", "Untitled"]);
    cx.update(|cx| {
        assert_eq!(workspace.read(cx).active_buffer().document.display_name(), "Untitled 3");
    });
}

fn folder_fixture(name: &str) -> PathBuf {
    let dir = test_dir(name);
    std::fs::create_dir_all(dir.join("project/src")).unwrap();
    std::fs::create_dir_all(dir.join("project/.git")).unwrap();
    std::fs::write(dir.join("project/src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("project/README.md"), "# Hi\n").unwrap();
    dir
}

/// The rows the sidebar's tree currently shows.
fn sidebar_labels(cx: &mut TestAppContext, workspace: &Entity<Workspace>) -> Vec<String> {
    cx.run_until_parked();
    cx.update(|cx| {
        let tree_state = workspace.read(cx).sidebar.read(cx).tree_state.clone();
        let tree_state = tree_state.read(cx);
        (0..)
            .map_while(|ix| tree_state.entry(ix))
            .map(|entry| entry.item().label.to_string())
            .collect()
    })
}

#[gpui_kit::test]
fn opening_a_folder_shows_it_in_the_sidebar_without_hidden_files(cx: &mut TestAppContext) {
    let dir = folder_fixture("sidebar");
    let (handle, workspace) = open_workspace(cx, &dir);

    open_file(cx, handle, &workspace, &dir.join("project"));

    cx.update(|cx| assert!(workspace.read(cx).sidebar_open));
    assert_eq!(sidebar_labels(cx, &workspace), ["src", "README.md"]);
}

#[gpui_kit::test]
fn ctrl_b_opens_the_active_files_folder(cx: &mut TestAppContext) {
    let dir = folder_fixture("ctrl-b");
    let (handle, workspace) = open_workspace(cx, &dir);
    open_file(cx, handle, &workspace, &dir.join("project/README.md"));

    press(cx, handle, "ctrl-b");
    cx.run_until_parked();

    cx.update(|cx| {
        let workspace = workspace.read(cx);
        assert!(workspace.sidebar_open);
        assert_eq!(workspace.sidebar.read(cx).root(), Some(dir.join("project").as_path()));
    });
    assert_eq!(sidebar_labels(cx, &workspace), ["src", "README.md"]);

    press(cx, handle, "ctrl-b");
    cx.update(|cx| assert!(!workspace.read(cx).sidebar_open));
}

fn theme_name(cx: &mut TestAppContext) -> String {
    cx.update(|cx| Theme::global(cx).theme_name().to_string())
}

#[gpui_kit::test]
fn theme_and_font_settings_apply_and_persist(cx: &mut TestAppContext) {
    let dir = test_dir("appearance");
    let (_, workspace) = open_workspace(cx, &dir);
    assert_eq!(theme_name(cx), "Slate Dark");

    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.update_appearance(cx, |settings| {
                settings.theme = Some("Gruvbox Light".into());
                settings.editor_font_size = Some(17.0);
            });
        })
    });
    assert_eq!(theme_name(cx), "Gruvbox Light");
    cx.update(|cx| {
        let theme = Theme::global(cx);
        assert!(!theme.is_dark());
        assert_eq!(theme.mono_font_size, px(17.));
    });

    // A fresh window reads the saved settings and lands on the same theme.
    let settings = Storage::in_dir(&dir).load_settings();
    assert_eq!(settings.theme.as_deref(), Some("Gruvbox Light"));
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| workspace.update_appearance(cx, |settings| settings.theme = None))
    });
    assert_eq!(theme_name(cx), "Slate Dark");
    let (_, _) = open_workspace(cx, &dir);
    assert_eq!(theme_name(cx), "Slate Dark");
}

#[gpui_kit::test]
fn ctrl_comma_opens_settings_and_escape_closes_them(cx: &mut TestAppContext) {
    let (handle, _) = open_workspace(cx, &test_dir("settings-sheet"));
    let has_sheet = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    };

    press(cx, handle, "ctrl-,");
    assert!(has_sheet(cx));
    press(cx, handle, "escape");
    assert!(!has_sheet(cx));
}

fn settle(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    for _ in 0..10 {
        cx.executor().advance_clock(std::time::Duration::from_millis(50));
        cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();
    }
}

#[gpui_kit::test]
fn with_a_font_list_open_the_first_x_click_closes_the_list(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("sheet-close"));
    let has_sheet = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    };
    // The open list has its own "close" button, so aim at the sheet's header.
    let click_sheet_x = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.within("sheet-content").click("close", cx);
        })
        .unwrap();
        cx.run_until_parked();
    };

    click(cx, handle, "settings");
    settle(cx, handle);
    cx.update_window(handle, |_, window, cx| {
        let picker = workspace.read(cx).font_pickers.as_ref().unwrap().editor.clone();
        picker.update(cx, |picker, cx| picker.focus(window, cx));
    })
    .unwrap();
    press(cx, handle, "enter");
    settle(cx, handle);

    // Kit's Select swallows the mouse-down that closes its list.
    click_sheet_x(cx);
    assert!(has_sheet(cx));
    settle(cx, handle);
    click_sheet_x(cx);
    assert!(!has_sheet(cx));
}

#[gpui_kit::test]
fn switching_themes_changes_syntax_and_palette_colors(cx: &mut TestAppContext) {
    let (_, workspace) = open_workspace(cx, &test_dir("theme-colors"));
    let snapshot = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let theme = Theme::global(cx);
            let keyword = theme.highlight_theme.style("keyword").and_then(|style| style.color);
            (theme.highlight_theme.name.clone(), keyword, theme.red)
        })
    };

    let slate = snapshot(cx);
    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.update_appearance(cx, |settings| settings.theme = Some("Gruvbox Dark".into()))
        })
    });
    let gruvbox = snapshot(cx);

    assert_eq!(gruvbox.0, "Gruvbox Dark");
    assert_ne!(slate.1, gruvbox.1);
    assert_ne!(slate.2, gruvbox.2);
}

#[gpui_kit::test]
fn tab_colors_follow_the_mode_and_a_picked_color_wins(cx: &mut TestAppContext) {
    let (handle, workspace) = open_workspace(cx, &test_dir("tab-modes"));
    press(cx, handle, "ctrl-n");
    press(cx, handle, "ctrl-n");
    let colors = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let workspace = workspace.read(cx);
            (0..workspace.buffers.len())
                .map(|ix| workspace.tab_hsla(ix, &workspace.buffers[ix], cx))
                .collect::<Vec<_>>()
        })
    };

    // Theme mode is the default: every tab gets a different syntax color.
    let themed = colors(cx);
    assert!(themed.iter().all(Option::is_some));
    assert_ne!(themed[0], themed[1]);
    assert_ne!(themed[1], themed[2]);

    cx.update(|cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.set_tab_color_mode(TabColorMode::Off, cx);
            let id = workspace.buffers[1].id;
            workspace.set_buffer_color(id, Some(TabColor::Teal), cx);
        })
    });
    let off = colors(cx);
    assert_eq!(off[0], None);
    assert!(off[1].is_some());
    assert_eq!(off[2], None);
}
