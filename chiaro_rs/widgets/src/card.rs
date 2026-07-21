//! Shared card-like surfaces.
//!
//! [`Variant`] selects the visual hierarchy of the surface, while [`Card`]
//! owns its layout metrics. The content remains an arbitrary [`Element`], so
//! callers can compose the surface without coupling it to a particular screen.

use std::fmt;

use iced::{
    Border, Element, Length, Padding, Shadow, Theme,
    theme::palette::mix,
    widget::{Container as IcedContainer, container as iced_container},
};

// Cards use the broadest radius in the shared surface hierarchy. Nested
// panels and callouts remain slightly tighter so the layout stays compact.
const CARD_RADIUS: f32 = 10.0;
const PANEL_RADIUS: f32 = 8.0;
const CALLOUT_RADIUS: f32 = 8.0;
const HIGHLIGHT_TINT: f32 = 0.16;

/// The visual hierarchy of a card-like surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Variant {
    /// A standalone card, optionally highlighted during interaction.
    #[default]
    Card,
    /// A compact section within a larger surface.
    Panel,
    /// A lower-level explanatory or status surface.
    Callout,
}

/// A themed Chiaro card builder.
#[must_use]
pub struct Card<'a, Message> {
    content: Element<'a, Message>,
    variant: Variant,
    width: Option<Length>,
    height: Option<Length>,
    padding: Option<Padding>,
    highlighted: bool,
    lift: f32,
}

impl<Message> fmt::Debug for Card<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Card")
            .field("variant", &self.variant)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .field("highlighted", &self.highlighted)
            .field("lift", &self.lift)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Card<'a, Message> {
    /// Creates a card containing `content`.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            variant: Variant::default(),
            width: None,
            height: None,
            padding: None,
            highlighted: false,
            lift: 0.0,
        }
    }

    /// Sets the visual hierarchy of the surface.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the width of the surface.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the height of the surface.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Sets the inner padding of the surface.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Highlights a card as a current interaction target.
    ///
    /// This setting only affects the [`Variant::Card`] appearance.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// Retains the normalized lift state supplied by interaction code.
    ///
    /// The fully flat visual style intentionally does not render elevation.
    /// The value remains part of the API so existing drag-state callers do
    /// not need special handling.
    pub fn lift(mut self, lift: f32) -> Self {
        self.lift = lift;
        self
    }

    /// Builds the underlying Iced container.
    pub fn build(self) -> IcedContainer<'a, Message> {
        let variant = self.variant;
        let highlighted = self.highlighted;
        let lift = self.lift;
        let mut container = iced_container(self.content)
            .style(move |theme| style(theme, variant, highlighted, lift));

        if let Some(width) = self.width {
            container = container.width(width);
        }

        if let Some(height) = self.height {
            container = container.height(height);
        }

        if let Some(padding) = self.padding {
            container = container.padding(padding);
        }

        container
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        card.build().into()
    }
}

/// Creates a standalone card.
pub fn card<'a, Message>(content: impl Into<Element<'a, Message>>) -> Card<'a, Message> {
    Card::new(content)
}

/// Creates a compact panel.
pub fn panel<'a, Message>(content: impl Into<Element<'a, Message>>) -> Card<'a, Message> {
    Card::new(content).variant(Variant::Panel)
}

/// Creates an explanatory or status callout.
pub fn callout<'a, Message>(content: impl Into<Element<'a, Message>>) -> Card<'a, Message> {
    Card::new(content).variant(Variant::Callout)
}

fn style(theme: &Theme, variant: Variant, highlighted: bool, _lift: f32) -> iced_container::Style {
    let palette = theme.extended_palette();

    match variant {
        Variant::Card => {
            let background = if highlighted {
                mix(
                    palette.background.weaker.color,
                    palette.primary.base.color,
                    HIGHLIGHT_TINT,
                )
            } else {
                palette.background.weaker.color
            };

            iced_container::Style {
                // Base is the app canvas; cards are Carbon layer 01. An
                // interaction target uses a quiet primary tint instead of a
                // frame so the surface remains fully flat.
                background: Some(background.into()),
                text_color: Some(palette.background.weaker.text),
                border: Border {
                    radius: CARD_RADIUS.into(),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                ..iced_container::Style::default()
            }
        },
        Variant::Panel => iced_container::Style {
            // Nested content moves one layer above its containing card.
            background: Some(palette.background.weak.color.into()),
            text_color: Some(palette.background.weak.text),
            border: Border {
                radius: PANEL_RADIUS.into(),
                ..Border::default()
            },
            ..iced_container::Style::default()
        },
        Variant::Callout => iced_container::Style {
            // Callouts sit above the surrounding form layer without relying
            // on a heavy shadow or saturated status color.
            background: Some(palette.background.neutral.color.into()),
            text_color: Some(palette.background.neutral.text),
            border: Border {
                radius: CALLOUT_RADIUS.into(),
                ..Border::default()
            },
            ..iced_container::Style::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_uses_the_first_carbon_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = style(&theme, Variant::Card, false, 0.0);

        assert_eq!(
            style.background,
            Some(palette.background.weaker.color.into())
        );
        assert_eq!(style.text_color, Some(palette.background.weaker.text));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CARD_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn highlighted_card_uses_a_primary_tint_without_a_border_or_shadow() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = style(&theme, Variant::Card, true, 1.0);

        assert_eq!(
            style.background,
            Some(
                mix(
                    palette.background.weaker.color,
                    palette.primary.base.color,
                    HIGHLIGHT_TINT,
                )
                .into()
            )
        );
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CARD_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn lift_remains_flat_for_all_values() {
        let below = style(&Theme::Dark, Variant::Card, false, -1.0);
        let above = style(&Theme::Dark, Variant::Card, false, 2.0);

        assert_eq!(below.shadow, Shadow::default());
        assert_eq!(above.shadow, Shadow::default());
    }

    #[test]
    fn panel_uses_the_second_carbon_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = style(&theme, Variant::Panel, true, 1.0);

        assert_eq!(style.background, Some(palette.background.weak.color.into()));
        assert_eq!(style.text_color, Some(palette.background.weak.text));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, PANEL_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn callout_uses_a_distinct_flat_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = style(&theme, Variant::Callout, false, 0.0);

        assert_eq!(
            style.background,
            Some(palette.background.neutral.color.into())
        );
        assert_eq!(style.text_color, Some(palette.background.neutral.text));
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CALLOUT_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }
}
