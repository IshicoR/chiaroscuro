//! Icon-only action buttons with a consistent tooltip.

use std::{borrow::Cow, fmt};

use iced::{
    Background, Border, Element, Length, Padding, Shadow, Theme,
    widget::{container, text, tooltip},
};

use crate::button::{Button, Size, Variant};

const TOOLTIP_RADIUS: f32 = 6.0;

/// A Chiaro icon-only action button builder.
///
/// The tooltip label is part of the component so icon actions are not left
/// without a textual description.
#[must_use]
pub struct IconButton<'a, Message> {
    button: Button<'a, Message>,
    label: Cow<'a, str>,
    tooltip_position: tooltip::Position,
}

impl<Message> fmt::Debug for IconButton<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IconButton")
            .field("button", &self.button)
            .field("label", &self.label)
            .field("tooltip_position", &self.tooltip_position)
            .finish()
    }
}

impl<'a, Message> IconButton<'a, Message> {
    /// Creates a disabled icon button with a tooltip label.
    pub fn new(icon: impl Into<Element<'a, Message>>, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            button: Button::new(icon).size(Size::Icon),
            label: label.into(),
            tooltip_position: tooltip::Position::Top,
        }
    }

    /// Sets the semantic visual role.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.button = self.button.variant(variant);
        self
    }

    /// Sets the icon button metrics.
    pub fn size(mut self, size: Size) -> Self {
        self.button = self.button.size(size);
        self
    }

    /// Overrides the default width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.button = self.button.width(width);
        self
    }

    /// Overrides the default height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.button = self.button.height(height);
        self
    }

    /// Overrides the default padding.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.button = self.button.padding(padding);
        self
    }

    /// Places the tooltip relative to the button.
    pub fn tooltip_position(mut self, position: tooltip::Position) -> Self {
        self.tooltip_position = position;
        self
    }

    /// Enables the button and publishes `message` when pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.button = self.button.on_press(message);
        self
    }

    /// Enables the button only when `message` is [`Some`].
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.button = self.button.on_press_maybe(message);
        self
    }
}

impl<'a, Message: Clone + 'a> IconButton<'a, Message> {
    /// Builds the button and tooltip.
    pub fn build(self) -> Element<'a, Message> {
        tooltip(
            self.button,
            container(text(self.label).size(12)).padding([4, 8]),
            self.tooltip_position,
        )
        .gap(4)
        .padding(0)
        .style(tooltip_style)
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<'a, Message>> for Element<'a, Message> {
    fn from(button: IconButton<'a, Message>) -> Self {
        button.build()
    }
}

/// Creates an icon-only button with a tooltip label.
pub fn icon_button<'a, Message>(
    icon: impl Into<Element<'a, Message>>,
    label: impl Into<Cow<'a, str>>,
) -> IconButton<'a, Message> {
    IconButton::new(icon, label)
}

/// Styles the shared compact tooltip used by icon actions.
///
/// This is public so non-button interaction targets, such as drag handles,
/// can present the exact same tooltip surface.
pub fn tooltip_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            radius: TOOLTIP_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        ..iced::widget::container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_a_top_tooltip() {
        let button = icon_button::<()>(text("x"), "Close");

        assert_eq!(button.label, "Close");
        assert_eq!(button.tooltip_position, tooltip::Position::Top);
    }

    #[test]
    fn tooltip_uses_a_compact_borderless_flat_surface() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = tooltip_style(&theme);

        assert_eq!(
            style.background,
            Some(Background::Color(palette.background.weak.color))
        );
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, TOOLTIP_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }
}
