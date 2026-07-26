//! Modal dialogs with a controlled open state.
//!
//! [`Dialog`] keeps the application content mounted behind a modal surface,
//! while preventing pointer, keyboard, touch, and input-method events from
//! reaching it. The application remains responsible for the `open` state and
//! for handling the message published by each dismissal path.

use std::{borrow::Cow, fmt};

use chiaro_i18n::{Text, tr};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    alignment::{Horizontal, Vertical},
    keyboard::{self, key},
    touch,
    widget::{Column, Row, container, text},
};
use iced_fonts::lucide;

use crate::{
    button::{Size as ButtonSize, Variant as ButtonVariant},
    icon_button::icon_button,
    typography,
};

const DEFAULT_WIDTH: f32 = 480.0;
const DEFAULT_PADDING: f32 = 24.0;
const VIEWPORT_MARGIN: f32 = 16.0;
const CONTENT_GAP: f32 = 20.0;
const HEADER_GAP: f32 = 4.0;
const CORNER_RADIUS: f32 = 12.0;
const BACKDROP_OPACITY: f32 = 0.64;

/// A controlled modal dialog.
///
/// The trigger is a regular button in the application view. Set [`Self::open`]
/// from application state, and use [`Self::on_close`] for the close button,
/// Escape key, and dismissible backdrop.
#[must_use]
pub struct Dialog<'a, Message> {
    base: Element<'a, Message>,
    body: Element<'a, Message>,
    open: bool,
    title: Option<Cow<'a, str>>,
    description: Option<Cow<'a, str>>,
    footer: Option<Element<'a, Message>>,
    on_close: Option<Message>,
    width: Length,
    padding: Padding,
    dismiss_on_backdrop: bool,
    dismiss_on_escape: bool,
    show_close_button: bool,
    close_label: Cow<'a, str>,
}

impl<Message> fmt::Debug for Dialog<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Dialog")
            .field("open", &self.open)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("has_footer", &self.footer.is_some())
            .field("has_close_message", &self.on_close.is_some())
            .field("width", &self.width)
            .field("padding", &self.padding)
            .field("dismiss_on_backdrop", &self.dismiss_on_backdrop)
            .field("dismiss_on_escape", &self.dismiss_on_escape)
            .field("show_close_button", &self.show_close_button)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Dialog<'a, Message> {
    /// Creates a closed dialog over `base`, containing `body`.
    pub fn new(
        base: impl Into<Element<'a, Message>>,
        body: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            base: base.into(),
            body: body.into(),
            open: false,
            title: None,
            description: None,
            footer: None,
            on_close: None,
            width: Length::Fixed(DEFAULT_WIDTH),
            padding: Padding::new(DEFAULT_PADDING),
            dismiss_on_backdrop: true,
            dismiss_on_escape: true,
            show_close_button: true,
            close_label: Cow::Borrowed(tr(Text::Close)),
        }
    }

    /// Controls whether the modal surface is visible.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the dialog heading.
    pub fn title(mut self, title: impl Into<Cow<'a, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the supporting text displayed below the heading.
    pub fn description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the action area displayed below the body.
    ///
    /// The caller controls the footer's responsive layout and button order.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Sets the preferred dialog width.
    ///
    /// The modal layout still keeps a margin around the surface on narrow
    /// windows.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the inner padding of the modal surface.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets whether pressing the backdrop publishes the close message.
    ///
    /// The backdrop always blocks input to the underlying content.
    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    /// Sets whether Escape publishes the close message.
    ///
    /// Escape is still captured while a non-dismissible dialog is open.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    /// Sets whether the standard close icon is shown.
    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    /// Sets the tooltip label of the standard close icon.
    pub fn close_label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.close_label = label.into();
        self
    }

    /// Sets the message used by every enabled dismissal path.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Sets the close message conditionally.
    pub fn on_close_maybe(mut self, message: Option<Message>) -> Self {
        self.on_close = message;
        self
    }
}

impl<'a, Message: Clone + 'a> Dialog<'a, Message> {
    /// Builds the modal widget.
    pub fn build(self) -> Element<'a, Message> {
        let has_header = self.title.is_some()
            || self.description.is_some()
            || (self.show_close_button && self.on_close.is_some());
        let mut content = Column::new().spacing(CONTENT_GAP).width(Length::Fill);

        if has_header {
            let mut heading = Column::new().spacing(HEADER_GAP).width(Length::Fill);

            if let Some(title) = self.title {
                heading = heading.push(text(title).size(20).font(typography::SANS_SEMIBOLD));
            }

            if let Some(description) = self.description {
                heading =
                    heading.push(container(text(description).size(14)).style(description_style));
            }

            let mut header = Row::new()
                .push(heading)
                .spacing(12)
                .align_y(Vertical::Top)
                .width(Length::Fill);

            if self.show_close_button
                && let Some(on_close) = self.on_close.clone()
            {
                header = header.push(
                    icon_button(lucide::x().size(16), self.close_label)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSmall)
                        .on_press(on_close),
                );
            }

            content = content.push(header);
        }

        content = content.push(self.body);

        if let Some(footer) = self.footer {
            content = content.push(
                container(footer)
                    .width(Length::Fill)
                    .align_x(Horizontal::Right),
            );
        }

        let surface = container(content)
            .width(self.width)
            .padding(self.padding)
            .style(surface_style);

        Element::new(Modal {
            base: self.base,
            surface: surface.into(),
            open: self.open,
            on_close: self.on_close,
            dismiss_on_backdrop: self.dismiss_on_backdrop,
            dismiss_on_escape: self.dismiss_on_escape,
        })
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        dialog.build()
    }
}

/// Creates a controlled modal dialog over `base`.
pub fn dialog<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
) -> Dialog<'a, Message> {
    Dialog::new(base, body)
}

struct Modal<'a, Message> {
    base: Element<'a, Message>,
    surface: Element<'a, Message>,
    open: bool,
    on_close: Option<Message>,
    dismiss_on_backdrop: bool,
    dismiss_on_escape: bool,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer> for Modal<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.base), Tree::new(&self.surface)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.base, &self.surface]);
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let base = self
            .base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = base.size();
        let available = Size::new(
            (size.width - VIEWPORT_MARGIN * 2.0).max(0.0),
            (size.height - VIEWPORT_MARGIN * 2.0).max(0.0),
        );
        let modal_limits = layout::Limits::new(Size::ZERO, available).loose();
        let surface =
            self.surface
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &modal_limits);
        let surface = surface.align(Alignment::Center, Alignment::Center, size);

        layout::Node::with_children(size, vec![base, surface])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut layouts = layout.children();
        let base_layout = layouts.next().expect("dialog base layout");
        let surface_layout = layouts.next().expect("dialog surface layout");

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            if self.open {
                self.surface.as_widget_mut().operate(
                    &mut tree.children[1],
                    surface_layout,
                    renderer,
                    operation,
                );
            } else {
                self.base.as_widget_mut().operate(
                    &mut tree.children[0],
                    base_layout,
                    renderer,
                    operation,
                );
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut layouts = layout.children();
        let base_layout = layouts.next().expect("dialog base layout");
        let surface_layout = layouts.next().expect("dialog surface layout");
        if !self.open {
            self.base.as_widget_mut().update(
                &mut tree.children[0],
                event,
                base_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            return;
        }

        if is_escape_press(event) {
            if self.dismiss_on_escape
                && let Some(on_close) = &self.on_close
            {
                shell.publish(on_close.clone());
            }

            shell.capture_event();
            return;
        }

        self.surface.as_widget_mut().update(
            &mut tree.children[1],
            event,
            surface_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        if is_backdrop_press(event, cursor, surface_layout.bounds()) {
            if self.dismiss_on_backdrop
                && let Some(on_close) = &self.on_close
            {
                shell.publish(on_close.clone());
            }

            shell.capture_event();
            return;
        }

        if is_user_input(event) {
            shell.capture_event();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut layouts = layout.children();
        let base_layout = layouts.next().expect("dialog base layout");
        let surface_layout = layouts.next().expect("dialog surface layout");
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            base_layout,
            if self.open {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
            viewport,
        );

        if !self.open {
            return;
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            Background::Color(backdrop_color()),
        );

        renderer.with_layer(*viewport, |renderer| {
            self.surface.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                surface_layout,
                cursor,
                viewport,
            );
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut layouts = layout.children();
        let base_layout = layouts.next().expect("dialog base layout");
        let surface_layout = layouts.next().expect("dialog surface layout");
        if self.open {
            self.surface.as_widget().mouse_interaction(
                &tree.children[1],
                surface_layout,
                cursor,
                viewport,
                renderer,
            )
        } else {
            self.base.as_widget().mouse_interaction(
                &tree.children[0],
                base_layout,
                cursor,
                viewport,
                renderer,
            )
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let mut layouts = layout.children();
        let base_layout = layouts.next().expect("dialog base layout");
        let surface_layout = layouts.next().expect("dialog surface layout");
        if self.open {
            self.surface.as_widget_mut().overlay(
                &mut tree.children[1],
                surface_layout,
                renderer,
                viewport,
                translation,
            )
        } else {
            self.base.as_widget_mut().overlay(
                &mut tree.children[0],
                base_layout,
                renderer,
                viewport,
                translation,
            )
        }
    }
}

fn is_escape_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        })
    )
}

fn is_backdrop_press(event: &Event, cursor: mouse::Cursor, surface: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
            .position()
            .is_some_and(|position| !surface.contains(position)),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !surface.contains(*position),
        _ => false,
    }
}

fn is_user_input(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(_) | Event::Mouse(_) | Event::Touch(_) | Event::InputMethod(_)
    )
}

const fn backdrop_color() -> Color {
    Color {
        a: BACKDROP_OPACITY,
        ..Color::BLACK
    }
}

fn surface_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        // The backdrop communicates modality while the lighter layer keeps
        // the borderless surface distinct from the page.
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        border: Border {
            radius: CORNER_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn description_style(theme: &Theme) -> container::Style {
    container::Style::default().color(with_alpha(
        theme.extended_palette().background.weak.text,
        0.72,
    ))
}

const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_controlled_and_dismissible() {
        let dialog = Dialog::<()>::new(text("base"), text("body"));

        assert!(!dialog.open);
        assert_eq!(dialog.width, Length::Fixed(DEFAULT_WIDTH));
        assert_eq!(dialog.padding, Padding::new(DEFAULT_PADDING));
        assert!(dialog.dismiss_on_backdrop);
        assert!(dialog.dismiss_on_escape);
        assert!(dialog.show_close_button);
        assert!(dialog.on_close.is_none());
    }

    #[test]
    fn surface_uses_a_borderless_flat_neutral_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let style = surface_style(&theme);

        assert_eq!(style.background, Some(palette.background.weak.color.into()));
        assert_eq!(style.text_color, Some(palette.background.weak.text));
        assert_eq!(style.border.radius, CORNER_RADIUS.into());
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn backdrop_is_a_visible_translucent_scrim() {
        let color = backdrop_color();

        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, BACKDROP_OPACITY);
        assert!(color.a > 0.6);
        assert!(color.a < 1.0);
    }

    #[test]
    fn touch_inside_surface_is_not_a_backdrop_press() {
        let surface = Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(100.0, 80.0));
        let event = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(1),
            position: iced::Point::new(30.0, 30.0),
        });

        assert!(!is_backdrop_press(
            &event,
            mouse::Cursor::Unavailable,
            surface,
        ));
    }

    #[test]
    fn touch_outside_surface_is_a_backdrop_press() {
        let surface = Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(100.0, 80.0));
        let event = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(1),
            position: iced::Point::new(10.0, 10.0),
        });

        assert!(is_backdrop_press(
            &event,
            mouse::Cursor::Unavailable,
            surface,
        ));
    }

    #[test]
    fn unavailable_mouse_does_not_dismiss_a_nested_overlay() {
        let surface = Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(100.0, 80.0));
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));

        assert!(!is_backdrop_press(
            &event,
            mouse::Cursor::Unavailable,
            surface,
        ));
    }
}
