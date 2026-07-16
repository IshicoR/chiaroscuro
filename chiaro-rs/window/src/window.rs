use std::time::{Duration, Instant};

use chiaro_theme::typography;
use iced::{
    Animation, Background, Border, Color, Element, Event,
    Length::{self, Fill, Fixed},
    Size, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    border::Radius,
    event,
    mouse::{self, Interaction},
    time,
    widget::{
        self, MouseArea, Space, Text, button, container::Style, mouse_area, row, text, tooltip,
    },
    window::{self, Direction, Id, Mode, Settings},
};
use iced_fonts::lucide;

const TITLE_BAR_HEIGHT: f32 = 34.0;
const WINDOW_CORNER_RADIUS: f32 = 10.0;
const WINDOW_CONTROL_BUTTON_SIZE: f32 = 24.0;
const RESIZE_HANDLE_SIZE: f32 = 6.0;
const ICON_SIZE: u32 = 12;
const BUTTON_CORNER_RADIUS: f32 = 24.0;
const CONTROL_TRANSITION_DURATION: Duration = Duration::from_millis(140);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);
const TOOLTIP_DELAY: Duration = Duration::from_secs(1);
const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo.png");
const WINDOW_ICON_SIZE: u32 = 256;

#[derive(Debug, Clone)]
pub struct WindowState {
    minimize_hover: Animation<bool>,
    maximize_hover: Animation<bool>,
    close_hover: Animation<bool>,
    now: Instant,
    backgrounded: bool,
    maximized: bool,
    focused: bool,
    active_border_color: Color,
    inactive_border_color: Color,
}

impl Default for WindowState {
    fn default() -> Self {
        let (active_border_color, inactive_border_color) = system_border_colors();

        Self {
            minimize_hover: hover_animation(),
            maximize_hover: hover_animation(),
            close_hover: hover_animation(),
            now: Instant::now(),
            backgrounded: false,
            maximized: false,
            focused: true,
            active_border_color,
            inactive_border_color,
        }
    }
}

impl WindowState {
    pub fn is_backgrounded(&self) -> bool {
        self.backgrounded
    }

    pub fn uses_rounded_corners(&self) -> bool {
        !self.maximized
    }

    fn border_color(&self) -> Color {
        if self.focused {
            self.active_border_color
        } else {
            self.inactive_border_color
        }
    }

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
    Resize(Direction),
    Minimize,
    ToggleMaximize,
    CloseRequested,
    Focused,
    Unfocused,
    Hover(WindowControl, bool),
    AnimationFrame(Instant),
    CheckMaximized,
    MaximizedChanged(bool),
}

pub fn settings() -> Settings {
    Settings {
        size: Size::new(960.0, 640.0),
        min_size: Some(Size::new(960.0, 640.0)),
        resizable: true,
        decorations: false,
        transparent: true,
        icon: window_icon(),
        exit_on_close_request: false,
        ..window::Settings::default()
    }
}

fn window_icon() -> Option<window::Icon> {
    let logo = image::load_from_memory(LOGO_BYTES).ok()?.resize_exact(
        WINDOW_ICON_SIZE,
        WINDOW_ICON_SIZE,
        image::imageops::FilterType::Lanczos3,
    );
    let logo = logo.into_rgba8();
    let (width, height) = logo.dimensions();

    window::icon::from_rgba(logo.into_raw(), width, height).ok()
}

pub fn subscription(state: &WindowState) -> Subscription<WindowMessage> {
    let animation = if state.is_animating() {
        time::every(ANIMATION_FRAME_INTERVAL).map(WindowMessage::AnimationFrame)
    } else {
        Subscription::none()
    };

    let close_requests = window::close_requests().map(|_| WindowMessage::CloseRequested);
    let window_events = event::listen_with(|event, _status, _window| match event {
        Event::Window(window::Event::Focused) => Some(WindowMessage::Focused),
        Event::Window(window::Event::Unfocused) => Some(WindowMessage::Unfocused),
        Event::Window(window::Event::Opened { .. } | window::Event::Resized(_)) => {
            Some(WindowMessage::CheckMaximized)
        },
        _ => None,
    });

    Subscription::batch([animation, close_requests, window_events])
}

pub fn update(
    state: &mut WindowState,
    msg: WindowMessage,
    can_hide_in_background: bool,
) -> Task<WindowMessage> {
    match msg {
        WindowMessage::Drag => with_latest(window::drag),
        WindowMessage::Resize(direction) => {
            with_latest(move |id| window::drag_resize(id, direction))
        },
        WindowMessage::Minimize => with_latest(|id| window::minimize(id, true)),
        WindowMessage::ToggleMaximize => with_latest(window::toggle_maximize),
        WindowMessage::CloseRequested => {
            state.backgrounded = true;
            background(can_hide_in_background)
        },
        WindowMessage::Focused => {
            state.backgrounded = false;
            state.focused = true;
            (state.active_border_color, state.inactive_border_color) = system_border_colors();
            Task::none()
        },
        WindowMessage::Unfocused => {
            state.focused = false;
            Task::none()
        },
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
        WindowMessage::CheckMaximized => {
            with_latest(|id| window::is_maximized(id).map(WindowMessage::MaximizedChanged))
        },
        WindowMessage::MaximizedChanged(maximized) => {
            state.maximized = maximized;
            Task::none()
        },
    }
}

pub fn view<'a, AppMessage: Clone + 'a>(
    state: &'a WindowState,
    title: &'a str,
    map_message: fn(WindowMessage) -> AppMessage,
) -> Element<'a, AppMessage> {
    let rounded = state.uses_rounded_corners();
    let drag_region = mouse_area(
        row![
            text(title).size(14).font(typography::SANS_SEMIBOLD),
            Space::new().width(Fill)
        ]
        .align_y(Vertical::Center)
        .width(Fill)
        .height(Fixed(TITLE_BAR_HEIGHT))
        .padding([0, 6]),
    )
    .on_press(map_message(WindowMessage::Drag))
    .on_double_click(map_message(WindowMessage::ToggleMaximize));

    widget::container(
        row![drag_region, controls(state).map(map_message),].align_y(Vertical::Center),
    )
    .width(Fill)
    .height(TITLE_BAR_HEIGHT)
    .padding([0, 6])
    .style(move |theme| bar_style(theme, rounded))
    .into()
}

pub fn resize_handles() -> Element<'static, WindowMessage> {
    let top = row![
        handle(
            Direction::NorthWest,
            Interaction::ResizingDiagonallyDown,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
        handle(Direction::North, Interaction::ResizingVertically, Fill),
        handle(
            Direction::NorthEast,
            Interaction::ResizingDiagonallyUp,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
    ]
    .height(Fixed(RESIZE_HANDLE_SIZE));
    let middle = row![
        handle(
            Direction::West,
            Interaction::ResizingHorizontally,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
        Space::new().width(Fill).height(Fill),
        handle(
            Direction::East,
            Interaction::ResizingHorizontally,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
    ]
    .height(Fill);
    let bottom = row![
        handle(
            Direction::SouthWest,
            Interaction::ResizingDiagonallyUp,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
        handle(Direction::South, Interaction::ResizingVertically, Fill),
        handle(
            Direction::SouthEast,
            Interaction::ResizingDiagonallyDown,
            Fixed(RESIZE_HANDLE_SIZE),
        ),
    ]
    .height(Fixed(RESIZE_HANDLE_SIZE));

    widget::container(widget::column![top, middle, bottom])
        .width(Fill)
        .height(Fill)
        .into()
}

pub fn focus_border(state: &WindowState) -> Element<'static, WindowMessage> {
    let rounded = state.uses_rounded_corners();
    let color = state.border_color();

    widget::container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |theme| focus_border_style(theme, rounded, color))
        .into()
}

pub fn show(state: &mut WindowState) -> Task<WindowMessage> {
    state.backgrounded = false;

    with_latest(|id| {
        Task::batch([
            window::set_mode(id, Mode::Windowed),
            window::minimize(id, false),
            window::gain_focus(id),
        ])
    })
}

fn handle(
    direction: Direction,
    interaction: mouse::Interaction,
    width: Length,
) -> MouseArea<'static, WindowMessage> {
    mouse_area(Space::new().width(width).height(Fill))
        .on_press(WindowMessage::Resize(direction))
        .interaction(interaction)
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
            WindowMessage::CloseRequested,
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

    tooltip(control, text(label).size(15), tooltip::Position::Bottom)
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
    let progress = if status == button::Status::Pressed {
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

    if status == button::Status::Disabled {
        style.text_color = style.text_color.scale_alpha(0.45);
    }

    style
}

fn bar_style(theme: &Theme, rounded: bool) -> Style {
    let colors = theme.extended_palette().background.weaker;

    Style::default()
        .background(Background::Color(colors.color))
        .color(colors.text)
        .border(Border {
            radius: if rounded {
                Radius {
                    top_left: WINDOW_CORNER_RADIUS,
                    top_right: WINDOW_CORNER_RADIUS,
                    ..Radius::default()
                }
            } else {
                Radius::default()
            },
            ..Border::default()
        })
}

fn focus_border_style(_theme: &Theme, rounded: bool, color: Color) -> Style {
    Style {
        border: Border {
            color: if !rounded { Color::TRANSPARENT } else { color },
            width: if rounded { 1.0 } else { 0.0 },
            radius: if rounded {
                WINDOW_CORNER_RADIUS.into()
            } else {
                iced::border::Radius::default()
            },
        },
        ..Style::default()
    }
}

#[cfg(target_os = "windows")]
fn system_border_colors() -> (Color, Color) {
    use winapi::um::winuser::{COLOR_ACTIVEBORDER, COLOR_INACTIVEBORDER, GetSysColor};

    // SAFETY: GetSysColor accepts a constant system color index and has no pointer arguments.
    let active = unsafe { GetSysColor(COLOR_ACTIVEBORDER) };
    // SAFETY: GetSysColor accepts a constant system color index and has no pointer arguments.
    let inactive = unsafe { GetSysColor(COLOR_INACTIVEBORDER) };

    (colorref_to_color(active), colorref_to_color(inactive))
}

#[cfg(target_os = "windows")]
fn colorref_to_color(color: u32) -> Color {
    Color::from_rgb8(
        (color & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        ((color >> 16) & 0xff) as u8,
    )
}

#[cfg(not(target_os = "windows"))]
fn system_border_colors() -> (Color, Color) {
    (
        Color::from_rgb8(0x80, 0x80, 0x80),
        Color::from_rgb8(0x40, 0x40, 0x40),
    )
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

fn background(can_hide: bool) -> Task<WindowMessage> {
    if can_hide {
        with_latest(|id| window::set_mode(id, Mode::Hidden))
    } else {
        with_latest(|id| window::minimize(id, true))
    }
}

fn with_latest(
    operation: impl Fn(Id) -> Task<WindowMessage> + Send + 'static,
) -> Task<WindowMessage> {
    window::latest().and_then(operation)
}

#[cfg(test)]
mod tests {
    use iced::{Color, Theme};

    use super::{WindowControl, WindowMessage, WindowState, focus_border_style, settings, update};

    #[test]
    fn settings_enable_window_resizing() {
        let settings = settings();

        assert!(settings.resizable);
        assert!(settings.transparent);
        assert!(settings.icon.is_some());
    }

    #[test]
    fn maximized_windows_disable_rounded_corners() {
        let mut state = WindowState::default();
        assert!(state.uses_rounded_corners());

        drop(update(
            &mut state,
            WindowMessage::MaximizedChanged(true),
            false,
        ));

        assert!(!state.uses_rounded_corners());
        assert_eq!(
            focus_border_style(&Theme::KanagawaWave, false, Color::BLACK)
                .border
                .width,
            0.0
        );
    }

    #[test]
    fn hover_messages_change_the_matching_animation_target() {
        let mut state = WindowState::default();

        drop(update(
            &mut state,
            WindowMessage::Hover(WindowControl::Close, true),
            false,
        ));

        assert!(state.close_hover.value());
        assert!(!state.minimize_hover.value());
        assert!(!state.maximize_hover.value());
    }

    #[test]
    fn close_request_backgrounds_until_the_window_is_focused() {
        let mut state = WindowState::default();

        drop(update(&mut state, WindowMessage::CloseRequested, false));
        assert!(state.is_backgrounded());

        drop(update(&mut state, WindowMessage::Focused, false));
        assert!(!state.is_backgrounded());
    }

    #[test]
    fn focus_events_update_the_window_border_state() {
        let mut state = WindowState::default();
        assert!(state.focused);

        drop(update(&mut state, WindowMessage::Unfocused, false));
        assert!(!state.focused);

        drop(update(&mut state, WindowMessage::Focused, false));
        assert!(state.focused);
    }
}
