use iced::{
    Background, Theme,
    widget::{button, container},
};

const ACTION_BUTTON_CORNER_RADIUS: f32 = 24.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn theme(self) -> Theme {
        match self {
            Self::Light => Theme::Light,
            Self::Dark => Theme::Dark,
        }
    }
}

pub fn content(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette().background.base;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
}

pub fn action_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border.radius = ACTION_BUTTON_CORNER_RADIUS.into();
    style
}
