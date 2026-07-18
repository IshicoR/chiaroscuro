use chiaro_actions::Action;
use chiaro_theme::typography;
use chiaro_telemetry::{ConnectionStatus, Session};
use iced::{
    Element,
    widget::{column, rule, text, toggler},
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
    SetDiagnostics(bool),
}

pub fn update(state: &mut SettingsState, message: SettingsMessage) -> Option<Action> {
    match message {
        SettingsMessage::SetDiagnostics(enabled) => {
            state.show_diagnostics = enabled;
            None
        },
    }
}

pub fn view<'a>(
    state: &'a SettingsState,
    session: &'a Session,
    config_error: Option<&'a str>,
) -> Element<'a, SettingsMessage> {
    let mut content = column![
        text("Settings").size(32).font(typography::SANS_SEMIBOLD),
        text("Diagnostics").size(22).font(typography::SANS_SEMIBOLD),
        toggler(state.show_diagnostics)
            .label("Show diagnostics")
            .on_toggle(SettingsMessage::SetDiagnostics),
    ];

    if state.show_diagnostics {
        let connection = if session.ibt_info().is_some() {
            "IBT recording"
        } else {
            match session.connection() {
                ConnectionStatus::Disconnected => "Disconnected",
                ConnectionStatus::Connecting => "Connecting",
                ConnectionStatus::Connected => "Connected",
            }
        };
        let sample_count = session.ibt_info().map_or_else(
            || format!("Samples received: {}", session.packets_received()),
            |info| format!("Records: {}", info.record_count),
        );

        content = content
            .push(text("Runtime").size(22).font(typography::SANS_SEMIBOLD))
            .push(text(format!("Connection: {connection}")))
            .push(text(sample_count))
            .push(text(format!(
                "Telemetry variables: {}",
                session
                    .latest_frame()
                    .map_or(0, chiaro_irsdk::TelemetryFrame::len)
            )));

        if let Some(info) = session.ibt_info() {
            content = content.push(
                text(format!("IBT file: {}", info.path.display()))
                    .width(iced::Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            );
        }

        if let Some(info) = session.session_info() {
            content = content.push(text(format!("Session info update: {}", info.update_count)));
        }

        if let Some(error) = session.last_error() {
            content = content.push(text(format!("Telemetry error: {error}")));
        }
    }

    if let Some(error) = config_error {
        content = content.push(text(format!("Configuration error: {error}")));
    }

    content = content
        .push(rule::horizontal(1))
        .push(text("About").size(22).font(typography::SANS_SEMIBOLD))
        .push(text("Chiaroscuro").size(20).font(typography::SANS_SEMIBOLD))
        .push(text("Desktop telemetry interface").size(18))
        .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(18));

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
