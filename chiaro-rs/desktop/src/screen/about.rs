use iced::{
    Element, Task,
    widget::{button, column, text},
};

use crate::{action::Action, appearance, navigation::Page};

#[derive(Debug, Clone, Default)]
pub struct State;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    OpenDashboard,
}

pub fn update(_state: &mut State, message: Message) -> (Task<Message>, Option<Action>) {
    let action = match message {
        Message::OpenDashboard => Action::Navigate(Page::Dashboard),
    };

    (Task::none(), Some(action))
}

pub fn view(_state: &State) -> Element<'_, Message> {
    column![
        text("Chiaroscuro").size(28),
        text("Desktop telemetry interface").size(14),
        text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14),
        button("Back to dashboard")
            .style(appearance::action_button)
            .on_press(Message::OpenDashboard),
    ]
    .spacing(16)
    .into()
}
