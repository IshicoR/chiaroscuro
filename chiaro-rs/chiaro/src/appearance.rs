use std::time::Duration;

use iced::{
    Background, Theme,
    widget::{button, container},
};

pub const APPLICATION_TITLE: &str = "Chiaroscuro";

pub const TITLE_BAR_HEIGHT: f32 = 34.0;
pub const MENU_BUTTON_SIZE: f32 = 24.0;
pub const WINDOW_CONTROL_BUTTON_SIZE: f32 = 24.0;
pub const HISTORY_WINDOW: Duration = Duration::from_secs(12);
pub const ICON_SIZE: u32 = 12;

pub const CONTENT_PADDING: f32 = 24.0;
pub const BUTTON_CORNER_RADIUS: f32 = 24.0;
pub const MENU_CORNER_RADIUS: f32 = 6.0;
pub const CARD_RADIUS: f32 = 10.0;

pub const CONTROL_TRANSITION_DURATION: Duration = Duration::from_millis(140);
pub const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub const DROP_DOWN_WIDTH: f32 = 190.0;

pub const TOOLTIP_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, Default)]
pub struct AppearanceState {
    mode: Mode,
}

impl AppearanceState {
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn theme(&self) -> Theme {
        match self.mode {
            Mode::Light => Theme::Light,
            Mode::Dark => Theme::Dark,
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
    style.border.radius = BUTTON_CORNER_RADIUS.into();
    style
}
