//! Carbon-inspired styles for the standard Iced form controls.
//!
//! Iced's [`checkbox`] and [`toggler`] widgets are still useful for simple
//! settings and display controls. Keeping their style functions here makes
//! their interaction layers consistent with the rest of Chiaro's widgets.

use iced::{
    Background, Border, Color, Theme,
    widget::{checkbox, toggler},
};

const CHECKBOX_RADIUS: f32 = 4.0;

/// Styles a standard Iced checkbox using Chiaro's Carbon Gray 100 layers.
pub fn checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let palette = theme.extended_palette();
    let (is_checked, is_hovered, is_disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };

    let background = if is_disabled {
        palette.background.weak.color
    } else if is_checked {
        if is_hovered {
            palette.primary.strong.color
        } else {
            palette.primary.base.color
        }
    } else if is_hovered {
        palette.background.weak.color
    } else {
        // An unchecked control still needs a visible hit target now that the
        // flat style does not use an outline.
        palette.background.weaker.color
    };

    checkbox::Style {
        background: Background::Color(background),
        icon_color: if is_disabled {
            with_alpha(palette.background.base.text, 0.45)
        } else {
            palette.primary.base.text
        },
        border: Border {
            radius: CHECKBOX_RADIUS.into(),
            ..Border::default()
        },
        text_color: Some(if is_disabled {
            with_alpha(palette.background.base.text, 0.45)
        } else {
            palette.background.base.text
        }),
    }
}

/// Styles a standard Iced toggler using Chiaro's Carbon Gray 100 layers.
pub fn toggler_style(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let palette = theme.extended_palette();
    let (is_toggled, is_hovered, is_disabled) = match status {
        toggler::Status::Active { is_toggled } => (is_toggled, false, false),
        toggler::Status::Hovered { is_toggled } => (is_toggled, true, false),
        toggler::Status::Disabled { is_toggled } => (is_toggled, false, true),
    };
    let background = if is_disabled {
        palette.background.weak.color
    } else if is_toggled {
        if is_hovered {
            palette.primary.strong.color
        } else {
            palette.primary.base.color
        }
    } else if is_hovered {
        palette.background.strong.color
    } else {
        palette.background.neutral.color
    };
    let foreground = if is_disabled {
        palette.background.weaker.color
    } else {
        palette.background.base.text
    };

    toggler::Style {
        background: Background::Color(background),
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        foreground: Background::Color(foreground),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(if is_disabled {
            with_alpha(palette.background.base.text, 0.45)
        } else {
            palette.background.base.text
        }),
        // Switches retain a pill thumb and track while checkboxes use the
        // tighter radius from the rest of the form-control family.
        border_radius: None,
        padding_ratio: 0.12,
    }
}

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use iced::widget::{checkbox, toggler};

    use super::*;

    #[test]
    fn checked_checkbox_uses_the_primary_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = checkbox_style(&theme, checkbox::Status::Active { is_checked: true });

        assert_eq!(
            style.background,
            Background::Color(palette.primary.base.color)
        );
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CHECKBOX_RADIUS.into());
    }

    #[test]
    fn toggled_switch_uses_the_primary_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = toggler_style(&theme, toggler::Status::Active { is_toggled: true });

        assert_eq!(
            style.background,
            Background::Color(palette.primary.base.color)
        );
        assert_eq!(style.background_border_width, 0.0);
        assert_eq!(style.background_border_color, Color::TRANSPARENT);
        assert_eq!(style.padding_ratio, 0.12);
    }

    #[test]
    fn unchecked_checkbox_uses_a_filled_layer_without_a_border() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = checkbox_style(&theme, checkbox::Status::Active { is_checked: false });

        assert_eq!(
            style.background,
            Background::Color(palette.background.weaker.color)
        );
        assert_eq!(style.border.width, 0.0);
    }

    #[test]
    fn disabled_form_controls_reduce_their_text_contrast() {
        let theme = Theme::Dark;
        let checkbox = checkbox_style(&theme, checkbox::Status::Disabled { is_checked: false });
        let toggler = toggler_style(&theme, toggler::Status::Disabled { is_toggled: false });

        assert_eq!(checkbox.text_color.unwrap().a, 0.45);
        assert_eq!(toggler.text_color.unwrap().a, 0.45);
    }
}
