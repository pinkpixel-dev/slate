mod assets;
mod document;
mod language;
mod theme;
mod workspace;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::workspace::Workspace;

const APP_ID: &str = "dev.pinkpixel.Slate";

fn main() {
    let file_arg = std::env::args_os().nth(1).map(std::path::PathBuf::from);

    application().with_assets(assets::AppAssets).run(move |cx| {
        init(cx);
        theme::init(cx);
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
            open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(window, cx)))
                .expect("failed to open the Slate window");

        if let Some(path) = file_arg {
            _ = window.update(cx, |_, window, cx| {
                workspace.update(cx, |workspace, cx| workspace.load(path, window, cx));
            });
        }
        cx.activate(true);
    });
}
