//! Compact, non-interactive labels for values and statuses.

use std::fmt;

use iced::{
    Border, Color, Element, Font, Length, Shadow, Theme,
    alignment::{Horizontal, Vertical},
    border,
    theme::palette::{Pair, mix},
    widget::{
        Container as IcedContainer, Space, Stack, Text, container as iced_container, row, text,
    },
};

const HEIGHT: f32 = 24.0;
const LABEL_SIZE: u32 = 13;
const PADDING: [u16; 2] = [2, 8];
const CORNER_RADIUS: f32 = 6.0;
const METER_PORTIONS: u16 = 1_000;
const CENTER_PORTIONS: u16 = METER_PORTIONS / 2;
const CENTER_MARKER_WIDTH: f32 = 1.0;
const METER_FILL_TINT: f32 = 0.2;

/// The semantic visual role of a badge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Variant {
    /// A value without a semantic status, including unavailable values.
    #[default]
    Neutral,
    /// Informational or primary data.
    Primary,
    /// A positive or active value.
    Success,
    /// A destructive, braking, or error value.
    Danger,
}

/// A themed, non-interactive Chiaro badge builder.
#[must_use]
pub struct Badge<'a> {
    label: Text<'a>,
    variant: Variant,
    width: Option<Length>,
    font: Option<Font>,
    meter: Meter,
    meter_color: Option<Color>,
}

impl fmt::Debug for Badge<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Badge")
            .field("variant", &self.variant)
            .field("width", &self.width)
            .field("font", &self.font)
            .field("meter", &self.meter)
            .field("meter_color", &self.meter_color)
            .finish_non_exhaustive()
    }
}

impl<'a> Badge<'a> {
    /// Creates a neutral badge containing `label`.
    pub fn new(label: impl text::IntoFragment<'a>) -> Self {
        Self {
            label: text(label),
            variant: Variant::default(),
            width: None,
            font: None,
            meter: Meter::Solid,
            meter_color: None,
        }
    }

    /// Sets the semantic visual role.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Overrides the content-sized width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the label font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Fills the badge from left to right by a normalized `0.0..=1.0` value.
    pub fn progress(mut self, progress: f32) -> Self {
        self.meter = Meter::Linear(clamp(progress, 0.0, 1.0));
        self
    }

    /// Fills from the center toward the sign of a normalized `-1.0..=1.0` value.
    pub fn centered_progress(mut self, progress: f32) -> Self {
        self.meter = Meter::Centered(clamp(progress, -1.0, 1.0));
        self
    }

    /// Overrides the semantic color used by the progress fill.
    pub fn meter_color(mut self, color: Color) -> Self {
        self.meter_color = Some(color);
        self
    }

    /// Builds the underlying Iced container.
    pub fn build<Message: 'a>(self) -> IcedContainer<'a, Message> {
        let variant = self.variant;
        let meter = self.meter;
        let meter_color = self.meter_color;
        let width = self.width.unwrap_or(Length::Shrink);
        let mut label = self.label.size(LABEL_SIZE);

        if let Some(font) = self.font {
            label = label.font(font);
        }

        let label = iced_container(label)
            .width(width)
            .height(Length::Fixed(HEIGHT))
            .padding(PADDING)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        let mut layers = Stack::new().push(label);

        if let Some(fill) = meter_layer(meter, variant, meter_color) {
            layers = layers.push_under(fill);
        }

        iced_container(layers)
            .width(width)
            .height(Length::Fixed(HEIGHT))
            .clip(true)
            .style(move |theme| style(theme, variant, meter))
    }
}

impl<'a, Message: 'a> From<Badge<'a>> for Element<'a, Message> {
    fn from(badge: Badge<'a>) -> Self {
        badge.build().into()
    }
}

/// Creates a compact neutral badge containing `label`.
pub fn badge<'a>(label: impl text::IntoFragment<'a>) -> Badge<'a> {
    Badge::new(label)
}

fn pair(theme: &Theme, variant: Variant) -> Pair {
    let palette = theme.extended_palette();

    match variant {
        Variant::Neutral => palette.background.neutral,
        Variant::Primary => palette.primary.weak,
        Variant::Success => palette.success.weak,
        Variant::Danger => palette.danger.weak,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Meter {
    #[default]
    Solid,
    Linear(f32),
    Centered(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeterSegments {
    before: u16,
    fill: u16,
    after: u16,
    round_left: bool,
    round_right: bool,
}

fn clamp(value: f32, minimum: f32, maximum: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        0.0
    }
}

fn meter_segments(meter: Meter) -> Option<MeterSegments> {
    match meter {
        Meter::Solid => None,
        Meter::Linear(progress) => {
            let fill = portions(progress, METER_PORTIONS);
            (fill > 0).then_some(MeterSegments {
                before: 0,
                fill,
                after: METER_PORTIONS - fill,
                round_left: true,
                round_right: fill == METER_PORTIONS,
            })
        },
        Meter::Centered(progress) => {
            let fill = portions(progress.abs(), CENTER_PORTIONS);
            if fill == 0 {
                return None;
            }

            if progress.is_sign_positive() {
                Some(MeterSegments {
                    before: CENTER_PORTIONS,
                    fill,
                    after: CENTER_PORTIONS - fill,
                    round_left: false,
                    round_right: fill == CENTER_PORTIONS,
                })
            } else {
                Some(MeterSegments {
                    before: CENTER_PORTIONS - fill,
                    fill,
                    after: CENTER_PORTIONS,
                    round_left: fill == CENTER_PORTIONS,
                    round_right: false,
                })
            }
        },
    }
}

fn portions(progress: f32, total: u16) -> u16 {
    (progress * f32::from(total)).round() as u16
}

fn portion_length(portions: u16) -> Length {
    if portions == 0 {
        Length::Fixed(0.0)
    } else {
        Length::FillPortion(portions)
    }
}

fn meter_layer<'a, Message: 'a>(
    meter: Meter,
    variant: Variant,
    meter_color: Option<Color>,
) -> Option<Element<'a, Message>> {
    match meter {
        Meter::Solid => None,
        Meter::Linear(_) => {
            meter_segments(meter).map(|segments| meter_fill(segments, variant, meter_color))
        },
        Meter::Centered(_) => {
            let mut layers = Stack::new().width(Length::Fill).height(Length::Fill);
            if let Some(segments) = meter_segments(meter) {
                layers = layers.push(meter_fill(segments, variant, meter_color));
            }

            Some(layers.push(center_marker()).into())
        },
    }
}

fn meter_fill<'a, Message: 'a>(
    segments: MeterSegments,
    variant: Variant,
    meter_color: Option<Color>,
) -> Element<'a, Message> {
    let fill = iced_container(Space::new())
        .width(portion_length(segments.fill))
        .height(Length::Fill)
        .style(move |theme| {
            meter_fill_style(
                theme,
                variant,
                meter_color,
                segments.round_left,
                segments.round_right,
            )
        });

    row![
        Space::new().width(portion_length(segments.before)),
        fill,
        Space::new().width(portion_length(segments.after)),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn center_marker<'a, Message: 'a>() -> Element<'a, Message> {
    let marker = iced_container(Space::new())
        .width(Length::Fixed(CENTER_MARKER_WIDTH))
        .height(Length::Fill)
        .style(center_marker_style);

    row![
        Space::new().width(Length::Fill),
        marker,
        Space::new().width(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn style(theme: &Theme, variant: Variant, meter: Meter) -> iced_container::Style {
    let pair = match meter {
        Meter::Solid => pair(theme, variant),
        Meter::Linear(_) | Meter::Centered(_) => theme.extended_palette().background.neutral,
    };

    iced_container::Style {
        background: Some(pair.color.into()),
        text_color: Some(pair.text),
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        ..iced_container::Style::default()
    }
}

fn meter_fill_style(
    theme: &Theme,
    variant: Variant,
    meter_color: Option<Color>,
    round_left: bool,
    round_right: bool,
) -> iced_container::Style {
    let mut radius = border::Radius::default();
    if round_left {
        radius = radius.left(CORNER_RADIUS);
    }
    if round_right {
        radius = radius.right(CORNER_RADIUS);
    }

    iced_container::Style {
        background: Some(meter_fill_color(theme, variant, meter_color).into()),
        border: Border {
            radius,
            ..Border::default()
        },
        ..iced_container::Style::default()
    }
}

fn meter_fill_color(theme: &Theme, variant: Variant, meter_color: Option<Color>) -> Color {
    if let Some(color) = meter_color {
        return color;
    }

    let palette = theme.extended_palette();
    let accent = match variant {
        Variant::Neutral => palette.background.strong.color,
        Variant::Primary => palette.primary.base.color,
        Variant::Success => palette.success.base.color,
        Variant::Danger => palette.danger.base.color,
    };

    mix(palette.background.neutral.color, accent, METER_FILL_TINT)
}

fn center_marker_style(theme: &Theme) -> iced_container::Style {
    iced_container::Style::default().background(theme.extended_palette().background.strong.color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_a_compact_neutral_badge() {
        let badge = Badge::new("N/A");

        assert_eq!(badge.variant, Variant::Neutral);
        assert_eq!(badge.width, None);
        assert_eq!(badge.meter, Meter::Solid);
        assert_eq!(badge.meter_color, None);
        assert_eq!(HEIGHT, 24.0);
        assert_eq!(PADDING, [2, 8]);
        assert_eq!(CORNER_RADIUS, 6.0);
        assert_eq!(CENTER_MARKER_WIDTH, 1.0);
    }

    #[test]
    fn semantic_variants_use_weak_palette_pairs() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();

        assert_eq!(pair(&theme, Variant::Neutral), palette.background.neutral);
        assert_eq!(pair(&theme, Variant::Primary), palette.primary.weak);
        assert_eq!(pair(&theme, Variant::Success), palette.success.weak);
        assert_eq!(pair(&theme, Variant::Danger), palette.danger.weak);
    }

    #[test]
    fn every_variant_uses_readable_opaque_colors() {
        let theme = Theme::Dark;

        for variant in [
            Variant::Neutral,
            Variant::Primary,
            Variant::Success,
            Variant::Danger,
        ] {
            let pair = pair(&theme, variant);

            assert_eq!(pair.color.a, 1.0);
            assert_eq!(pair.text.a, 1.0);
            assert!(pair.text.is_readable_on(pair.color));
        }
    }

    #[test]
    fn badges_remain_flat_and_borderless() {
        let style = style(&Theme::Dark, Variant::Primary, Meter::Solid);

        assert!(style.background.is_some());
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CORNER_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn linear_progress_clamps_and_fills_from_the_left() {
        let low = Badge::new("0%").progress(-1.0);
        let partial = Badge::new("42.5%").progress(0.425);
        let high = Badge::new("100%").progress(2.0);

        assert_eq!(low.meter, Meter::Linear(0.0));
        assert_eq!(meter_segments(low.meter), None);
        assert_eq!(
            meter_segments(partial.meter),
            Some(MeterSegments {
                before: 0,
                fill: 425,
                after: 575,
                round_left: true,
                round_right: false,
            })
        );
        assert_eq!(high.meter, Meter::Linear(1.0));
        assert!(meter_segments(high.meter).is_some_and(|segments| segments.round_right));
    }

    #[test]
    fn centered_progress_changes_direction_at_zero() {
        let negative = Badge::new("-90°").centered_progress(-0.5);
        let zero = Badge::new("0°").centered_progress(0.0);
        let positive = Badge::new("90°").centered_progress(0.5);

        assert_eq!(
            meter_segments(negative.meter),
            Some(MeterSegments {
                before: 250,
                fill: 250,
                after: 500,
                round_left: false,
                round_right: false,
            })
        );
        assert_eq!(meter_segments(zero.meter), None);
        assert_eq!(
            meter_segments(positive.meter),
            Some(MeterSegments {
                before: 500,
                fill: 250,
                after: 250,
                round_left: false,
                round_right: false,
            })
        );
    }

    #[test]
    fn non_finite_progress_is_rendered_as_empty() {
        assert_eq!(
            Badge::new("N/A").progress(f32::NAN).meter,
            Meter::Linear(0.0)
        );
        assert_eq!(
            Badge::new("N/A").centered_progress(f32::INFINITY).meter,
            Meter::Centered(0.0)
        );
    }

    #[test]
    fn meter_uses_a_neutral_track_and_readable_text() {
        use iced::theme::Palette;

        let theme = Theme::custom(
            "Carbon Dark",
            Palette {
                background: Color::from_rgb8(0x16, 0x16, 0x16),
                text: Color::from_rgb8(0xF4, 0xF4, 0xF4),
                primary: Color::from_rgb8(0x45, 0x89, 0xFF),
                success: Color::from_rgb8(0x42, 0xBE, 0x65),
                warning: Color::from_rgb8(0xF1, 0xC2, 0x1B),
                danger: Color::from_rgb8(0xFA, 0x4D, 0x56),
            },
        );
        let palette = theme.extended_palette();
        let track = style(&theme, Variant::Success, Meter::Linear(0.5));

        assert_eq!(
            track.background,
            Some(palette.background.neutral.color.into())
        );
        assert_eq!(track.text_color, Some(palette.background.neutral.text));
        for variant in [Variant::Primary, Variant::Success, Variant::Danger] {
            let fill = meter_fill_color(&theme, variant, None);
            assert!(
                palette.background.neutral.text.is_readable_on(fill),
                "{variant:?} meter contrast is {}",
                fill.relative_contrast(palette.background.neutral.text),
            );
        }
    }

    #[test]
    fn custom_meter_color_is_used_without_tinting() {
        let color = Color::from_rgb(0.12, 0.72, 0.38);
        let badge = Badge::new("42.5%").progress(0.425).meter_color(color);

        assert_eq!(badge.meter_color, Some(color));
        assert_eq!(
            meter_fill_color(&Theme::Dark, Variant::Success, Some(color)),
            color
        );
    }
}
