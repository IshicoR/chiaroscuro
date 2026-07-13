mod action;
mod app;
mod appearance;
mod configuration;
mod menu;
mod navigation;
mod screen;
mod session;
mod telemetry;
mod widget;
mod window;

use app::Chiaroscuro;

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

fn main() -> iced::Result {
    iced::application(Chiaroscuro::new, Chiaroscuro::update, Chiaroscuro::view)
        .title(Chiaroscuro::title)
        .theme(Chiaroscuro::theme)
        .subscription(Chiaroscuro::subscription)
        .font(iced_fonts::LUCIDE_FONT_BYTES)
        .window(window::settings())
        .centered()
        .run()
}
