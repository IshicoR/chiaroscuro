use chiaro_widgets::{WindowControlKind, tabs::BAR_HEIGHT as TITLE_BAR_HEIGHT, window_control};
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

const WINDOW_CORNER_RADIUS: f32 = 10.0;
const RESIZE_HANDLE_SIZE: f32 = 6.0;
const TITLE_BAR_LOGO_SIZE: f32 = 20.0;
const TITLE_BAR_LOGO_PIXELS: u32 = 40;
const TITLE_BAR_LOGO_RADIUS: f32 = 4.0;
const TITLE_BAR_LOGO_RIGHT_PADDING: f32 = 4.0;
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
        size: Size::new(1380.0, 720.0),
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
    let logo = ::image::load_from_memory(LOGO_BYTES).ok()?.resize_exact(
        WINDOW_ICON_SIZE,
        WINDOW_ICON_SIZE,
        ::image::imageops::FilterType::Lanczos3,
    );
    let logo = logo.into_rgba8();
    let (width, height) = logo.dimensions();

    icon::from_rgba(logo.into_raw(), width, height).ok()
}

fn title_bar_logo() -> Option<image::Handle> {
    let logo = ::image::load_from_memory(LOGO_BYTES).ok()?.resize_exact(
        TITLE_BAR_LOGO_PIXELS,
        TITLE_BAR_LOGO_PIXELS,
        ::image::imageops::FilterType::Lanczos3,
    );
    let logo = logo.into_rgba8();

    Some(image::Handle::from_rgba(
        TITLE_BAR_LOGO_PIXELS,
        TITLE_BAR_LOGO_PIXELS,
        logo.into_raw(),
    ))
}

pub fn subscription() -> Subscription<WindowMessage> {
    let close_requests = window::close_requests().map(|_| WindowMessage::CloseRequested);
    let window_events = event::listen_with(|event, _status, _window| match event {
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
            state.backgrounded = can_hide_in_background;
            if can_hide_in_background {
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
    tab_bar: Option<Element<'a, AppMessage>>,
    map_message: fn(WindowMessage) -> AppMessage,
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
        .center_y(Fixed(TITLE_BAR_HEIGHT))
        .padding(Padding {
            right: TITLE_BAR_LOGO_RIGHT_PADDING,
            ..Padding::ZERO
        });
    let brand = drag_region(brand, map_message);
    let remaining_space = drag_region(
        Space::new().width(Fill).height(Fixed(TITLE_BAR_HEIGHT)),
        map_message,
    );
    let controls = widget::container(controls().map(map_message))
        .height(Fill)
        .padding(Padding {
            right: 6.0,
            ..Padding::ZERO
        })
        .align_y(Vertical::Center);
    let mut content = row![brand];
    if let Some(tab_bar) = tab_bar {
        content = content.push(tab_bar);
    }
    let content = content
        .push(remaining_space)
        .push(controls)
        .align_y(Vertical::Center);

    widget::container(content)
        .width(Fill)
        .height(TITLE_BAR_HEIGHT)
        .style(move |theme| bar_style(theme, rounded))
        .into()
}

fn drag_region<'a, AppMessage: Clone + 'a>(
    content: impl Into<Element<'a, AppMessage>>,
    map_message: fn(WindowMessage) -> AppMessage,
) -> MouseArea<'a, AppMessage> {
    mouse_area(content)
        .on_press(map_message(WindowMessage::Drag))
        .on_double_click(map_message(WindowMessage::ToggleMaximize))
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
    .spacing(0)
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

fn with_latest(
    operation: impl Fn(Id) -> Task<WindowMessage> + Send + 'static,
) -> Task<WindowMessage> {
    window::latest().and_then(operation)
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
