use iced::{
    Element,
    widget::{button, column, text},
};

use crate::{action::Action, appearance, navigation::Screen};

#[derive(Debug, Clone, Default)]
pub struct AboutState;

#[derive(Debug, Clone, Copy)]
pub enum AboutMessage {
    OpenDashboard,
}

pub fn update(_state: &mut AboutState, message: AboutMessage) -> Option<Action> {
    Some(match message {
        AboutMessage::OpenDashboard => Action::Navigate(Screen::Dashboard),
    })
}

pub fn view(_state: &AboutState) -> Element<'_, AboutMessage> {
    column![
        text("Chiaroscuro").size(28),
        text("Desktop telemetry interface").size(14),
        text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14),
        button("Back to dashboard")
            .style(appearance::action_button)
            .on_press(AboutMessage::OpenDashboard),
    ]
    .spacing(16)
    .into()
}
