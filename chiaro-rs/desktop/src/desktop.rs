mod action;
mod app;
mod appearance;
mod menu;
mod navigation;
mod screen;
mod session;
mod telemetry;
mod widget;
mod window;

use app::App;
use iced::{Size, window as iced_window};

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(iced_fonts::LUCIDE_FONT_BYTES)
        .window(iced_window::Settings {
            size: Size::new(960.0, 640.0),
            min_size: Some(Size::new(720.0, 480.0)),
            decorations: false,
            exit_on_close_request: false,
            ..iced_window::Settings::default()
        })
        .centered()
        .run()
}
