use iced::{
    Background, Border, Color, Theme,
    widget::{button, container},
};
use iced_aw::style::{Status as MenuStatus, menu_bar};

pub const TITLE_BAR_HEIGHT: f32 = 40.0;
pub const MENU_BUTTON_SIZE: f32 = 34.0;
pub const WINDOW_CONTROL_SIZE: f32 = 34.0;
pub const CONTENT_PADDING: f32 = 24.0;
pub const CORNER_RADIUS: f32 = 4.0;

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

pub fn title_bar(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette().background.weak;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
}

pub fn content(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette().background.base;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
}

pub fn menu_bar(theme: &Theme, status: MenuStatus) -> menu_bar::Style {
    let palette = theme.extended_palette();

    menu_bar::Style {
        bar_background: Background::Color(Color::TRANSPARENT),
        bar_border: Border::default(),
        menu_background: Background::Color(palette.background.base.color),
        menu_border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: CORNER_RADIUS.into(),
        },
        path: Background::Color(Color::TRANSPARENT),
        path_border: Border::default(),
        ..menu_bar::primary(theme, status)
    }
}

pub fn menu_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let base = button::Style {
        text_color: palette.background.base.text,
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette.background.strong.color)),
            text_color: palette.background.strong.text,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: base.text_color.scale_alpha(0.45),
            ..base
        },
        button::Status::Active => base,
    }
}

pub fn menu_toggle(theme: &Theme, status: button::Status, expanded: bool) -> button::Style {
    let mut style = menu_button(theme, status);

    if expanded && status == button::Status::Active {
        let colors = theme.extended_palette().background.strong;
        style.background = Some(Background::Color(colors.color));
        style.text_color = colors.text;
    }

    style
}

pub fn window_control(
    theme: &Theme,
    status: button::Status,
    destructive: bool,
    hover_progress: f32,
) -> button::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base;
    let target = if destructive {
        palette.danger.base
    } else {
        palette.background.strong
    };
    let progress = if status == button::Status::Pressed {
        1.0
    } else {
        hover_progress.clamp(0.0, 1.0)
    };

    let mut style = button::Style {
        background: Some(Background::Color(target.color.scale_alpha(progress))),
        text_color: mix_color(base.text, target.text, progress),
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    };

    if status == button::Status::Disabled {
        style.text_color = style.text_color.scale_alpha(0.45);
    }

    style
}

fn mix_color(start: Color, end: Color, amount: f32) -> Color {
    Color {
        r: start.r + (end.r - start.r) * amount,
        g: start.g + (end.g - start.g) * amount,
        b: start.b + (end.b - start.b) * amount,
        a: start.a + (end.a - start.a) * amount,
    }
}

pub fn action_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border.radius = CORNER_RADIUS.into();
    style
}

#[cfg(test)]
mod tests {
    use super::{AppearanceState, Mode};

    #[test]
    fn light_is_the_default_theme() {
        assert_eq!(AppearanceState::default().mode(), Mode::Light);
    }

    #[test]
    fn theme_mode_can_be_changed() {
        let mut state = AppearanceState::default();

        state.set_mode(Mode::Dark);

        assert_eq!(state.mode(), Mode::Dark);
    }
}
