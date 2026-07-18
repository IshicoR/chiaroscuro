use iced::{Border, Color, Shadow, Theme, Vector, widget::container};

const CARD_RADIUS: f32 = 10.0;

pub(super) fn card(theme: &Theme, highlighted: bool, lift: f32) -> container::Style {
    let palette = theme.extended_palette();
    let lift = lift.clamp(0.0, 1.0);

    container::Style {
        background: Some(palette.background.weakest.color.into()),
        text_color: Some(palette.background.weakest.text),
        border: Border {
            color: if highlighted {
                with_alpha(palette.primary.base.color, 0.9)
            } else {
                with_alpha(palette.background.strong.color, 0.55)
            },
            width: if highlighted { 2.0 } else { 1.0 },
            radius: CARD_RADIUS.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.24 * lift,
                ..Color::BLACK
            },
            offset: Vector::new(0.0, 5.0 * lift),
            blur_radius: 14.0 * lift,
        },
        ..container::Style::default()
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}
