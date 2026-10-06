mod assets;
mod document;
mod file_index;
mod file_tree;
mod language;
mod line_ops;
mod minimap;
mod session;
mod sidebar;
mod single_instance;
mod storage;
mod tab_color;
mod text_format;
mod theme;
mod workspace;

use std::path::PathBuf;

use futures::StreamExt as _;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::single_instance::Claim;
use crate::storage::Storage;
use crate::workspace::Workspace;

const APP_ID: &str = "dev.pinkpixel.Slate";

fn main() {
    // Absolute, because a running Slate may get them and its folder isn't ours.
    let file_args: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .map(|path| std::path::absolute(&path).unwrap_or(path))
        .collect();

    // A Slate that's already running opens the files (or just comes forward) instead.
    let mut incoming = match single_instance::claim(&single_instance::socket_path(), &file_args) {
        Ok(Claim::Forwarded) => return,
        Ok(Claim::Primary(incoming)) => Some(incoming),
        Err(err) => {
            eprintln!("slate: can't share one window between launches: {err}");
            None
        }
    };

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

        let opened = open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(storage, window, cx)));
        let (window, workspace) = match opened {
            Ok(opened) => opened,
            Err(err) => {
                eprintln!("{}", window_error_message(&err.to_string()));
                // Nothing has opened yet, so there's nothing to save or close.
                // (`cx.quit()` here leaves GPUI's loop running with no window.)
                std::process::exit(1);
            }
        };

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

        if let Some(mut incoming) = incoming.take() {
            cx.spawn(async move |cx| {
                while let Some(paths) = incoming.next().await {
                    let opened = window.update(cx, |_, window, cx| {
                        workspace.update(cx, |workspace, cx| {
                            for path in paths {
                                workspace.open_path(path, true, window, cx);
                            }
                        });
                        window.activate_window();
                    });
                    if opened.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.activate(true);
    });
}

/// What to print when the window can't open. Almost always this means no
/// usable Vulkan driver, so point at the usual causes instead of a backtrace.
fn window_error_message(err: &str) -> String {
    let mut message = format!(
        "slate: couldn't open a window: {err}\n\n\
         Slate draws with Vulkan, so this usually means no Vulkan driver is available.\n\
         Run `vulkaninfo --summary` to see what your system has, and install the driver\n\
         for your GPU (vulkan-radeon, vulkan-intel, or nvidia-utils on Arch)."
    );
    if let Some(filter) = std::env::var_os("VK_LOADER_DRIVERS_SELECT") {
        message.push_str(&format!(
            "\n\nVK_LOADER_DRIVERS_SELECT is set to {:?}, which hides every Vulkan driver\n\
             that doesn't match it. Try running Slate with it unset.",
            filter.to_string_lossy()
        ));
    }
    message
}
