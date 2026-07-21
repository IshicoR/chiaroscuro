//! Shared colors for quiet selected rows and navigation destinations.

use iced::{Color, Theme, widget::button};

const ACTIVE_ALPHA: f32 = 0.18;
const HOVERED_ALPHA: f32 = 0.26;
const PRESSED_ALPHA: f32 = 0.34;
const DISABLED_ALPHA: f32 = 0.08;
const DISABLED_FOREGROUND_ALPHA: f32 = 0.4;

/// The background layer and foreground used by a quiet selected item.
///
/// The primary palette foreground is intended for an opaque primary surface.
/// Quiet selections only tint the dark canvas, so they retain the canvas
/// foreground to keep labels and icons readable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuietSelection {
    pub background: Color,
    pub foreground: Color,
}

/// Returns the shared selected appearance for full-width rows and destinations.
pub fn quiet_selection(theme: &Theme, status: button::Status) -> QuietSelection {
    let palette = theme.extended_palette();
    let (background_alpha, foreground) = match status {
        button::Status::Active => (ACTIVE_ALPHA, palette.background.base.text),
        button::Status::Hovered => (HOVERED_ALPHA, palette.background.base.text),
        button::Status::Pressed => (PRESSED_ALPHA, palette.background.base.text),
        button::Status::Disabled => (
            DISABLED_ALPHA,
            with_alpha(palette.background.base.text, DISABLED_FOREGROUND_ALPHA),
        ),
    };

    QuietSelection {
        background: with_alpha(palette.primary.base.color, background_alpha),
        foreground,
    }
}

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use iced::{
        Color, Theme,
        theme::{Palette, palette::mix},
        widget::button,
    };

    use super::*;

    const CARBON_DARK: Palette = Palette {
        background: Color::from_rgb8(0x16, 0x16, 0x16),
        text: Color::from_rgb8(0xF4, 0xF4, 0xF4),
        primary: Color::from_rgb8(0x45, 0x89, 0xFF),
        success: Color::from_rgb8(0x42, 0xBE, 0x65),
        warning: Color::from_rgb8(0xF1, 0xC2, 0x1B),
        danger: Color::from_rgb8(0xFA, 0x4D, 0x56),
    };

    #[test]
    fn tinted_primary_layer_keeps_the_dark_canvas_foreground() {
        let theme = Theme::custom("Carbon Dark test", CARBON_DARK);
        let palette = theme.extended_palette();
        let selected = quiet_selection(&theme, button::Status::Active);

        assert_eq!(
            selected.background,
            with_alpha(palette.primary.base.color, ACTIVE_ALPHA)
        );
        assert_eq!(selected.foreground, palette.background.base.text);
        assert_ne!(selected.foreground, palette.primary.base.text);
    }

    #[test]
    fn interaction_strengthens_the_tint_and_disabled_mutes_the_foreground() {
        let theme = Theme::custom("Carbon Dark test", CARBON_DARK);
        let active = quiet_selection(&theme, button::Status::Active);
        let hovered = quiet_selection(&theme, button::Status::Hovered);
        let pressed = quiet_selection(&theme, button::Status::Pressed);
        let disabled = quiet_selection(&theme, button::Status::Disabled);

        assert!(hovered.background.a > active.background.a);
        assert!(pressed.background.a > hovered.background.a);
        assert!(disabled.foreground.a < active.foreground.a);
    }

    #[test]
    fn interactive_foregrounds_remain_readable_after_the_tint_is_composited() {
        let theme = Theme::custom("Carbon Dark test", CARBON_DARK);
        let palette = theme.extended_palette();

        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
        ] {
            let selected = quiet_selection(&theme, status);
            let composited_background = mix(
                palette.background.weaker.color,
                palette.primary.base.color,
                selected.background.a,
            );

            assert!(selected.foreground.relative_contrast(composited_background) >= 6.0);
        }
    }
}
