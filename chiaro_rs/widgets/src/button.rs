//! Shared action buttons.
//!
//! A button has two independent design decisions:
//! [`Variant`] describes its semantic emphasis and [`Size`] describes its
//! layout metrics. The content remains an arbitrary [`Element`], so labels,
//! icons, and loading indicators can be composed by the caller.

use std::fmt;

use iced::{
    Background, Border, Color, Element, Length, Padding, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Button as IcedButton, button as iced_button, container},
};

// A moderate radius softens compact desktop controls while keeping adjacent
// actions visually distinct and easy to scan.
const CORNER_RADIUS: f32 = 6.0;

/// The semantic visual role of a button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Variant {
    /// The primary action on a surface.
    #[default]
    Primary,
    /// A complementary action with less emphasis.
    Secondary,
    /// A low-emphasis action rendered on a subtle surface layer.
    Outline,
    /// A low-emphasis action that becomes visible on interaction.
    Ghost,
    /// An irreversible or destructive action.
    Destructive,
}

/// The layout metrics of a button.
///
/// Icon sizes are square by default. Calling [`Button::width`] explicitly
/// overrides the default width, including for icon buttons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Size {
    ExtraSmall,
    Small,
    #[default]
    Medium,
    Large,
    IconExtraSmall,
    IconSmall,
    Icon,
    IconLarge,
}

impl Size {
    const fn height(self) -> f32 {
        match self {
            Self::ExtraSmall => 24.0,
            Self::Small => 28.0,
            Self::Medium => 32.0,
            Self::Large => 40.0,
            Self::IconExtraSmall => 24.0,
            Self::IconSmall => 28.0,
            Self::Icon => 32.0,
            Self::IconLarge => 40.0,
        }
    }

    const fn padding(self) -> [u16; 2] {
        match self {
            Self::ExtraSmall => [2, 8],
            Self::Small => [4, 12],
            Self::Medium => [6, 16],
            Self::Large => [8, 20],
            Self::IconExtraSmall | Self::IconSmall | Self::Icon | Self::IconLarge => [0, 0],
        }
    }

    const fn is_icon(self) -> bool {
        matches!(
            self,
            Self::IconExtraSmall | Self::IconSmall | Self::Icon | Self::IconLarge
        )
    }

    fn default_width(self, height: Length) -> Length {
        if self.is_icon() {
            match height {
                Length::Fixed(height) => Length::Fixed(height),
                _ => Length::Fixed(self.height()),
            }
        } else {
            Length::Shrink
        }
    }
}

/// A themed Chiaro button builder.
///
/// Buttons are disabled until an `on_press` message is provided. Use
/// [`Button::on_press_maybe`] when enabled state is conditional.
#[must_use]
pub struct Button<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    variant: Variant,
    size: Size,
    width: Option<Length>,
    height: Option<Length>,
    padding: Option<Padding>,
}

impl<Message> fmt::Debug for Button<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Button")
            .field("enabled", &self.on_press.is_some())
            .field("variant", &self.variant)
            .field("size", &self.size)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Button<'a, Message> {
    /// Creates a disabled button with the default variant and size.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            variant: Variant::default(),
            size: Size::default(),
            width: None,
            height: None,
            padding: None,
        }
    }

    /// Sets the semantic visual role.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the layout metrics.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Overrides the default width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Overrides the height defined by [`Size`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Overrides the padding defined by [`Size`].
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
}

impl<'a, Message: Clone + 'a> Button<'a, Message> {
    /// Builds the underlying Iced button.
    pub fn build(self) -> IcedButton<'a, Message> {
        let variant = self.variant;
        let size = self.size;
        let height = self.height.unwrap_or_else(|| Length::Fixed(size.height()));
        let width = self.width.unwrap_or_else(|| size.default_width(height));
        let padding = self.padding.unwrap_or_else(|| size.padding().into());
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
            .style(move |theme, status| style(theme, status, variant))
            .on_press_maybe(self.on_press)
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(button: Button<'a, Message>) -> Self {
        button.build().into()
    }
}

/// Creates a button with Chiaro's default variant and size.
pub fn button<'a, Message>(content: impl Into<Element<'a, Message>>) -> Button<'a, Message> {
    Button::new(content)
}

fn style(theme: &Theme, status: iced_button::Status, variant: Variant) -> iced_button::Style {
    match variant {
        Variant::Primary => primary_style(theme, status),
        Variant::Secondary => secondary_style(theme, status),
        Variant::Outline => outline_style(theme, status),
        Variant::Ghost => ghost_style(theme, status),
        Variant::Destructive => destructive_style(theme, status),
    }
}

fn primary_style(theme: &Theme, status: iced_button::Status) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
        iced_button::Status::Active => (palette.primary.base.color, palette.primary.base.text),
        iced_button::Status::Hovered => (palette.primary.strong.color, palette.primary.strong.text),
        iced_button::Status::Pressed => (palette.primary.weak.color, palette.primary.weak.text),
        iced_button::Status::Disabled => (
            with_alpha(palette.primary.base.color, 0.35),
            with_alpha(palette.primary.base.text, 0.5),
        ),
    };

    filled_style(background, text_color)
}

fn destructive_style(theme: &Theme, status: iced_button::Status) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
        iced_button::Status::Active => (palette.danger.base.color, palette.danger.base.text),
        iced_button::Status::Hovered => (palette.danger.strong.color, palette.danger.strong.text),
        iced_button::Status::Pressed => (palette.danger.weak.color, palette.danger.weak.text),
        iced_button::Status::Disabled => (
            with_alpha(palette.danger.base.color, 0.35),
            with_alpha(palette.danger.base.text, 0.5),
        ),
    };

    filled_style(background, text_color)
}

fn secondary_style(theme: &Theme, status: iced_button::Status) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
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
    };

    filled_style(background, text_color)
}

fn filled_style(background: Color, text_color: Color) -> iced_button::Style {
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

fn outline_style(theme: &Theme, status: iced_button::Status) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
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
            with_alpha(palette.background.weaker.color, 0.4),
            with_alpha(palette.background.base.text, 0.4),
        ),
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

fn ghost_style(theme: &Theme, status: iced_button::Status) -> iced_button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
        iced_button::Status::Active => (None, palette.background.base.text),
        iced_button::Status::Hovered => (
            Some(palette.background.weaker.color),
            palette.background.weaker.text,
        ),
        iced_button::Status::Pressed => (
            Some(palette.background.weak.color),
            palette.background.weak.text,
        ),
        iced_button::Status::Disabled => (None, with_alpha(palette.background.base.text, 0.4)),
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

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_primary_and_medium() {
        assert_eq!(Variant::default(), Variant::Primary);
        assert_eq!(Size::default(), Size::Medium);
    }

    #[test]
    fn labeled_sizes_have_explicit_heights_and_padding() {
        assert_eq!(Size::ExtraSmall.height(), 24.0);
        assert_eq!(Size::ExtraSmall.padding(), [2, 8]);
        assert_eq!(Size::Small.height(), 28.0);
        assert_eq!(Size::Medium.height(), 32.0);
        assert_eq!(Size::Large.height(), 40.0);
        assert!(!Size::Medium.is_icon());
    }

    #[test]
    fn icon_sizes_are_square_and_have_no_padding() {
        for size in [
            Size::IconExtraSmall,
            Size::IconSmall,
            Size::Icon,
            Size::IconLarge,
        ] {
            assert!(size.is_icon());
            assert_eq!(size.padding(), [0, 0]);
            assert_eq!(
                size.default_width(Length::Fixed(size.height())),
                Length::Fixed(size.height())
            );
        }
    }

    #[test]
    fn icon_width_tracks_an_explicit_fixed_height() {
        assert_eq!(
            Size::Icon.default_width(Length::Fixed(38.0)),
            Length::Fixed(38.0)
        );
    }

    #[test]
    fn outline_uses_layers_without_a_border() {
        let theme = Theme::Dark;
        let active = outline_style(&theme, iced_button::Status::Active);
        let disabled = outline_style(&theme, iced_button::Status::Disabled);

        assert!(active.background.is_some());
        assert_eq!(active.border.width, 0.0);
        assert!(disabled.text_color.a < active.text_color.a);
    }

    #[test]
    fn secondary_uses_a_layered_surface() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let active = secondary_style(&theme, iced_button::Status::Active);
        let hovered = secondary_style(&theme, iced_button::Status::Hovered);

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
    fn primary_and_destructive_variants_use_their_semantic_colors() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();

        assert_eq!(
            primary_style(&theme, iced_button::Status::Active).background,
            Some(Background::Color(palette.primary.base.color))
        );
        assert_eq!(
            destructive_style(&theme, iced_button::Status::Active).background,
            Some(Background::Color(palette.danger.base.color))
        );
    }

    #[test]
    fn ghost_background_only_appears_during_interaction() {
        let theme = Theme::Dark;

        assert!(
            ghost_style(&theme, iced_button::Status::Active)
                .background
                .is_none()
        );
        assert!(
            ghost_style(&theme, iced_button::Status::Hovered)
                .background
                .is_some()
        );
        assert!(
            ghost_style(&theme, iced_button::Status::Pressed)
                .background
                .is_some()
        );
        assert!(
            ghost_style(&theme, iced_button::Status::Disabled)
                .background
                .is_none()
        );
    }
}
