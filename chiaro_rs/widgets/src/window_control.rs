//! Window chrome control buttons.
//!
//! These controls own their interaction animation, but only publish a caller
//! supplied message. Window lifecycle operations remain the responsibility of
//! the window crate.

use std::{
    fmt::{self, Debug},
    time::Duration,
};

use chiaro_i18n::{Text, tr};
use iced::{
    Animation, Background, Border, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{container, text, tooltip},
    window,
};
use iced_fonts::lucide;

const BUTTON_SIZE: f32 = 28.0;
const ICON_SIZE: u32 = 14;
const ICON_TOP_PADDING: f32 = 2.0;
const CORNER_RADIUS: f32 = 24.0;
const TOOLTIP_RADIUS: f32 = 6.0;
const FOCUS_BORDER_WIDTH: f32 = 2.0;
const TRANSITION_DURATION: Duration = Duration::from_millis(140);
const TOOLTIP_DELAY: Duration = Duration::from_secs(1);

/// The visual and accessibility role of a window control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Minimize,
    Maximize,
    Close,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Minimize => tr(Text::Minimize),
            Self::Maximize => tr(Text::Maximize),
            Self::Close => tr(Text::Close),
        }
    }

    const fn is_destructive(self) -> bool {
        matches!(self, Self::Close)
    }
}

/// A Chiaro window control button builder.
#[must_use]
pub struct WindowControlButton<Message> {
    kind: Kind,
    on_press: Message,
}

impl<Message> Debug for WindowControlButton<Message> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowControlButton")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl<Message> WindowControlButton<Message> {
    /// Creates a window control that publishes `on_press` when activated.
    pub fn new(kind: Kind, on_press: Message) -> Self {
        Self { kind, on_press }
    }
}

impl<'a, Message: Clone + 'a> WindowControlButton<Message> {
    /// Builds the control and its tooltip.
    pub fn build(self) -> Element<'a, Message> {
        let icon = match self.kind {
            Kind::Minimize => lucide::minus(),
            Kind::Maximize => lucide::square(),
            Kind::Close => lucide::x(),
        }
        .size(ICON_SIZE);
        let content = container(icon)
            .padding(Padding::ZERO.top(ICON_TOP_PADDING))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);
        let control = Element::new(Control {
            content: content.into(),
            kind: self.kind,
            on_press: self.on_press,
        });

        tooltip(
            control,
            container(text(self.kind.label()).size(12)).padding([4, 8]),
            tooltip::Position::Bottom,
        )
        .gap(4)
        .delay(TOOLTIP_DELAY)
        .style(tooltip_style)
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<WindowControlButton<Message>> for Element<'a, Message> {
    fn from(control: WindowControlButton<Message>) -> Self {
        control.build()
    }
}

/// Creates a window chrome control button.
pub fn window_control<Message>(kind: Kind, on_press: Message) -> WindowControlButton<Message> {
    WindowControlButton::new(kind, on_press)
}

struct Control<'a, Message> {
    content: Element<'a, Message>,
    kind: Kind,
    on_press: Message,
}

struct State {
    is_focused: bool,
    pressed: Option<Press>,
    hover: Animation<bool>,
    now: iced::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Mouse,
    Touch { id: touch::Finger, is_inside: bool },
    Keyboard(ActivationKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivationKey {
    Enter,
    Space,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Interaction {
    captured: bool,
    redraw: bool,
    activate: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_focused: false,
            pressed: None,
            hover: Animation::new(false).duration(TRANSITION_DURATION),
            now: iced::time::Instant::now(),
        }
    }
}

impl State {
    fn handle_event(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        mouse_is_inside: bool,
    ) -> Interaction {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if mouse_is_inside && self.pressed.is_none() {
                    self.is_focused = true;
                    self.pressed = Some(Press::Mouse);

                    Interaction::captured(true)
                } else if !mouse_is_inside {
                    Interaction::redraw_if(self.blur())
                } else {
                    Interaction::default()
                }
            },
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.pressed == Some(Press::Mouse) {
                    self.pressed = None;

                    Interaction {
                        captured: true,
                        redraw: true,
                        activate: mouse_is_inside,
                    }
                } else {
                    Interaction::default()
                }
            },
            Event::Mouse(mouse::Event::CursorLeft) => {
                Interaction::redraw_if(self.cancel_mouse_press())
            },
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                if matches!(self.pressed, Some(Press::Touch { .. })) {
                    return Interaction::default();
                }

                if bounds.contains(*position) && self.pressed.is_none() {
                    self.is_focused = true;
                    self.pressed = Some(Press::Touch {
                        id: *id,
                        is_inside: true,
                    });

                    Interaction::captured(true)
                } else if !bounds.contains(*position) {
                    Interaction::redraw_if(self.blur())
                } else {
                    Interaction::default()
                }
            },
            Event::Touch(touch::Event::FingerMoved { id, position }) => {
                let Some(Press::Touch {
                    id: active_id,
                    is_inside,
                }) = self.pressed.as_mut()
                else {
                    return Interaction::default();
                };

                if active_id != id {
                    return Interaction::default();
                }

                let next_is_inside = bounds.contains(*position);
                let changed = *is_inside != next_is_inside;
                *is_inside = next_is_inside;

                Interaction {
                    captured: true,
                    redraw: changed,
                    activate: false,
                }
            },
            Event::Touch(touch::Event::FingerLifted { id, position }) => {
                let Some(Press::Touch { id: active_id, .. }) = self.pressed else {
                    return Interaction::default();
                };

                if active_id != *id {
                    return Interaction::default();
                }

                self.pressed = None;

                Interaction {
                    captured: true,
                    redraw: true,
                    activate: bounds.contains(*position),
                }
            },
            Event::Touch(touch::Event::FingerLost { id, .. }) => {
                let Some(Press::Touch { id: active_id, .. }) = self.pressed else {
                    return Interaction::default();
                };

                if active_id != *id {
                    return Interaction::default();
                }

                self.pressed = None;

                Interaction::captured(true)
            },
            Event::Keyboard(keyboard::Event::KeyPressed { key, repeat, .. }) if self.is_focused => {
                if matches!(key.as_ref(), keyboard::Key::Named(key::Named::Escape)) {
                    let changed = self.pressed.take().is_some();

                    return Interaction {
                        captured: changed,
                        redraw: changed,
                        activate: false,
                    };
                }

                let Some(key) = activation_key(key) else {
                    return Interaction::default();
                };

                if self.pressed == Some(Press::Keyboard(key)) {
                    return Interaction::captured(false);
                }

                if *repeat || self.pressed.is_some() {
                    return Interaction::default();
                }

                self.pressed = Some(Press::Keyboard(key));

                Interaction::captured(true)
            },
            Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) if self.is_focused => {
                let Some(key) = activation_key(key) else {
                    return Interaction::default();
                };

                if self.pressed != Some(Press::Keyboard(key)) {
                    return Interaction::default();
                }

                self.pressed = None;

                Interaction {
                    captured: true,
                    redraw: true,
                    activate: true,
                }
            },
            Event::Window(window::Event::Unfocused) => Interaction::redraw_if(self.blur()),
            _ => Interaction::default(),
        }
    }

    fn handle_captured_event(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        mouse_is_inside: bool,
    ) -> bool {
        match event {
            Event::Window(window::Event::Unfocused) => self.blur(),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if !mouse_is_inside => {
                self.blur()
            },
            Event::Mouse(
                mouse::Event::ButtonReleased(mouse::Button::Left) | mouse::Event::CursorLeft,
            ) => self.cancel_mouse_press(),
            Event::Touch(touch::Event::FingerPressed { position, .. })
                if !bounds.contains(*position)
                    && !matches!(self.pressed, Some(Press::Touch { .. })) =>
            {
                self.blur()
            },
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) => self.cancel_touch_press(*id),
            Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                let Some(key) = activation_key(key) else {
                    return false;
                };

                if self.pressed == Some(Press::Keyboard(key)) {
                    self.pressed = None;
                    true
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    fn blur(&mut self) -> bool {
        let changed = self.is_focused || self.pressed.is_some();
        self.is_focused = false;
        self.pressed = None;
        changed
    }

    fn cancel_mouse_press(&mut self) -> bool {
        if self.pressed == Some(Press::Mouse) {
            self.pressed = None;
            true
        } else {
            false
        }
    }

    fn cancel_touch_press(&mut self, id: touch::Finger) -> bool {
        if matches!(self.pressed, Some(Press::Touch { id: active_id, .. }) if active_id == id) {
            self.pressed = None;
            true
        } else {
            false
        }
    }

    fn is_visually_pressed(&self, mouse_is_inside: bool) -> bool {
        match self.pressed {
            Some(Press::Mouse) => mouse_is_inside,
            Some(Press::Touch { is_inside, .. }) => is_inside,
            Some(Press::Keyboard(_)) => self.is_focused,
            None => false,
        }
    }
}

impl Interaction {
    const fn captured(redraw: bool) -> Self {
        Self {
            captured: true,
            redraw,
            activate: false,
        }
    }

    const fn redraw_if(redraw: bool) -> Self {
        Self {
            captured: false,
            redraw,
            activate: false,
        }
    }
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.blur();
    }
}

fn activation_key(key: &keyboard::Key) -> Option<ActivationKey> {
    match key.as_ref() {
        keyboard::Key::Named(key::Named::Enter) => Some(ActivationKey::Enter),
        keyboard::Key::Named(key::Named::Space) => Some(ActivationKey::Space),
        _ => None,
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer> for Control<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(BUTTON_SIZE), Length::Fixed(BUTTON_SIZE))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(
            limits,
            Length::Fixed(BUTTON_SIZE),
            Length::Fixed(BUTTON_SIZE),
            Padding::ZERO,
            |limits| {
                self.content
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits)
            },
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(None, layout.bounds(), state);
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("window control content"),
                renderer,
                operation,
            );
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().expect("window control content"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let bounds = layout.bounds();
        let hovered = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();

        if shell.is_event_captured() {
            if state.handle_captured_event(event, bounds, hovered) {
                shell.request_redraw();
            }

            return;
        }

        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) => *now,
            _ => iced::time::Instant::now(),
        };

        if state.hover.value() != hovered {
            state.hover.go_mut(hovered, now);
            state.now = now;
            shell.request_redraw();
        }

        let interaction = state.handle_event(event, bounds, hovered);

        if interaction.activate {
            shell.publish(self.on_press.clone());
        }

        if interaction.captured {
            shell.capture_event();
        }

        if interaction.redraw {
            shell.request_redraw();
        }

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = *now;

            if state.hover.is_animating(*now) {
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let pressed = state.is_visually_pressed(cursor.is_over(bounds));
        let progress = if pressed {
            1.0
        } else {
            state.hover.interpolate(0.0, 1.0, state.now)
        };
        let appearance = appearance(theme, self.kind, progress, pressed);
        let focus_border = theme.extended_palette().primary.base.color;

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: CORNER_RADIUS.into(),
                    color: focus_border,
                    width: if state.is_focused {
                        FOCUS_BORDER_WIDTH
                    } else {
                        0.0
                    },
                },
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(appearance.background),
        );

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: appearance.text,
            },
            layout.children().next().expect("window control content"),
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("window control content"),
            renderer,
            viewport,
            translation,
        )
    }
}

struct Appearance {
    background: Color,
    text: Color,
}

fn appearance(theme: &Theme, kind: Kind, progress: f32, pressed: bool) -> Appearance {
    let palette = theme.extended_palette();
    let base = palette.background.base;
    let target = if kind.is_destructive() && pressed {
        palette.danger.base
    } else if kind.is_destructive() {
        palette.danger.weak
    } else if pressed {
        palette.background.strong
    } else {
        palette.background.weak
    };
    let progress = progress.clamp(0.0, 1.0);

    Appearance {
        background: target.color.scale_alpha(progress),
        text: mix_color(base.text, target.text, progress),
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            radius: TOOLTIP_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn mix_color(start: Color, end: Color, amount: f32) -> Color {
    Color {
        r: start.r + (end.r - start.r) * amount,
        g: start.g + (end.g - start.g) * amount,
        b: start.b + (end.b - start.b) * amount,
        a: start.a + (end.a - start.a) * amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rectangle = Rectangle {
        x: 10.0,
        y: 20.0,
        width: BUTTON_SIZE,
        height: BUTTON_SIZE,
    };

    #[test]
    fn close_is_the_only_destructive_control() {
        assert!(!Kind::Minimize.is_destructive());
        assert!(!Kind::Maximize.is_destructive());
        assert!(Kind::Close.is_destructive());
    }

    #[test]
    fn controls_have_accessible_labels() {
        assert_eq!(Kind::Minimize.label(), "Minimize");
        assert_eq!(Kind::Maximize.label(), "Maximize");
        assert_eq!(Kind::Close.label(), "Close");
    }

    #[test]
    fn tooltip_uses_the_shared_flat_surface() {
        let style = tooltip_style(&Theme::Dark);

        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius, TOOLTIP_RADIUS.into());
        assert_eq!(style.shadow, Shadow::default());
    }

    #[test]
    fn hover_progress_changes_the_background_opacity() {
        let idle = appearance(&Theme::Dark, Kind::Minimize, 0.0, false);
        let hovered = appearance(&Theme::Dark, Kind::Minimize, 1.0, false);

        assert_eq!(idle.background.a, 0.0);
        assert!(hovered.background.a > idle.background.a);
    }

    #[test]
    fn pressed_controls_use_a_stronger_interaction_layer() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let hovered = appearance(&theme, Kind::Minimize, 1.0, false);
        let pressed = appearance(&theme, Kind::Minimize, 1.0, true);

        assert_eq!(hovered.background, palette.background.weak.color);
        assert_eq!(pressed.background, palette.background.strong.color);
    }

    #[test]
    fn close_uses_the_danger_layer_only_while_interacting() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let hovered = appearance(&theme, Kind::Close, 1.0, false);
        let pressed = appearance(&theme, Kind::Close, 1.0, true);

        assert_eq!(hovered.background, palette.danger.weak.color);
        assert_eq!(pressed.background, palette.danger.base.color);
    }

    #[test]
    fn touch_uses_its_position_instead_of_the_mouse_cursor() {
        let mut state = State::default();
        let finger = touch::Finger(7);
        let press = Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: iced::Point::new(12.0, 22.0),
        });

        let interaction = state.handle_event(&press, BOUNDS, false);

        assert!(interaction.captured);
        assert!(interaction.redraw);
        assert!(state.is_focused);
        assert_eq!(
            state.pressed,
            Some(Press::Touch {
                id: finger,
                is_inside: true
            })
        );
    }

    #[test]
    fn touch_only_responds_to_the_finger_that_started_the_press() {
        let mut state = State::default();
        let active = touch::Finger(3);
        let other = touch::Finger(4);
        state.handle_event(
            &Event::Touch(touch::Event::FingerPressed {
                id: active,
                position: iced::Point::new(12.0, 22.0),
            }),
            BOUNDS,
            false,
        );

        let move_other = state.handle_event(
            &Event::Touch(touch::Event::FingerMoved {
                id: other,
                position: iced::Point::new(60.0, 60.0),
            }),
            BOUNDS,
            false,
        );
        let lift_other = state.handle_event(
            &Event::Touch(touch::Event::FingerLifted {
                id: other,
                position: iced::Point::new(12.0, 22.0),
            }),
            BOUNDS,
            false,
        );

        assert_eq!(move_other, Interaction::default());
        assert_eq!(lift_other, Interaction::default());
        assert_eq!(
            state.pressed,
            Some(Press::Touch {
                id: active,
                is_inside: true
            })
        );
    }

    #[test]
    fn touch_release_outside_cancels_activation() {
        let mut state = State::default();
        let finger = touch::Finger(5);
        state.handle_event(
            &Event::Touch(touch::Event::FingerPressed {
                id: finger,
                position: iced::Point::new(12.0, 22.0),
            }),
            BOUNDS,
            false,
        );
        let moved = state.handle_event(
            &Event::Touch(touch::Event::FingerMoved {
                id: finger,
                position: iced::Point::new(60.0, 60.0),
            }),
            BOUNDS,
            false,
        );
        let released = state.handle_event(
            &Event::Touch(touch::Event::FingerLifted {
                id: finger,
                position: iced::Point::new(60.0, 60.0),
            }),
            BOUNDS,
            false,
        );

        assert!(moved.captured);
        assert!(moved.redraw);
        assert!(released.captured);
        assert!(!released.activate);
        assert_eq!(state.pressed, None);
    }

    #[test]
    fn enter_and_space_activate_a_focused_control_on_release() {
        for named in [key::Named::Enter, key::Named::Space] {
            let mut state = State::default();
            operation::Focusable::focus(&mut state);

            let pressed = state.handle_event(&key_pressed(named), BOUNDS, false);
            let released = state.handle_event(&key_released(named), BOUNDS, false);

            assert!(pressed.captured);
            assert!(pressed.redraw);
            assert!(released.captured);
            assert!(released.activate);
            assert_eq!(state.pressed, None);
        }
    }

    #[test]
    fn losing_focus_cancels_an_active_press() {
        let mut state = State::default();
        operation::Focusable::focus(&mut state);
        state.handle_event(&key_pressed(key::Named::Space), BOUNDS, false);

        operation::Focusable::unfocus(&mut state);

        assert!(!state.is_focused);
        assert_eq!(state.pressed, None);
        assert_eq!(
            state.handle_event(&key_released(key::Named::Space), BOUNDS, false),
            Interaction::default()
        );
    }

    #[test]
    fn escape_cancels_keyboard_activation_without_blurring() {
        let mut state = State::default();
        operation::Focusable::focus(&mut state);
        state.handle_event(&key_pressed(key::Named::Space), BOUNDS, false);

        let cancelled = state.handle_event(&key_pressed(key::Named::Escape), BOUNDS, false);
        let released = state.handle_event(&key_released(key::Named::Space), BOUNDS, false);

        assert!(cancelled.captured);
        assert!(cancelled.redraw);
        assert!(state.is_focused);
        assert_eq!(state.pressed, None);
        assert_eq!(released, Interaction::default());
    }

    #[test]
    fn window_focus_loss_cancels_pointer_press() {
        let mut state = State::default();
        state.handle_event(
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            BOUNDS,
            true,
        );

        let interaction =
            state.handle_event(&Event::Window(window::Event::Unfocused), BOUNDS, true);

        assert!(interaction.redraw);
        assert!(!interaction.activate);
        assert!(!state.is_focused);
        assert_eq!(state.pressed, None);
    }

    fn key_pressed(named: key::Named) -> Event {
        let key = keyboard::Key::Named(named);

        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        })
    }

    fn key_released(named: key::Named) -> Event {
        let key = keyboard::Key::Named(named);

        Event::Keyboard(keyboard::Event::KeyReleased {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
        })
    }
}
