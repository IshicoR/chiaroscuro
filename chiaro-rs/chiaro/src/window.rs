use std::time::Instant;

use iced::{
    Animation, Background, Border, Color, Element,
    Length::Fill,
    Size, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    time,
    widget::button,
    widget::{
        self, Text, button::Status as ButtonStatus, container::Style, mouse_area, row, text,
        tooltip,
    },
    window::{self, Id, Settings},
};
use iced_fonts::lucide;

use crate::appearance::{
    ANIMATION_FRAME_INTERVAL, APPLICATION_TITLE, BUTTON_CORNER_RADIUS, CONTROL_TRANSITION_DURATION,
    ICON_SIZE, TITLE_BAR_HEIGHT, TOOLTIP_DELAY, WINDOW_CONTROL_BUTTON_SIZE,
};

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

#[derive(Debug, Clone, Copy)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowMessage {
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
    Hover(WindowControl, bool),
    AnimationFrame(Instant),
}

pub fn settings() -> Settings {
    Settings {
        size: Size::new(960.0, 640.0),
        min_size: Some(Size::new(720.0, 480.0)),
        decorations: false,
        exit_on_close_request: false,
        ..window::Settings::default()
    }
}

pub fn subscription(state: &WindowState) -> Subscription<WindowMessage> {
    let animation = if state.is_animating() {
        time::every(ANIMATION_FRAME_INTERVAL).map(WindowMessage::AnimationFrame)
    } else {
        Subscription::none()
    };

    Subscription::batch([animation])
}

pub fn update(state: &mut WindowState, msg: WindowMessage) -> Task<WindowMessage> {
    match msg {
        WindowMessage::Drag => with_latest(window::drag),
        WindowMessage::Minimize => with_latest(|id| window::minimize(id, true)),
        WindowMessage::ToggleMaximize => with_latest(window::toggle_maximize),
        WindowMessage::Close => with_latest(window::close),
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

pub fn view<'a, AppMessage: Clone + 'a>(
    state: &'a WindowState,
    leading: Element<'a, AppMessage>,
    show_title: bool,
    map_message: fn(WindowMessage) -> AppMessage,
) -> Element<'a, AppMessage> {
    let title = if show_title { APPLICATION_TITLE } else { "" };
    let drag_region = mouse_area(
        widget::container(text(title).size(ICON_SIZE))
            .width(Fill)
            .height(TITLE_BAR_HEIGHT)
            .center_y(TITLE_BAR_HEIGHT)
            .padding([0, 16]),
    )
    .on_press(map_message(WindowMessage::Drag))
    .on_double_click(map_message(WindowMessage::ToggleMaximize));

    widget::container(
        row![leading, drag_region, controls(state).map(map_message),].align_y(Vertical::Center),
    )
    .width(Fill)
    .height(TITLE_BAR_HEIGHT)
    .padding([0, 6])
    .style(bar_style)
    .into()
}

pub fn close() -> Task<WindowMessage> {
    with_latest(window::close)
}

fn controls(state: &WindowState) -> Element<'_, WindowMessage> {
    row![
        control(
            &state.minimize_hover,
            state.now,
            lucide::minus().size(ICON_SIZE),
            "Minimize",
            WindowMessage::Minimize,
            WindowControl::Minimize,
            false,
        ),
        control(
            &state.maximize_hover,
            state.now,
            lucide::square().size(ICON_SIZE),
            "Maximize",
            WindowMessage::ToggleMaximize,
            WindowControl::Maximize,
            false,
        ),
        control(
            &state.close_hover,
            state.now,
            lucide::x().size(ICON_SIZE),
            "Close",
            WindowMessage::Close,
            WindowControl::Close,
            true,
        ),
    ]
    .align_y(Vertical::Center)
    .spacing(6)
    .into()
}

fn control<'a>(
    hover: &'a Animation<bool>,
    now: Instant,
    icon: Text<'static>,
    label: &'static str,
    msg: WindowMessage,
    control_kind: WindowControl,
    destructive: bool,
) -> Element<'a, WindowMessage> {
    let hover_progress = hover.interpolate(0.0, 1.0, now);
    let button = button(icon.align_x(Horizontal::Center).align_y(Vertical::Center))
        .width(WINDOW_CONTROL_BUTTON_SIZE)
        .height(WINDOW_CONTROL_BUTTON_SIZE)
        .padding(0)
        .on_press(msg)
        .style(move |theme, status| control_style(theme, status, destructive, hover_progress));
    let control = mouse_area(button)
        .on_enter(WindowMessage::Hover(control_kind, true))
        .on_exit(WindowMessage::Hover(control_kind, false));

    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .delay(TOOLTIP_DELAY)
        .into()
}

fn control_style(
    theme: &Theme,
    status: button::Status,
    destructive: bool,
    hover_progress: f32,
) -> button::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base;
    let target = if destructive {
        palette.danger.base
    } else {
        palette.background.strong
    };
    let progress = if status == ButtonStatus::Pressed {
        1.0
    } else {
        hover_progress.clamp(0.0, 1.0)
    };

    let mut style = button::Style {
        background: Some(Background::Color(target.color.scale_alpha(progress))),
        text_color: mix_color(base.text, target.text, progress),
        border: Border {
            radius: BUTTON_CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    };

    if status == ButtonStatus::Disabled {
        style.text_color = style.text_color.scale_alpha(0.45);
    }

    style
}

fn bar_style(theme: &Theme) -> Style {
    let colors = theme.extended_palette().background.weaker;

    Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
}

fn mix_color(start: Color, end: Color, amount: f32) -> Color {
    Color {
        r: start.r + (end.r - start.r) * amount,
        g: start.g + (end.g - start.g) * amount,
        b: start.b + (end.b - start.b) * amount,
        a: start.a + (end.a - start.a) * amount,
    }
}

fn hover_animation() -> Animation<bool> {
    Animation::new(false).duration(CONTROL_TRANSITION_DURATION)
}

fn with_latest(
    operation: impl Fn(Id) -> Task<WindowMessage> + Send + 'static,
) -> Task<WindowMessage> {
    window::latest().and_then(operation)
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
