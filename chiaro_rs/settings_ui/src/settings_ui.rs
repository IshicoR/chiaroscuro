use chiaro_actions::Action;
use chiaro_telemetry::{ConnectionStatus, LiveTelemetrySourceInfo, Session};
use chiaro_widgets::{ButtonVariant, button, callout, dialog, panel, toggler_style, typography};
use iced::{
    Color, Element,
    widget::{column, row, text, toggler},
};

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    show_diagnostics: bool,
    dialog_preview_open: bool,
}

impl SettingsState {
    pub fn show_diagnostics(&self) -> bool {
        self.show_diagnostics
    }

    pub fn set_show_diagnostics(&mut self, show_diagnostics: bool) {
        self.show_diagnostics = show_diagnostics;
    }

    pub fn is_dialog_preview_open(&self) -> bool {
        self.dialog_preview_open
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsMessage {
    SetDiagnostics(bool),
    OpenDialogPreview,
    CloseDialogPreview,
    ConfirmDialogPreview,
}

impl SettingsMessage {
    pub const fn persists_configuration(self) -> bool {
        matches!(self, Self::SetDiagnostics(_))
    }
}

pub fn update(state: &mut SettingsState, message: SettingsMessage) -> Option<Action> {
    match message {
        SettingsMessage::SetDiagnostics(enabled) => {
            state.show_diagnostics = enabled;
            None
        },
        SettingsMessage::OpenDialogPreview => {
            state.dialog_preview_open = true;
            None
        },
        SettingsMessage::CloseDialogPreview | SettingsMessage::ConfirmDialogPreview => {
            state.dialog_preview_open = false;
            None
        },
    }
}

pub fn view<'a>(
    state: &'a SettingsState,
    session: &'a Session,
    config_error: Option<&'a str>,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, SettingsMessage> {
    let diagnostics_toggle = toggler(state.show_diagnostics)
        .label("Show diagnostics")
        .size(20)
        .spacing(12)
        .style(toggler_style)
        .on_toggle(SettingsMessage::SetDiagnostics);
    let mut diagnostics = column![
        text("Diagnostics").size(20).font(typography::SANS_SEMIBOLD),
        text("Expose live telemetry and recording details for troubleshooting.").size(15),
        diagnostics_toggle,
    ]
    .spacing(10);

    if state.show_diagnostics {
        let connection = if session.ibt_info().is_some() {
            "IBT recording"
        } else {
            if !live_source.is_available() {
                "Unavailable on this platform"
            } else {
                match session.connection() {
                    ConnectionStatus::Disconnected => "Disconnected",
                    ConnectionStatus::Connecting => "Connecting",
                    ConnectionStatus::Connected => "Connected",
                }
            }
        };
        let sample_count = session.ibt_info().map_or_else(
            || format!("Samples received: {}", session.packets_received()),
            |info| format!("Records: {}", info.record_count),
        );

        diagnostics = diagnostics
            .push(text("Runtime").size(16).font(typography::SANS_SEMIBOLD))
            .push(text(format!(
                "Live source: {} ({})",
                live_source.display_name(),
                live_source.id()
            )))
            .push(text(format!("Connection: {connection}")))
            .push(text(sample_count))
            .push(text(format!(
                "Telemetry variables: {}",
                session
                    .latest_frame()
                    .map_or(0, chiaro_irsdk::TelemetryFrame::len)
            )));

        if let Some(reason) = live_source.unavailable_reason() {
            diagnostics = diagnostics.push(text(format!("Live source unavailable: {reason}")));
        }

        if let Some(info) = session.ibt_info() {
            diagnostics = diagnostics.push(
                text(format!("IBT source: {}", info.source.description()))
                    .width(iced::Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            );
        }

        if let Some(info) = session.session_info() {
            diagnostics =
                diagnostics.push(text(format!("Session info update: {}", info.update_count)));
        }

        if let Some(error) = session.last_error() {
            diagnostics = diagnostics.push(text(format!("Telemetry error: {error}")));
        }
    }

    let mut content = column![
        text("Settings").size(32).font(typography::SANS_SEMIBOLD),
        text("Application preferences and diagnostics").size(16),
        panel(diagnostics).padding(16).width(iced::Length::Fill),
    ]
    .spacing(16);

    if let Some(error) = config_error {
        content = content.push(
            callout(
                text(format!("Configuration error: {error}"))
                    .color(Color::from_rgb8(0xFA, 0x4D, 0x56)),
            )
            .padding(12)
            .width(iced::Length::Fill),
        );
    }

    let component_preview = panel(
        column![
            text("Component preview")
                .size(20)
                .font(typography::SANS_SEMIBOLD),
            text("Open the shared modal to verify its layout and dismissal behavior.").size(15),
            button(text("Open dialog preview"))
                .variant(ButtonVariant::Outline)
                .on_press(SettingsMessage::OpenDialogPreview),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(iced::Length::Fill);
    let about = panel(
        column![
            text("About").size(20).font(typography::SANS_SEMIBOLD),
            text("Chiaroscuro").size(18).font(typography::SANS_SEMIBOLD),
            text("Desktop telemetry interface").size(15),
            text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(15),
        ]
        .spacing(6),
    )
    .padding(16)
    .width(iced::Length::Fill);

    content = content.push(component_preview).push(about);

    content.into()
}

/// Wraps the complete application view with the Settings dialog preview.
///
/// The complete view must be supplied as `base` so the modal barrier also
/// covers navigation and window chrome.
pub fn with_dialog_preview<'a, Message, Map>(
    base: impl Into<Element<'a, Message>>,
    state: &SettingsState,
    map: Map,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Map: Fn(SettingsMessage) -> Message + Copy + 'a,
{
    let body = column![
        text("The Settings view stays mounted behind this surface."),
        text("Verify the close icon, Cancel button, Escape key, and backdrop click."),
        text("Navigation and window controls should remain inactive while the dialog is open."),
    ]
    .spacing(10);
    let footer = row![
        button(text("Cancel"))
            .variant(ButtonVariant::Outline)
            .on_press(map(SettingsMessage::CloseDialogPreview)),
        button(text("Confirm")).on_press(map(SettingsMessage::ConfirmDialogPreview)),
    ]
    .spacing(8);

    dialog(base, body)
        .open(state.is_dialog_preview_open())
        .title("Dialog preview")
        .description("A live verification of the shared Chiaro dialog component.")
        .footer(footer)
        .width(520)
        .close_label("Close dialog")
        .on_close(map(SettingsMessage::CloseDialogPreview))
        .into()
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

    #[test]
    fn dialog_preview_is_controlled_by_settings_messages() {
        let mut state = SettingsState::default();

        let _action = update(&mut state, SettingsMessage::OpenDialogPreview);
        assert!(state.is_dialog_preview_open());

        let _action = update(&mut state, SettingsMessage::CloseDialogPreview);
        assert!(!state.is_dialog_preview_open());
    }

    #[test]
    fn preview_messages_do_not_persist_configuration() {
        assert!(SettingsMessage::SetDiagnostics(true).persists_configuration());
        assert!(!SettingsMessage::OpenDialogPreview.persists_configuration());
        assert!(!SettingsMessage::CloseDialogPreview.persists_configuration());
        assert!(!SettingsMessage::ConfirmDialogPreview.persists_configuration());
    }
}
