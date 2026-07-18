use iced::{
    Background, Border, Color, Theme,
    theme::Style,
    widget::{button, container},
};

pub mod typography;

const ACTION_BUTTON_CORNER_RADIUS: f32 = 24.0;
const WINDOW_CORNER_RADIUS: f32 = 10.0;

pub fn application(theme: &Theme) -> Style {
    Style {
        background_color: Color::TRANSPARENT,
        text_color: theme.extended_palette().background.base.text,
    }
}

pub fn content(theme: &Theme, rounded: bool) -> container::Style {
    let colors = theme.extended_palette().background.base;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
        .border(Border {
            radius: if rounded {
                iced::border::Radius {
                    bottom_right: WINDOW_CORNER_RADIUS,
                    ..iced::border::Radius::default()
                }
            } else {
                iced::border::Radius::default()
            },
            ..Border::default()
        })
}

pub fn action_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border.radius = ACTION_BUTTON_CORNER_RADIUS.into();
    style
}

pub fn secondary_action_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::secondary(theme, status);
    style.border.radius = ACTION_BUTTON_CORNER_RADIUS.into();
    style
}
