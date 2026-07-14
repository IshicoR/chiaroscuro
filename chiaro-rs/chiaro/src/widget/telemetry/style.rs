use iced::{Border, Color, Theme, widget::container};
use iced_plot::{GridStyle, PlotStyle, default_style};

use crate::appearance::CARD_RADIUS;

pub(super) fn card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weakest.color.into()),
        text_color: Some(palette.background.weakest.text),
        border: Border {
            color: with_alpha(palette.background.strong.color, 0.55),
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

pub(super) fn plot(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let mut style = default_style(theme);

    style.frame.background = Some(Color::TRANSPARENT.into());
    style.plot_area.background = Some(palette.background.base.color.into());
    style.plot_area.border = Border {
        color: with_alpha(palette.background.strong.color, 0.45),
        width: 1.0,
        radius: 8.0.into(),
    };
    style.legend.background = Some(with_alpha(palette.background.base.color, 0.92).into());
    style.legend.border.radius = 7.0.into();
    style.tooltip.background = Some(with_alpha(palette.background.base.color, 0.94).into());
    style.tooltip.text_color = Some(palette.background.base.text);
    style.tooltip.border = Border {
        color: with_alpha(palette.primary.base.color, 0.5),
        width: 1.0,
        radius: 6.0.into(),
    };
    style.grid = GridStyle {
        major: with_alpha(palette.background.base.text, 0.16),
        minor: with_alpha(palette.background.base.text, 0.08),
        sub_minor: with_alpha(palette.background.base.text, 0.04),
    };
    style.tick_label_color = with_alpha(palette.background.base.text, 0.68);
    style.axis_label_color = with_alpha(palette.background.base.text, 0.82);

    style
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}
