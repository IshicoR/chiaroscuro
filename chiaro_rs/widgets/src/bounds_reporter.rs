//! A transparent widget wrapper that reports its laid-out bounds.
//!
//! Iced reconciles widget trees by position. A reorderable card can therefore
//! inherit the wrapper state of a different card after the grid is reordered.
//! The reporter tracks both its identity key and its last bounds so reordered
//! cards always publish a fresh measurement.

use iced::{
    Element, Event, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};

/// Wraps `content` and publishes its logical layout bounds when they change.
///
/// `key` identifies the content independently of its position in a parent
/// collection. The callback is also invoked when only the key changes, which
/// keeps measurements correct when Iced reuses a widget tree after reordering.
/// The visible bounds are clipped to the viewport supplied by the parent.
pub fn bounds_reporter<'a, Key, Message>(
    key: Key,
    content: impl Into<Element<'a, Message>>,
    on_change: impl Fn(Key, Rectangle, Option<Rectangle>) -> Message + 'a,
) -> Element<'a, Message>
where
    Key: Clone + PartialEq + 'static,
    Message: 'a,
{
    Element::new(BoundsReporter {
        key,
        content: content.into(),
        on_change: Box::new(on_change),
    })
}

struct BoundsReporter<'a, Key, Message> {
    key: Key,
    content: Element<'a, Message>,
    on_change: Box<dyn Fn(Key, Rectangle, Option<Rectangle>) -> Message + 'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Measurement {
    bounds: Rectangle,
    visible_bounds: Option<Rectangle>,
}

#[derive(Debug)]
struct State<Key> {
    key: Key,
    measurement: Option<Measurement>,
}

impl<Key: Clone + PartialEq> State<Key> {
    fn new(key: Key) -> Self {
        Self {
            key,
            measurement: None,
        }
    }

    fn reset_for_key(&mut self, key: &Key) {
        if self.key != *key {
            self.key = key.clone();
            self.measurement = None;
        }
    }

    fn observe(&mut self, key: &Key, measurement: Measurement) -> bool {
        self.reset_for_key(key);

        if self.measurement == Some(measurement) {
            false
        } else {
            self.measurement = Some(measurement);
            true
        }
    }
}

impl<'a, Key, Message> Widget<Message, Theme, iced::Renderer> for BoundsReporter<'a, Key, Message>
where
    Key: Clone + PartialEq + 'static,
    Message: 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Key>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.key.clone()))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.state
            .downcast_mut::<State<Key>>()
            .reset_for_key(&self.key);
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
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
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let measurement = Measurement {
            bounds: layout.bounds(),
            visible_bounds: layout.bounds().intersection(viewport),
        };
        let state = tree.state.downcast_mut::<State<Key>>();

        if state.observe(&self.key, measurement) {
            shell.publish((self.on_change)(
                self.key.clone(),
                measurement.bounds,
                measurement.visible_bounds,
            ));
        }

        self.content.as_widget_mut().update(
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Measurement, State};
    use iced::Rectangle;

    const BOUNDS: Rectangle = Rectangle {
        x: 10.0,
        y: 20.0,
        width: 300.0,
        height: 180.0,
    };

    const MEASUREMENT: Measurement = Measurement {
        bounds: BOUNDS,
        visible_bounds: Some(BOUNDS),
    };

    #[test]
    fn reports_the_initial_measurement_once() {
        let mut state = State::new(1_u8);

        assert!(state.observe(&1, MEASUREMENT));
        assert!(!state.observe(&1, MEASUREMENT));
    }

    #[test]
    fn reports_when_bounds_change() {
        let mut state = State::new(1_u8);
        assert!(state.observe(&1, MEASUREMENT));

        let moved = Measurement {
            bounds: Rectangle {
                x: BOUNDS.x + 12.0,
                ..BOUNDS
            },
            ..MEASUREMENT
        };

        assert!(state.observe(&1, moved));
        assert!(!state.observe(&1, moved));
    }

    #[test]
    fn reports_again_when_the_identity_changes() {
        let mut state = State::new(1_u8);
        assert!(state.observe(&1, MEASUREMENT));

        state.reset_for_key(&2);

        assert!(state.observe(&2, MEASUREMENT));
        assert!(!state.observe(&2, MEASUREMENT));
    }

    #[test]
    fn reports_when_only_the_visible_bounds_change() {
        let mut state = State::new(1_u8);
        assert!(state.observe(&1, MEASUREMENT));

        let clipped = Measurement {
            visible_bounds: Some(Rectangle {
                height: 80.0,
                ..BOUNDS
            }),
            ..MEASUREMENT
        };

        assert!(state.observe(&1, clipped));
        assert!(!state.observe(&1, clipped));
    }
}
