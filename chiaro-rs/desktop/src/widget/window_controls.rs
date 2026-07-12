use iced::{
    Element,
    alignment::{Horizontal, Vertical},
    widget::{button, row, text, tooltip},
};
use iced_fonts::lucide;

use crate::appearance;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Minimize,
    ToggleMaximize,
    Close,
}

pub fn view() -> Element<'static, Message> {
    row![
        control(
            lucide::minus().size(16),
            "Minimize",
            Message::Minimize,
            false
        ),
        control(
            lucide::square().size(14),
            "Maximize",
            Message::ToggleMaximize,
            false
        ),
        control(lucide::x().size(16), "Close", Message::Close, true),
    ]
    .align_y(Vertical::Center)
    .into()
}

fn control(
    icon: iced::widget::Text<'static>,
    label: &'static str,
    message: Message,
    destructive: bool,
) -> Element<'static, Message> {
    let control = button(icon.align_x(Horizontal::Center).align_y(Vertical::Center))
        .width(appearance::WINDOW_CONTROL_SIZE)
        .height(appearance::WINDOW_CONTROL_SIZE)
        .padding(0)
        .on_press(message)
        .style(move |theme, status| appearance::window_control(theme, status, destructive));

    tooltip(control, text(label).size(12), tooltip::Position::Bottom).into()
}
