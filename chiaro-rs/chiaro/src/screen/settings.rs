use iced::{
    Element,
    widget::{column, text, toggler},
};

use crate::{
    action::Action,
    appearance::{self, Mode},
    session::{ConnectionStatus, Session},
};

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    show_diagnostics: bool,
}

impl SettingsState {
    pub fn show_diagnostics(&self) -> bool {
        self.show_diagnostics
    }

    pub fn set_show_diagnostics(&mut self, show_diagnostics: bool) {
        self.show_diagnostics = show_diagnostics;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsMessage {
    SetDark(bool),
    SetDiagnostics(bool),
}

pub fn update(state: &mut SettingsState, message: SettingsMessage) -> Option<Action> {
    match message {
        SettingsMessage::SetDark(enabled) => Some(Action::SetTheme(if enabled {
            Mode::Dark
        } else {
            Mode::Light
        })),
        SettingsMessage::SetDiagnostics(enabled) => {
            state.show_diagnostics = enabled;
            None
        },
    }
}

pub fn view<'a>(
    state: &'a SettingsState,
    theme: appearance::Mode,
    session: &'a Session,
    configuration_error: Option<&'a str>,
) -> Element<'a, SettingsMessage> {
    let mut content = column![
        text("Settings").size(28),
        text("Appearance").size(18),
        toggler(theme == appearance::Mode::Dark)
            .label("Dark theme")
            .on_toggle(SettingsMessage::SetDark),
        text("Diagnostics").size(18),
        toggler(state.show_diagnostics)
            .label("Show diagnostics")
            .on_toggle(SettingsMessage::SetDiagnostics),
    ];

    if state.show_diagnostics {
        let connection = match session.connection() {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting",
            ConnectionStatus::Connected => "Connected",
        };

        content = content
            .push(text("Runtime").size(18))
            .push(text(format!("Connection: {connection}")))
            .push(text(format!("Server: {}", session.server_addr())))
            .push(text(format!(
                "Packets received: {}",
                session.packets_received()
            )));

        if let Some(error) = session.last_error() {
            content = content.push(text(format!("Telemetry error: {error}")));
        }
    }

    if let Some(error) = configuration_error {
        content = content.push(text(format!("Configuration error: {error}")));
    }

    content.spacing(16).into()
}

#[cfg(test)]
mod tests {
    use super::{SettingsMessage, SettingsState, update};

    #[test]
    fn diagnostics_setting_is_updated() {
        let mut state = SettingsState::default();

        let _action = update(&mut state, SettingsMessage::SetDiagnostics(true));

        assert!(state.show_diagnostics());
    }
}
