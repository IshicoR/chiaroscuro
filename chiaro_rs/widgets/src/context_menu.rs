//! A controlled context menu anchored to a right-click position.
//!
//! [`ContextMenu`] keeps the application responsible for its open state. A
//! right-click over the base content publishes an open message and remembers
//! the exact cursor position. While open, the menu is rendered in an Iced
//! overlay, receives input before the base content, and stays inside the
//! viewport.

use std::fmt;

use iced::{
    Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch, window,
};

const DEFAULT_VIEWPORT_MARGIN: f32 = 8.0;

/// A controlled context menu.
///
/// The position passed to [`Self::on_open`] is relative to the base content.
/// Set [`Self::open`] from application state after handling the open message.
/// Menu item messages are not interpreted or closed automatically, so the
/// caller can decide whether an action keeps the menu open.
#[must_use]
pub struct ContextMenu<'a, Message> {
    base: Element<'a, Message>,
    menu: Element<'a, Message>,
    open: bool,
    on_open: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_close: Option<Message>,
    viewport_margin: f32,
}

impl<Message> fmt::Debug for ContextMenu<'_, Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContextMenu")
            .field("open", &self.open)
            .field("has_open_handler", &self.on_open.is_some())
            .field("has_close_message", &self.on_close.is_some())
            .field("viewport_margin", &self.viewport_margin)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> ContextMenu<'a, Message> {
    /// Creates a closed context menu over `base`.
    pub fn new(
        base: impl Into<Element<'a, Message>>,
        menu: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            base: base.into(),
            menu: menu.into(),
            open: false,
            on_open: None,
            on_close: None,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
        }
    }

    /// Controls whether the menu overlay is visible.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the message created by a right-click over the base content.
    ///
    /// The callback receives the click position relative to the base bounds.
    pub fn on_open(mut self, on_open: impl Fn(Point) -> Message + 'a) -> Self {
        self.on_open = Some(Box::new(on_open));
        self
    }

    /// Sets the message published by Escape or a press outside the menu.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Sets the minimum distance between the menu and the viewport edges.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = margin.max(0.0);
        self
    }
}

impl<'a, Message: Clone + 'a> ContextMenu<'a, Message> {
    /// Builds the context menu widget.
    pub fn build(self) -> Element<'a, Message> {
        Element::new(ControlledMenu {
            base: self.base,
            menu: self.menu,
            open: self.open,
            on_open: self.on_open,
            on_close: self.on_close,
            viewport_margin: self.viewport_margin,
        })
    }
}

impl<'a, Message: Clone + 'a> From<ContextMenu<'a, Message>> for Element<'a, Message> {
    fn from(context_menu: ContextMenu<'a, Message>) -> Self {
        context_menu.build()
    }
}

/// Creates a controlled context menu over `base`.
pub fn context_menu<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
) -> ContextMenu<'a, Message> {
    ContextMenu::new(base, menu)
}

struct ControlledMenu<'a, Message> {
    base: Element<'a, Message>,
    menu: Element<'a, Message>,
    open: bool,
    on_open: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_close: Option<Message>,
    viewport_margin: f32,
}

#[derive(Debug, Default)]
struct State {
    anchor: Point,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer>
    for ControlledMenu<'a, Message>
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.base), Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.base, &self.menu]);
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
        self.base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());

        if !self.open {
            operation.traverse(&mut |operation| {
                self.base.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    operation,
                );
            });
        }
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
        if shell.is_event_captured() {
            return;
        }

        if !self.open
            && let Some(position) = opening_position(event, cursor, layout.bounds())
            && let Some(on_open) = &self.on_open
        {
            tree.state.downcast_mut::<State>().anchor = position;
            shell.publish(on_open(position));
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if self.open && is_user_input(event) {
            shell.capture_event();
            return;
        }

        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            if self.open {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if self.open {
            mouse::Interaction::None
        } else {
            self.base.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
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
        if !self.open {
            return self.base.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            );
        }

        let anchor = tree.state.downcast_ref::<State>().anchor;
        let position = layout.position() + translation + Vector::new(anchor.x, anchor.y);

        Some(overlay::Element::new(Box::new(MenuOverlay {
            position,
            menu: &mut self.menu,
            tree: &mut tree.children[1],
            on_close: self.on_close.as_ref(),
            viewport_margin: self.viewport_margin,
        })))
    }
}

struct MenuOverlay<'a, 'b, Message> {
    position: Point,
    menu: &'a mut Element<'b, Message>,
    tree: &'a mut Tree,
    on_close: Option<&'a Message>,
    viewport_margin: f32,
}

impl<Message: Clone> overlay::Overlay<Message, Theme, iced::Renderer>
    for MenuOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let margin = self
            .viewport_margin
            .min(bounds.width / 2.0)
            .min(bounds.height / 2.0);
        let available = Size::new(
            (bounds.width - margin * 2.0).max(0.0),
            (bounds.height - margin * 2.0).max(0.0),
        );
        let menu = self.menu.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, available),
        );
        let position = clamped_menu_position(self.position, menu.size(), bounds, margin);

        layout::Node::with_children(bounds, vec![menu.move_to(position)])
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let menu_layout = layout.children().next().expect("context menu layout");

        if is_escape_press(event)
            || is_outside_press(event, cursor, menu_layout.bounds())
            || is_window_dismissal(event)
        {
            if let Some(on_close) = self.on_close {
                shell.publish(on_close.clone());
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        self.menu.as_widget_mut().update(
            self.tree,
            event,
            menu_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );

        if !shell.is_event_captured() && is_user_input(event) {
            shell.capture_event();
        }
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let menu_layout = layout.children().next().expect("context menu layout");
        self.menu.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            menu_layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let menu_layout = layout.children().next().expect("context menu layout");
        self.menu
            .as_widget_mut()
            .operate(self.tree, menu_layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let menu_layout = layout.children().next().expect("context menu layout");
        self.menu.as_widget().mouse_interaction(
            self.tree,
            menu_layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let menu_layout = layout.children().next().expect("context menu layout");
        self.menu.as_widget_mut().overlay(
            self.tree,
            menu_layout,
            renderer,
            &layout.bounds(),
            Vector::ZERO,
        )
    }
}

fn opening_position(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> Option<Point> {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
    )
    .then(|| cursor.position_in(bounds))
    .flatten()
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

fn is_outside_press(event: &Event, cursor: mouse::Cursor, menu: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left | mouse::Button::Right)) => {
            cursor
                .position()
                .is_some_and(|position| !menu.contains(position))
        },
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !menu.contains(*position),
        _ => false,
    }
}

fn is_window_dismissal(event: &Event) -> bool {
    matches!(
        event,
        Event::Window(window::Event::Resized { .. } | window::Event::Unfocused)
    )
}

fn is_user_input(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(_) | Event::Mouse(_) | Event::Touch(_) | Event::InputMethod(_)
    )
}

fn clamped_menu_position(anchor: Point, menu: Size, viewport: Size, margin: f32) -> Point {
    let margin = margin.max(0.0);
    let max_x = (viewport.width - margin - menu.width).max(margin);
    let max_y = (viewport.height - margin - menu.height).max(margin);
    let preferred_x = if anchor.x + menu.width > viewport.width - margin {
        anchor.x - menu.width
    } else {
        anchor.x
    };
    let preferred_y = if anchor.y + menu.height > viewport.height - margin {
        anchor.y - menu.height
    } else {
        anchor.y
    };

    Point::new(
        preferred_x.clamp(margin, max_x),
        preferred_y.clamp(margin, max_y),
    )
}

#[cfg(test)]
mod tests {
    use iced::{Event, Point, Rectangle, Size, keyboard, mouse, touch};

    use super::{clamped_menu_position, is_escape_press, is_outside_press, opening_position};

    #[test]
    fn right_click_reports_the_exact_local_position() {
        let bounds = Rectangle::new(Point::new(100.0, 40.0), Size::new(300.0, 180.0));
        let cursor = mouse::Cursor::Available(Point::new(146.0, 91.0));
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));

        assert_eq!(
            opening_position(&event, cursor, bounds),
            Some(Point::new(46.0, 51.0))
        );
    }

    #[test]
    fn clicks_outside_the_base_do_not_open_the_menu() {
        let bounds = Rectangle::new(Point::new(100.0, 40.0), Size::new(300.0, 180.0));
        let cursor = mouse::Cursor::Available(Point::new(99.0, 91.0));
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));

        assert_eq!(opening_position(&event, cursor, bounds), None);
    }

    #[test]
    fn menu_stays_at_the_anchor_when_there_is_room() {
        assert_eq!(
            clamped_menu_position(
                Point::new(100.0, 80.0),
                Size::new(120.0, 90.0),
                Size::new(500.0, 400.0),
                8.0,
            ),
            Point::new(100.0, 80.0)
        );
    }

    #[test]
    fn menu_flips_inside_the_bottom_right_viewport_edges() {
        assert_eq!(
            clamped_menu_position(
                Point::new(490.0, 390.0),
                Size::new(120.0, 90.0),
                Size::new(500.0, 400.0),
                8.0,
            ),
            Point::new(370.0, 300.0)
        );
    }

    #[test]
    fn oversized_menu_is_clamped_to_the_viewport_margin() {
        assert_eq!(
            clamped_menu_position(
                Point::new(20.0, 20.0),
                Size::new(500.0, 400.0),
                Size::new(300.0, 200.0),
                8.0,
            ),
            Point::new(8.0, 8.0)
        );
    }

    #[test]
    fn outside_mouse_and_touch_presses_are_dismissals() {
        let menu = Rectangle::new(Point::new(40.0, 40.0), Size::new(100.0, 80.0));
        let outside = mouse::Cursor::Available(Point::new(20.0, 20.0));
        let left = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let right = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        let touch = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(1),
            position: Point::new(20.0, 20.0),
        });

        assert!(is_outside_press(&left, outside, menu));
        assert!(is_outside_press(&right, outside, menu));
        assert!(is_outside_press(&touch, mouse::Cursor::Unavailable, menu));
    }

    #[test]
    fn unavailable_cursor_does_not_dismiss_a_nested_overlay() {
        let menu = Rectangle::new(Point::new(40.0, 40.0), Size::new(100.0, 80.0));
        let left = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));

        assert!(!is_outside_press(&left, mouse::Cursor::Unavailable, menu));
    }

    #[test]
    fn escape_is_recognized_as_a_dismissal() {
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });

        assert!(is_escape_press(&event));
    }
}
