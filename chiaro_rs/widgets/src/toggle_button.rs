//! Selectable buttons for binary and mutually exclusive choices.
//!
//! A toggle button keeps its selection state in the application model. The
//! caller supplies that state on every view and receives an ordinary message
//! when the button is pressed.

use std::fmt;

use iced::{
    Background, Border, Color, Element, Length, Padding, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Button as IcedButton, button as iced_button, container},
};

/// Carbon's compact control height.
const DEFAULT_SIZE: f32 = 32.0;
const CORNER_RADIUS: f32 = 6.0;

/// A themed Chiaro button that represents a selectable value.
///
/// The button is disabled until an `on_press` message is provided. Its
/// selection state is visual only; the caller remains responsible for updating
/// the corresponding application state.
#[must_use]
pub struct ToggleButton<'a, Message> {
    content: Element<'a, Message>,
    selected: bool,
    on_press: Option<Message>,
    width: Option<Length>,
    height: Option<Length>,
    padding: Option<Padding>,
}

impl<Message> fmt::Debug for ToggleButton<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ToggleButton")
            .field("selected", &self.selected)
            .field("enabled", &self.on_press.is_some())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> ToggleButton<'a, Message> {
    /// Creates a disabled toggle button with the supplied selection state.
    pub fn new(content: impl Into<Element<'a, Message>>, selected: bool) -> Self {
        Self {
            content: content.into(),
            selected,
            on_press: None,
            width: None,
            height: None,
            padding: None,
        }
    }

    /// Updates whether the button is visually selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Overrides the default square width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Overrides the default square height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Overrides the default zero padding.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Enables the button and publishes `message` when pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Enables the button only when `message` is [`Some`].
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub(crate) const fn is_selected(&self) -> bool {
        self.selected
    }
}

impl<'a, Message: Clone + 'a> ToggleButton<'a, Message> {
    /// Builds the underlying Iced button.
    pub fn build(self) -> IcedButton<'a, Message> {
        let selected = self.selected;
        let width = self.width.unwrap_or(Length::Fixed(DEFAULT_SIZE));
        let height = self.height.unwrap_or(Length::Fixed(DEFAULT_SIZE));
        let padding = self.padding.unwrap_or(Padding::ZERO);
        let content_width = if width == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        };
        let content_height = if height == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        };
        let content = container(self.content)
            .width(content_width)
            .height(content_height)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        iced_button(content)
            .width(width)
            .height(height)
            .padding(padding)
            .style(move |theme, status| style(theme, status, selected))
            .on_press_maybe(self.on_press)
    }
}

impl<'a, Message: Clone + 'a> From<ToggleButton<'a, Message>> for Element<'a, Message> {
    fn from(button: ToggleButton<'a, Message>) -> Self {
        button.build().into()
    }
}

/// Creates a toggle button with the supplied selection state.
pub fn toggle_button<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
) -> ToggleButton<'a, Message> {
    ToggleButton::new(content, selected)
}

fn style(theme: &Theme, status: iced_button::Status, selected: bool) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = if selected {
        match status {
            iced_button::Status::Active => (palette.primary.base.color, palette.primary.base.text),
            iced_button::Status::Hovered => {
                (palette.primary.strong.color, palette.primary.strong.text)
            },
            iced_button::Status::Pressed => (palette.primary.weak.color, palette.primary.weak.text),
            iced_button::Status::Disabled => (
                with_alpha(palette.primary.base.color, 0.35),
                with_alpha(palette.primary.base.text, 0.5),
            ),
        }
    } else {
        match status {
            iced_button::Status::Active => (
                palette.background.weaker.color,
                palette.background.weaker.text,
            ),
            iced_button::Status::Hovered => {
                (palette.background.weak.color, palette.background.weak.text)
            },
            iced_button::Status::Pressed => (
                palette.background.neutral.color,
                palette.background.neutral.text,
            ),
            iced_button::Status::Disabled => (
                with_alpha(palette.background.weaker.color, 0.55),
                with_alpha(palette.background.weaker.text, 0.45),
            ),
        }
    };

    iced_button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..iced_button::Style::default()
    }
}

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_square_and_preserve_selection() {
        let button = ToggleButton::<()>::new(iced::widget::text("Layout"), true);

        assert!(button.selected);
        assert_eq!(button.width, None);
        assert_eq!(button.height, None);
        assert_eq!(button.padding, None);
        assert!(button.on_press.is_none());
        assert_eq!(DEFAULT_SIZE, 32.0);
    }

    #[test]
    fn metrics_and_message_can_be_overridden() {
        let button = ToggleButton::<u8>::new(iced::widget::text("Layout"), false)
            .width(28)
            .height(30)
            .padding(6)
            .on_press_maybe(Some(7));

        assert_eq!(button.width, Some(Length::Fixed(28.0)));
        assert_eq!(button.height, Some(Length::Fixed(30.0)));
        assert_eq!(button.padding, Some(Padding::new(6.0)));
        assert_eq!(button.on_press, Some(7));
    }

    #[test]
    fn selected_uses_primary_colors() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let selected = style(&theme, iced_button::Status::Active, true);

        assert_eq!(
            selected.background,
            Some(Background::Color(palette.primary.base.color))
        );
        assert_eq!(selected.text_color, palette.primary.base.text);
        assert_eq!(selected.border.width, 0.0);
    }

    #[test]
    fn unselected_background_changes_on_hover() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let active = style(&theme, iced_button::Status::Active, false);
        let hovered = style(&theme, iced_button::Status::Hovered, false);

        assert_eq!(
            active.background,
            Some(Background::Color(palette.background.weaker.color))
        );
        assert_eq!(
            hovered.background,
            Some(Background::Color(palette.background.weak.color))
        );
        assert_eq!(active.border.width, 0.0);
        assert_eq!(active.border.radius, CORNER_RADIUS.into());
    }

    #[test]
    fn pressed_state_has_distinct_feedback() {
        let theme = Theme::Dark;

        for selected in [false, true] {
            let active = style(&theme, iced_button::Status::Active, selected);
            let pressed = style(&theme, iced_button::Status::Pressed, selected);

            assert_ne!(pressed.background, active.background);
            assert_eq!(pressed.border.width, 0.0);
        }
    }

    #[test]
    fn disabled_state_is_visually_muted() {
        let theme = Theme::Dark;

        for selected in [false, true] {
            let active = style(&theme, iced_button::Status::Active, selected);
            let disabled = style(&theme, iced_button::Status::Disabled, selected);

            assert!(disabled.text_color.a < active.text_color.a);
            assert_eq!(disabled.border.width, 0.0);
        }
    }
}
