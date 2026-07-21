use iced::{
    Background, Border, Color, Theme,
    widget::{button, container},
};
use iced_plot::{GridStyle, PlotStyle, default_style};

const PLOT_CORNER_RADIUS: f32 = 8.0;

pub(super) fn plot(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let mut style = default_style(theme);

    style.frame.background = Some(Color::TRANSPARENT.into());
    style.plot_area.background = Some(palette.background.base.color.into());
    style.plot_area.border = Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: PLOT_CORNER_RADIUS.into(),
    };
    style.legend.background = Some(with_alpha(palette.background.weaker.color, 0.98).into());
    style.legend.border = Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: PLOT_CORNER_RADIUS.into(),
    };
    style.tooltip.background = Some(with_alpha(palette.background.weaker.color, 0.98).into());
    style.tooltip.text_color = Some(palette.background.weaker.text);
    style.tooltip.border = Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: PLOT_CORNER_RADIUS.into(),
    };
    style.grid = GridStyle {
        major: with_alpha(palette.background.strong.color, 0.72),
        minor: with_alpha(palette.background.weak.color, 0.72),
        sub_minor: with_alpha(palette.background.weak.color, 0.36),
    };
    style.tick_label_color = with_alpha(palette.background.base.text, 0.76);
    style.axis_label_color = with_alpha(palette.background.base.text, 0.92);

    style
}

pub(super) fn tooltip(theme: &Theme) -> container::Style {
    plot(theme).tooltip
}

pub(super) fn context_menu(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.background.weaker.text),
        background: Some(palette.background.weaker.color.into()),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub(super) fn context_menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Active | button::Status::Disabled => None,
        button::Status::Hovered => Some(palette.background.weak.color),
        button::Status::Pressed => Some(palette.background.neutral.color),
    };

    button::Style {
        background: background.map(Background::Color),
        text_color: palette.background.weaker.text,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}
