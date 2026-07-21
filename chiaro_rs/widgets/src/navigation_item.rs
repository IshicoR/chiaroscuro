//! An icon-only navigation destination used in compact application rails.

use std::{borrow::Cow, fmt};

use iced::{
    Background, Border, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{button as iced_button, container, text, tooltip},
};

use crate::{icon_tooltip_style, quiet_selection};

const SIZE: f32 = 40.0;
const TOOLTIP_GAP: f32 = 6.0;
const TOOLTIP_TEXT_SIZE: u32 = 12;
const CORNER_RADIUS: f32 = 6.0;

/// A compact, full-width navigation destination.
#[must_use]
pub struct NavigationItem<'a, Message> {
    icon: Element<'a, Message>,
    label: Cow<'a, str>,
    selected: bool,
    on_press: Option<Message>,
}

impl<Message> fmt::Debug for NavigationItem<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NavigationItem")
            .field("selected", &self.selected)
            .field("enabled", &self.on_press.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a, Message> NavigationItem<'a, Message> {
    /// Creates a disabled navigation item from an icon and label.
    pub fn new(icon: impl Into<Element<'a, Message>>, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            selected: false,
            on_press: None,
        }
    }

    /// Marks whether this destination is currently selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Enables the item and publishes `message` when pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Enables the item only when `message` is [`Some`].
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, Message: Clone + 'a> NavigationItem<'a, Message> {
    /// Builds the icon button and its right-side tooltip.
    pub fn build(self) -> Element<'a, Message> {
        let selected = self.selected;
        let icon = container(self.icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        let button = iced_button(icon)
            .width(Length::Fill)
            .height(Length::Fixed(SIZE))
            .padding(0)
            .on_press_maybe(self.on_press)
            .style(move |theme, status| style(theme, status, selected))
            .clip(true);

        tooltip(
            button,
            container(text(self.label).size(TOOLTIP_TEXT_SIZE)).padding([4, 8]),
            tooltip::Position::Right,
        )
        .gap(TOOLTIP_GAP)
        .padding(0)
        .style(icon_tooltip_style)
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<NavigationItem<'a, Message>> for Element<'a, Message> {
    fn from(item: NavigationItem<'a, Message>) -> Self {
        item.build()
    }
}

/// Creates a compact navigation item from an icon and label.
pub fn navigation_item<'a, Message>(
    icon: impl Into<Element<'a, Message>>,
    label: impl Into<Cow<'a, str>>,
) -> NavigationItem<'a, Message> {
    NavigationItem::new(icon, label)
}

fn style(theme: &Theme, status: iced_button::Status, selected: bool) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = if selected {
        let selected = quiet_selection(theme, status);
        (Some(selected.background), selected.foreground)
    } else {
        match status {
            iced_button::Status::Active => (None, with_alpha(palette.background.base.text, 0.78)),
            iced_button::Status::Hovered => (
                Some(palette.background.weak.color),
                palette.background.weak.text,
            ),
            iced_button::Status::Pressed => (
                Some(palette.background.neutral.color),
                palette.background.neutral.text,
            ),
            iced_button::Status::Disabled => (None, with_alpha(palette.background.base.text, 0.35)),
        }
    };

    iced_button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..iced_button::Style::default()
    }
}

const fn with_alpha(color: iced::Color, alpha: f32) -> iced::Color {
    iced::Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_navigation_uses_a_40_pixel_square_target() {
        assert_eq!(SIZE, 40.0);
        assert_eq!(CORNER_RADIUS, 6.0);
        assert_eq!(TOOLTIP_GAP, 6.0);
    }

    #[test]
    fn navigation_labels_are_kept_for_the_right_side_tooltip() {
        let item = navigation_item::<()>(text("icon"), "Dashboard");

        assert_eq!(item.label, "Dashboard");
    }

    #[test]
    fn selected_item_uses_a_muted_primary_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = style(&theme, iced_button::Status::Active, true);

        assert_eq!(
            style.background,
            Some(Background::Color(
                quiet_selection(&theme, iced_button::Status::Active).background
            ))
        );
        assert_eq!(style.text_color, palette.background.base.text);
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, CORNER_RADIUS.into());
    }

    #[test]
    fn unselected_item_only_creates_a_layer_on_interaction() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let active = style(&theme, iced_button::Status::Active, false);
        let hovered = style(&theme, iced_button::Status::Hovered, false);

        assert!(active.background.is_none());
        assert_eq!(
            hovered.background,
            Some(Background::Color(palette.background.weak.color))
        );
    }
}
