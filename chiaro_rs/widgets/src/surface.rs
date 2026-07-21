use iced::{Background, Border, Color, Theme, theme::Style, widget::container};

const WINDOW_CORNER_RADIUS: f32 = 10.0;
const CARD_CONTENT_RADIUS: f32 = 8.0;

pub fn application(theme: &Theme) -> Style {
    Style {
        // The native window is transparent so its rounded corners can show
        // through. The application content below owns the first Carbon layer.
        background_color: Color::TRANSPARENT,
        text_color: theme.extended_palette().background.base.text,
    }
}

pub fn content(theme: &Theme, rounded: bool) -> container::Style {
    let palette = theme.extended_palette();
    let colors = palette.background.base;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
        .border(Border {
            // Preserve native window clipping without drawing an outer frame.
            radius: iced::border::Radius {
                // This is an internal corner between the navigation rail and
                // the workspace, so it remains rounded when maximized.
                top_left: WINDOW_CORNER_RADIUS,
                bottom_right: if rounded { WINDOW_CORNER_RADIUS } else { 0.0 },
                ..iced::border::Radius::default()
            },
            ..Border::default()
        })
}

/// Paints the recessed content area shared by chart and data cards.
pub fn card_content(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette().background.base;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
        .border(Border {
            radius: CARD_CONTENT_RADIUS.into(),
            ..Border::default()
        })
}

/// Paints the layer exposed behind rounded navigation and content corners.
pub fn workspace(theme: &Theme, rounded: bool) -> container::Style {
    let palette = theme.extended_palette();
    let colors = palette.background.weaker;

    container::Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
        .border(Border {
            radius: if rounded {
                iced::border::Radius {
                    bottom_left: WINDOW_CORNER_RADIUS,
                    bottom_right: WINDOW_CORNER_RADIUS,
                    ..iced::border::Radius::default()
                }
            } else {
                iced::border::Radius::default()
            },
            ..Border::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_keeps_transparent_window_corners() {
        assert_eq!(
            application(&Theme::Dark).background_color,
            Color::TRANSPARENT
        );
    }

    #[test]
    fn rounded_content_keeps_clipping_without_an_outer_frame() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = content(&theme, true);

        assert_eq!(
            style.background,
            Some(Background::Color(palette.background.base.color))
        );
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.radius.top_left, WINDOW_CORNER_RADIUS);
        assert_eq!(style.border.radius.bottom_right, WINDOW_CORNER_RADIUS);
    }

    #[test]
    fn maximized_content_keeps_only_its_internal_top_left_corner() {
        let style = content(&Theme::Dark, false);

        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.radius.top_left, WINDOW_CORNER_RADIUS);
        assert_eq!(style.border.radius.bottom_right, 0.0);
    }

    #[test]
    fn workspace_uses_the_navigation_layer_behind_rounded_corners() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = workspace(&theme, true);

        assert_eq!(
            style.background,
            Some(Background::Color(palette.background.weaker.color))
        );
        assert_eq!(style.border.radius.bottom_left, WINDOW_CORNER_RADIUS);
        assert_eq!(style.border.radius.bottom_right, WINDOW_CORNER_RADIUS);
    }

    #[test]
    fn card_content_uses_the_chart_surface_without_a_frame() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = card_content(&theme);

        assert_eq!(
            style.background,
            Some(Background::Color(palette.background.base.color))
        );
        assert_eq!(style.text_color, Some(palette.background.base.text));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CARD_CONTENT_RADIUS.into());
    }
}
