mod main_view;
mod theme;

pub(crate) use main_view::MainView;
pub(crate) use theme::{ThemeColors, ThemeMode};

use gpui_kit::{
    AppContext as _, Bounds, Point, Result, Size, TitlebarOptions, WindowOptions, application,
    open_window,
};

/// Default window width in logical pixels.
const WINDOW_WIDTH: f32 = 1280.0;
/// Default window height in logical pixels.
const WINDOW_HEIGHT: f32 = 800.0;
/// Native window title.
const WINDOW_TITLE: &str = "Zplayer";

pub(crate) fn run() -> Result<()> {
    application().run(|cx| {
        gpui_kit::init(cx);
        let bounds = Bounds {
            origin: Point::default(),
            size: Size {
                width: gpui_kit::px(WINDOW_WIDTH),
                height: gpui_kit::px(WINDOW_HEIGHT),
            },
        };
        open_window(
            WindowOptions {
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| MainView::new(window, cx)),
        )
        .expect("failed to open Zplayer main window");
    });
    Ok(())
}
