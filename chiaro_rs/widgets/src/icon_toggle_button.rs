//! Icon-only selectable buttons with a consistent tooltip.

use std::{borrow::Cow, fmt};

use iced::{
    Background, Border, Element, Length, Padding, Shadow, Theme,
    widget::{container, text, tooltip},
};

use crate::toggle_button::ToggleButton;

const TOOLTIP_RADIUS: f32 = 6.0;

/// A Chiaro icon-only toggle button builder.
///
/// The caller owns the selection state and supplies it on every view. The
/// tooltip label is part of the component so the icon always has a textual
/// description.
#[must_use]
pub struct IconToggleButton<'a, Message> {
    button: ToggleButton<'a, Message>,
    label: Cow<'a, str>,
    tooltip_position: tooltip::Position,
}

impl<Message> fmt::Debug for IconToggleButton<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IconToggleButton")
            .field("button", &self.button)
            .field("selected", &self.button.is_selected())
            .field("label", &self.label)
            .field("tooltip_position", &self.tooltip_position)
            .finish()
    }
}

impl<'a, Message> IconToggleButton<'a, Message> {
    /// Creates a disabled icon toggle with a tooltip label.
    pub fn new(
        icon: impl Into<Element<'a, Message>>,
        label: impl Into<Cow<'a, str>>,
        selected: bool,
    ) -> Self {
        Self {
            button: ToggleButton::new(icon, selected),
            label: label.into(),
            tooltip_position: tooltip::Position::Top,
        }
    }

    /// Updates whether the button is visually selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.button = self.button.selected(selected);
        self
    }

    /// Overrides the default square width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.button = self.button.width(width);
        self
    }

    /// Overrides the default square height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.button = self.button.height(height);
        self
    }

    /// Overrides the default zero padding.
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

impl<'a, Message: Clone + 'a> IconToggleButton<'a, Message> {
    /// Builds the toggle button and tooltip.
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

impl<'a, Message: Clone + 'a> From<IconToggleButton<'a, Message>> for Element<'a, Message> {
    fn from(button: IconToggleButton<'a, Message>) -> Self {
        button.build()
    }
}

/// Creates an icon-only toggle button with a tooltip label.
pub fn icon_toggle_button<'a, Message>(
    icon: impl Into<Element<'a, Message>>,
    label: impl Into<Cow<'a, str>>,
    selected: bool,
) -> IconToggleButton<'a, Message> {
    IconToggleButton::new(icon, label, selected)
}

fn tooltip_style(theme: &Theme) -> iced::widget::container::Style {
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
    fn defaults_to_a_top_tooltip_and_preserves_selection() {
        let button = icon_toggle_button::<()>(text("x"), "Single column", true);

        assert_eq!(button.label, "Single column");
        assert_eq!(button.tooltip_position, tooltip::Position::Top);
        assert!(button.button.is_selected());
    }

    #[test]
    fn selection_and_tooltip_position_can_be_overridden() {
        let button = icon_toggle_button::<()>(text("x"), "Two columns", false)
            .selected(true)
            .tooltip_position(tooltip::Position::Bottom);

        assert!(button.button.is_selected());
        assert_eq!(button.tooltip_position, tooltip::Position::Bottom);
    }

    #[test]
    fn enabled_and_disabled_buttons_build() {
        let _: Element<'_, u8> = icon_toggle_button(text("x"), "Enabled", false)
            .on_press(1)
            .into();
        let _: Element<'_, u8> = icon_toggle_button(text("x"), "Disabled", false).into();
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
