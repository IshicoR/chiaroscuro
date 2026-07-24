use ::image::imageops::FilterType::Lanczos3;
use chiaro_widgets::{WindowControlKind, window_control};
use iced::{
    Background, Border, Element, Event,
    Length::{self, Fill, Fixed},
    Padding, Size, Subscription, Task, Theme,
    alignment::Vertical,
    border::Radius,
    event,
    mouse::{self, Interaction},
    widget::{self, MouseArea, Space, container::Style, image, mouse_area, row},
    window::{self, Direction, Id, Mode, Settings, icon},
};

/// Default width of the application window in pixels
const INITIAL_WINDOW_WIDTH: f32 = 1380.0;

/// Default height of the application window in pixels
const INITIAL_WINDOW_HEIGHT: f32 = 720.0;

/// Minimum width of the application window in pixels
const MIN_WINDOW_WIDTH: f32 = 960.0;

/// Minimum height of the application window in pixels
const MIN_WINDOW_HEIGHT: f32 = 640.0;

/// Spacing between window control buttons (minimize, maximize, close)
const WINDOW_CONTROL_SPACING: f32 = 6.0;

/// Corner radius for the window's rounded corners
const WINDOW_CORNER_RADIUS: f32 = 10.0;

/// Size of the resize handle area in pixels
const RESIZE_HANDLE_SIZE: f32 = 6.0;

/// Right padding for the title bar area
const TITLE_BAR_RIGHT_PADDING: f32 = 6.0;

/// Height of the application title bar
const TITLE_BAR_HEIGHT: f32 = 40.0;

/// Display size of the title bar logo in pixels
const TITLE_BAR_LOGO_SIZE: f32 = 24.0;

/// Pixel dimensions of the title bar logo image
const TITLE_BAR_LOGO_PIXELS: u32 = 40;

/// Corner radius for the title bar logo
const TITLE_BAR_LOGO_RADIUS: f32 = 5.0;

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo.png");

const WINDOW_ICON_SIZE: u32 = 256;

#[derive(Debug, Clone)]
pub struct WindowState {
    backgrounded: bool,
    maximized: bool,
    logo: Option<image::Handle>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            backgrounded: false,
            maximized: false,
            logo: title_bar_logo(),
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
    CheckMaximized,
    MaximizedChanged(bool),
}

pub fn settings() -> Settings {
    Settings {
        size: Size::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT),
        min_size: Some(Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)),
        resizable: true,
        decorations: false,
        transparent: true,
        icon: window_icon(),
        exit_on_close_request: false,
        ..window::Settings::default()
    }
}

fn window_icon() -> Option<window::Icon> {
    let logo = ::image::load_from_memory(LOGO_BYTES)
        .ok()?
        .resize_exact(WINDOW_ICON_SIZE, WINDOW_ICON_SIZE, Lanczos3)
        .into_rgba8();
    let (width, height) = logo.dimensions();

    icon::from_rgba(logo.into_raw(), width, height).ok()
}

fn title_bar_logo() -> Option<image::Handle> {
    let logo = ::image::load_from_memory(LOGO_BYTES)
        .ok()?
        .resize_exact(
            TITLE_BAR_LOGO_PIXELS,
            TITLE_BAR_LOGO_PIXELS,
            ::image::imageops::FilterType::Lanczos3,
        )
        .into_rgba8();

    Some(image::Handle::from_rgba(
        TITLE_BAR_LOGO_PIXELS,
        TITLE_BAR_LOGO_PIXELS,
        logo.into_raw(),
    ))
}

pub fn subscription() -> Subscription<WindowMessage> {
    let close_requests = window::close_requests().map(|_| WindowMessage::CloseRequested);
    let window_events = event::listen_with(|event, _, _| match event {
        Event::Window(window::Event::Focused) => Some(WindowMessage::Focused),
        Event::Window(window::Event::Unfocused) => Some(WindowMessage::Unfocused),
        Event::Window(window::Event::Opened { .. } | window::Event::Resized(_)) => {
            Some(WindowMessage::CheckMaximized)
        },
        _ => None,
    });

    Subscription::batch([close_requests, window_events])
}

pub fn update(
    state: &mut WindowState,
    msg: WindowMessage,
    is_background: bool,
) -> Task<WindowMessage> {
    match msg {
        WindowMessage::Drag => with_latest(window::drag),
        WindowMessage::Resize(direction) => {
            with_latest(move |id| window::drag_resize(id, direction))
        },
        WindowMessage::Minimize => with_latest(|id| window::minimize(id, true)),
        WindowMessage::ToggleMaximize => with_latest(window::toggle_maximize),
        WindowMessage::CloseRequested => {
            state.backgrounded = is_background;
            if is_background {
                with_latest(|id| window::set_mode(id, Mode::Hidden))
            } else {
                iced::exit()
            }
        },
        WindowMessage::Focused => {
            state.backgrounded = false;
            Task::none()
        },
        WindowMessage::Unfocused => Task::none(),
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
    brand_width: f32,
    map_msg: fn(WindowMessage) -> AppMessage,
) -> Element<'a, AppMessage> {
    let rounded = state.uses_rounded_corners();
    let logo: Element<'_, AppMessage> = match &state.logo {
        Some(handle) => image(handle.clone())
            .width(TITLE_BAR_LOGO_SIZE)
            .height(TITLE_BAR_LOGO_SIZE)
            .border_radius(TITLE_BAR_LOGO_RADIUS)
            .into(),
        None => Space::new()
            .width(TITLE_BAR_LOGO_SIZE)
            .height(TITLE_BAR_LOGO_SIZE)
            .into(),
    };
    let brand = widget::container(logo)
        .center_x(Fixed(brand_width))
        .center_y(Fixed(TITLE_BAR_HEIGHT));
    let brand = drag_region(brand, map_msg);
    let space = drag_region(
        Space::new().width(Fill).height(Fixed(TITLE_BAR_HEIGHT)),
        map_msg,
    );
    let controls = widget::container(controls().map(map_msg))
        .height(Fill)
        .padding(Padding {
            right: TITLE_BAR_RIGHT_PADDING,
            ..Padding::ZERO
        })
        .align_y(Vertical::Center);
    let content = row![brand, space, controls].align_y(Vertical::Center);

    widget::container(content)
        .width(Fill)
        .height(TITLE_BAR_HEIGHT)
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

fn drag_region<'a, AppMessage: Clone + 'a>(
    content: impl Into<Element<'a, AppMessage>>,
    map_msg: fn(WindowMessage) -> AppMessage,
) -> MouseArea<'a, AppMessage> {
    mouse_area(content)
        .on_press(map_msg(WindowMessage::Drag))
        .on_double_click(map_msg(WindowMessage::ToggleMaximize))
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

fn controls() -> Element<'static, WindowMessage> {
    row![
        window_control(WindowControlKind::Minimize, WindowMessage::Minimize,),
        window_control(WindowControlKind::Maximize, WindowMessage::ToggleMaximize,),
        window_control(WindowControlKind::Close, WindowMessage::CloseRequested,),
    ]
    .align_y(Vertical::Center)
    .spacing(WINDOW_CONTROL_SPACING)
    .into()
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

fn with_latest(op: impl Fn(Id) -> Task<WindowMessage> + Send + 'static) -> Task<WindowMessage> {
    window::latest().and_then(op)
}

#[cfg(test)]
mod tests {
    use super::{WindowMessage, WindowState, settings, update};

    #[test]
    fn title_bar_logo_is_predecoded_and_cached_in_window_state() {
        let state = WindowState::default();

        assert!(matches!(
            state.logo,
            Some(iced::widget::image::Handle::Rgba {
                width: 40,
                height: 40,
                ..
            })
        ));
    }

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
    }

    #[test]
    fn close_request_backgrounds_only_when_a_tray_can_restore_it() {
        let mut state = WindowState::default();

        drop(update(&mut state, WindowMessage::CloseRequested, true));
        assert!(state.is_backgrounded());

        drop(update(&mut state, WindowMessage::Focused, false));
        assert!(!state.is_backgrounded());

        drop(update(&mut state, WindowMessage::CloseRequested, false));
        assert!(!state.is_backgrounded());
    }
}
