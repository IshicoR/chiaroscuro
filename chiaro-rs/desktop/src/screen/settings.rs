use iced::{
    Element, Task,
    widget::{column, text, toggler},
};

use crate::{action::Action, appearance};

#[derive(Debug, Clone, Default)]
pub struct State {
    show_diagnostics: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    SetDark(bool),
    SetDiagnostics(bool),
}

pub fn update(state: &mut State, message: Message) -> (Task<Message>, Option<Action>) {
    let action = match message {
        Message::SetDark(enabled) => Some(Action::SetTheme(if enabled {
            appearance::Mode::Dark
        } else {
            appearance::Mode::Light
        })),
        Message::SetDiagnostics(enabled) => {
            state.show_diagnostics = enabled;
            None
        },
    };

    (Task::none(), action)
}

pub fn view(state: &State, theme: appearance::Mode) -> Element<'_, Message> {
    column![
        text("Settings").size(28),
        text("Appearance").size(18),
        toggler(theme == appearance::Mode::Dark)
            .label("Dark theme")
            .on_toggle(Message::SetDark),
        text("Diagnostics").size(18),
        toggler(state.show_diagnostics)
            .label("Show diagnostics")
            .on_toggle(Message::SetDiagnostics),
    ]
    .spacing(16)
    .into()
}
