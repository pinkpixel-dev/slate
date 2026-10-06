mod assets;
mod document;
mod file_tree;
mod language;
mod session;
mod sidebar;
mod storage;
mod tab_color;
mod theme;
mod workspace;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::storage::Storage;
use crate::workspace::Workspace;

const APP_ID: &str = "dev.pinkpixel.Slate";

fn main() {
    let file_args: Vec<std::path::PathBuf> = std::env::args_os().skip(1).map(Into::into).collect();

    let storage = Storage::from_env();

    application().with_assets(assets::AppAssets).run(move |cx| {
        init(cx);
        theme::init(storage.themes_dir(), cx);
        sidebar::init(cx);
        workspace::init(cx);

        // Closing the last window ends the process, like any desktop editor.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(480.), px(320.))),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some(APP_ID.into()),
            ..TitleBar::window_options()
        };

        let (window, workspace) =
            open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(storage, window, cx)))
                .expect("failed to open the Slate window");

        // `slate a.md b.rs` opens each file in a tab, and a folder opens in the
        // sidebar. A path that doesn't exist yet opens empty and is created on save.
        // With no arguments, the last session comes back instead.
        _ = window.update(cx, |_, window, cx| {
            workspace.update(cx, |workspace, cx| {
                if file_args.is_empty() {
                    workspace.restore_session(window, cx);
                }
                for path in file_args {
                    workspace.open_path(path, true, window, cx);
                }
            });
        });
        cx.activate(true);
    });
}
