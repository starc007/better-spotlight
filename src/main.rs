mod apps;
mod fuzzy;
mod search;
mod theme;
mod ui;

use gpui::{
    px, size, App, AppContext, Application, Bounds, Point, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let window_size = size(px(theme::WINDOW_WIDTH), px(theme::WINDOW_HEIGHT));

        let screen = cx.displays().first().map(|d| d.bounds()).unwrap_or_else(|| {
            Bounds::new(Point::new(px(0.), px(0.)), size(px(1440.), px(900.)))
        });
        let x = screen.origin.x + (screen.size.width - window_size.width) / 2.;
        let y = screen.origin.y + screen.size.height * 0.18;
        let bounds = Bounds::new(Point::new(x, y), window_size);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(search::Spotlight::new);
            view.read(cx).focus.clone().focus(window);
            view
        })
        .unwrap();

        cx.activate(true);
    });
}
