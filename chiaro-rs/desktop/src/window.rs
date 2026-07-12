use iced::{
    Element,
    Length::Fill,
    Size, Subscription, Task,
    alignment::Vertical,
    widget::{container, mouse_area, row, text},
    window::{self as iced_window, Id},
};

use crate::{appearance, widget::window_controls};

#[derive(Debug, Clone)]
pub struct State {
    size: Size,
}

impl Default for State {
    fn default() -> Self {
        Self {
            size: Size::new(960.0, 640.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Resized(Id, Size),
    CloseRequested(Id),
    Drag,
    ToggleMaximize,
    Control(window_controls::Message),
}

pub fn subscription() -> Subscription<Message> {
    Subscription::batch([
        iced_window::resize_events().map(|(id, size)| Message::Resized(id, size)),
        iced_window::close_requests().map(Message::CloseRequested),
    ])
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Resized(_id, size) => {
            state.size = size;
            Task::none()
        },
        Message::CloseRequested(id) => iced_window::close(id),
        Message::Drag => with_latest(iced_window::drag),
        Message::ToggleMaximize => with_latest(iced_window::toggle_maximize),
        Message::Control(message) => match message {
            window_controls::Message::Minimize => with_latest(|id| iced_window::minimize(id, true)),
            window_controls::Message::ToggleMaximize => with_latest(iced_window::toggle_maximize),
            window_controls::Message::Close => with_latest(iced_window::close),
        },
    }
}

pub fn title_bar<'a, AppMessage: Clone + 'a>(
    leading: Element<'a, AppMessage>,
    show_title: bool,
    map_message: fn(Message) -> AppMessage,
) -> Element<'a, AppMessage> {
    let title = if show_title { "CHIAROSCURO" } else { "" };
    let drag_region = mouse_area(
        container(text(title).size(13))
            .width(Fill)
            .height(appearance::TITLE_BAR_HEIGHT)
            .center_y(appearance::TITLE_BAR_HEIGHT)
            .padding([0, 16]),
    )
    .on_press(map_message(Message::Drag))
    .on_double_click(map_message(Message::ToggleMaximize));

    container(
        row![
            leading,
            drag_region,
            window_controls::view().map(move |message| map_message(Message::Control(message))),
        ]
        .align_y(Vertical::Center),
    )
    .width(Fill)
    .height(appearance::TITLE_BAR_HEIGHT)
    .style(appearance::title_bar)
    .into()
}

pub fn close() -> Task<Message> {
    with_latest(iced_window::close)
}

fn with_latest(operation: impl Fn(Id) -> Task<Message> + Send + 'static) -> Task<Message> {
    iced_window::latest().and_then(operation)
}
