use std::time::{Duration, Instant};

use iced::{
    Animation, Element,
    Length::Fill,
    Size, Subscription, Task,
    alignment::{Horizontal, Vertical},
    widget::{button, container, mouse_area, row, text, tooltip},
    window::{self as iced_window, Id},
};
use iced_fonts::lucide;

use crate::appearance;

const CONTROL_TRANSITION_DURATION: Duration = Duration::from_millis(140);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug, Clone)]
pub struct WindowState {
    minimize_hover: Animation<bool>,
    maximize_hover: Animation<bool>,
    close_hover: Animation<bool>,
    now: Instant,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            minimize_hover: hover_animation(),
            maximize_hover: hover_animation(),
            close_hover: hover_animation(),
            now: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowMessage {
    CloseRequested(Id),
    Drag,
    ToggleMaximize,
    Minimize,
    Close,
    Hover(WindowControl, bool),
    AnimationFrame(Instant),
}

pub fn subscription(state: &WindowState) -> Subscription<WindowMessage> {
    let animation = if state.is_animating() {
        iced::time::every(ANIMATION_FRAME_INTERVAL).map(WindowMessage::AnimationFrame)
    } else {
        Subscription::none()
    };

    Subscription::batch([
        iced_window::close_requests().map(WindowMessage::CloseRequested),
        animation,
    ])
}

pub fn update(state: &mut WindowState, message: WindowMessage) -> Task<WindowMessage> {
    match message {
        WindowMessage::CloseRequested(id) => iced_window::close(id),
        WindowMessage::Drag => with_latest(iced_window::drag),
        WindowMessage::ToggleMaximize => with_latest(iced_window::toggle_maximize),
        WindowMessage::Minimize => with_latest(|id| iced_window::minimize(id, true)),
        WindowMessage::Close => with_latest(iced_window::close),
        WindowMessage::Hover(control, hovered) => {
            let now = Instant::now();
            state.now = now;
            state.animation_mut(control).go_mut(hovered, now);
            Task::none()
        },
        WindowMessage::AnimationFrame(now) => {
            state.now = now;
            Task::none()
        },
    }
}

pub fn settings() -> iced_window::Settings {
    iced_window::Settings {
        size: Size::new(960.0, 640.0),
        min_size: Some(Size::new(720.0, 480.0)),
        decorations: false,
        exit_on_close_request: false,
        ..iced_window::Settings::default()
    }
}

pub fn title_bar<'a, AppMessage: Clone + 'a>(
    state: &'a WindowState,
    leading: Element<'a, AppMessage>,
    show_title: bool,
    map_message: fn(WindowMessage) -> AppMessage,
) -> Element<'a, AppMessage> {
    let title = if show_title { "CHIAROSCURO" } else { "" };
    let drag_region = mouse_area(
        container(text(title).size(13))
            .width(Fill)
            .height(appearance::TITLE_BAR_HEIGHT)
            .center_y(appearance::TITLE_BAR_HEIGHT)
            .padding([0, 16]),
    )
    .on_press(map_message(WindowMessage::Drag))
    .on_double_click(map_message(WindowMessage::ToggleMaximize));

    container(
        row![leading, drag_region, controls(state).map(map_message),].align_y(Vertical::Center),
    )
    .width(Fill)
    .height(appearance::TITLE_BAR_HEIGHT)
    .style(appearance::title_bar)
    .into()
}

fn controls(state: &WindowState) -> Element<'_, WindowMessage> {
    row![
        control(
            &state.minimize_hover,
            state.now,
            lucide::minus().size(16),
            "Minimize",
            WindowMessage::Minimize,
            WindowControl::Minimize,
            false,
        ),
        control(
            &state.maximize_hover,
            state.now,
            lucide::square().size(14),
            "Maximize",
            WindowMessage::ToggleMaximize,
            WindowControl::Maximize,
            false,
        ),
        control(
            &state.close_hover,
            state.now,
            lucide::x().size(16),
            "Close",
            WindowMessage::Close,
            WindowControl::Close,
            true,
        ),
    ]
    .align_y(Vertical::Center)
    .into()
}

fn control<'a>(
    hover: &'a Animation<bool>,
    now: Instant,
    icon: iced::widget::Text<'static>,
    label: &'static str,
    message: WindowMessage,
    control_kind: WindowControl,
    destructive: bool,
) -> Element<'a, WindowMessage> {
    let hover_progress = hover.interpolate(0.0, 1.0, now);
    let button = button(icon.align_x(Horizontal::Center).align_y(Vertical::Center))
        .width(appearance::WINDOW_CONTROL_SIZE)
        .height(appearance::WINDOW_CONTROL_SIZE)
        .padding(0)
        .on_press(message)
        .style(move |theme, status| {
            appearance::window_control(theme, status, destructive, hover_progress)
        });
    let control = mouse_area(button)
        .on_enter(WindowMessage::Hover(control_kind, true))
        .on_exit(WindowMessage::Hover(control_kind, false));

    tooltip(control, text(label).size(12), tooltip::Position::Bottom).into()
}

impl WindowState {
    fn animation_mut(&mut self, control: WindowControl) -> &mut Animation<bool> {
        match control {
            WindowControl::Minimize => &mut self.minimize_hover,
            WindowControl::Maximize => &mut self.maximize_hover,
            WindowControl::Close => &mut self.close_hover,
        }
    }

    fn is_animating(&self) -> bool {
        self.minimize_hover.is_animating(self.now)
            || self.maximize_hover.is_animating(self.now)
            || self.close_hover.is_animating(self.now)
    }
}

fn hover_animation() -> Animation<bool> {
    Animation::new(false).duration(CONTROL_TRANSITION_DURATION)
}

pub fn close() -> Task<WindowMessage> {
    with_latest(iced_window::close)
}

fn with_latest(
    operation: impl Fn(Id) -> Task<WindowMessage> + Send + 'static,
) -> Task<WindowMessage> {
    iced_window::latest().and_then(operation)
}

#[cfg(test)]
mod tests {
    use super::{WindowControl, WindowMessage, WindowState, update};

    #[test]
    fn hover_messages_change_the_matching_animation_target() {
        let mut state = WindowState::default();

        drop(update(
            &mut state,
            WindowMessage::Hover(WindowControl::Close, true),
        ));

        assert!(state.close_hover.value());
        assert!(!state.minimize_hover.value());
        assert!(!state.maximize_hover.value());
    }
}
