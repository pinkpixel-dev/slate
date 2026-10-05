mod theme;
mod workspace;

use gpui_kit::component::TitleBar;
use gpui_kit::*;

use crate::workspace::Workspace;

const APP_ID: &str = "dev.pinkpixel.Slate";

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
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

        open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(window, cx)))
            .expect("failed to open the Slate window");
        cx.activate(true);
    });
}
