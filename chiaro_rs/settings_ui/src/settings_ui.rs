use chiaro_actions::Action;
use chiaro_i18n::{Locale, Text, Translations, tr};
use chiaro_telemetry::{ConnectionStatus, LiveTelemetrySourceInfo, Session};
use chiaro_widgets::{ButtonVariant, button, callout, dialog, panel, toggler_style, typography};
use iced::{
    Color, Element,
    widget::{column, pick_list, row, text, toggler},
};

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    show_diagnostics: bool,
    locale: Locale,
    dialog_preview_open: bool,
}

impl SettingsState {
    pub fn show_diagnostics(&self) -> bool {
        self.show_diagnostics
    }

    pub fn set_show_diagnostics(&mut self, show_diagnostics: bool) {
        self.show_diagnostics = show_diagnostics;
    }

    pub const fn locale(&self) -> Locale {
        self.locale
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
        chiaro_i18n::set_locale(locale);
    }

    pub fn is_dialog_preview_open(&self) -> bool {
        self.dialog_preview_open
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsMessage {
    SetDiagnostics(bool),
    SetLocale(Locale),
    OpenDialogPreview,
    CloseDialogPreview,
    ConfirmDialogPreview,
}

impl SettingsMessage {
    pub const fn persists_configuration(self) -> bool {
        matches!(self, Self::SetDiagnostics(_) | Self::SetLocale(_))
    }
}

pub fn update(state: &mut SettingsState, message: SettingsMessage) -> Option<Action> {
    match message {
        SettingsMessage::SetDiagnostics(enabled) => {
            state.show_diagnostics = enabled;
            None
        },
        SettingsMessage::SetLocale(locale) => {
            state.set_locale(locale);
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
    let translations = Translations::new(state.locale);
    let diagnostics_toggle = toggler(state.show_diagnostics)
        .label(translations.get(Text::ShowDiagnostics))
        .size(20)
        .spacing(12)
        .style(toggler_style)
        .on_toggle(SettingsMessage::SetDiagnostics);
    let mut diagnostics = column![
        text(translations.get(Text::Diagnostics))
            .size(20)
            .font(typography::SANS_SEMIBOLD),
        text(translations.get(Text::DiagnosticsDescription)).size(15),
        diagnostics_toggle,
    ]
    .spacing(10);

    if state.show_diagnostics {
        let connection = if session.ibt_info().is_some() {
            translations.get(Text::IbtRecording)
        } else {
            if !live_source.is_available() {
                translations.get(Text::UnavailableOnThisPlatform)
            } else {
                match session.connection() {
                    ConnectionStatus::Disconnected => translations.get(Text::Disconnected),
                    ConnectionStatus::Connecting => translations.get(Text::Connecting),
                    ConnectionStatus::Connected => translations.get(Text::Connected),
                }
            }
        };
        let sample_count = session.ibt_info().map_or_else(
            || {
                format!(
                    "{}: {}",
                    tr(Text::SamplesReceived),
                    session.packets_received()
                )
            },
            |info| format!("{}: {}", tr(Text::Records), info.record_count),
        );

        diagnostics = diagnostics
            .push(
                text(translations.get(Text::Runtime))
                    .size(16)
                    .font(typography::SANS_SEMIBOLD),
            )
            .push(text(format!(
                "{}: {} ({})",
                tr(Text::LiveSource),
                live_source.display_name(),
                live_source.id()
            )))
            .push(text(format!("{}: {connection}", tr(Text::Connection))))
            .push(text(sample_count))
            .push(text(format!(
                "{}: {}",
                tr(Text::TelemetryVariables),
                session
                    .latest_frame()
                    .map_or(0, chiaro_irsdk::TelemetryFrame::len)
            )));

        if let Some(reason) = live_source.unavailable_reason() {
            diagnostics = diagnostics.push(text(format!(
                "{}: {reason}",
                tr(Text::LiveSourceUnavailable)
            )));
        }

        if let Some(info) = session.ibt_info() {
            diagnostics = diagnostics.push(
                text(format!(
                    "{}: {}",
                    tr(Text::IbtSource),
                    info.source.description()
                ))
                .width(iced::Length::Fill)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            );
        }

        if let Some(info) = session.session_info() {
            diagnostics = diagnostics.push(text(format!(
                "{}: {}",
                tr(Text::SessionInfoUpdate),
                info.update_count
            )));
        }

        if let Some(error) = session.last_error() {
            diagnostics = diagnostics.push(text(format!("{}: {error}", tr(Text::TelemetryError))));
        }
    }

    let mut content = column![
        text(translations.get(Text::Settings))
            .size(32)
            .font(typography::SANS_SEMIBOLD),
        text(translations.get(Text::ApplicationPreferences)).size(16),
        row![
            text(translations.get(Text::Language)),
            pick_list(Locale::ALL, Some(state.locale), SettingsMessage::SetLocale)
        ],
        panel(diagnostics).padding(16).width(iced::Length::Fill),
    ]
    .spacing(16);

    if let Some(error) = config_error {
        content = content.push(
            callout(
                text(format!("{}: {error}", tr(Text::ConfigurationError)))
                    .color(Color::from_rgb8(0xFA, 0x4D, 0x56)),
            )
            .padding(12)
            .width(iced::Length::Fill),
        );
    }

    let component_preview = panel(
        column![
            text(translations.get(Text::ComponentPreview))
                .size(20)
                .font(typography::SANS_SEMIBOLD),
            text(translations.get(Text::ComponentPreviewDescription)).size(15),
            button(text(translations.get(Text::OpenDialogPreview)))
                .variant(ButtonVariant::Outline)
                .on_press(SettingsMessage::OpenDialogPreview),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(iced::Length::Fill);
    let about = panel(
        column![
            text(translations.get(Text::About))
                .size(20)
                .font(typography::SANS_SEMIBOLD),
            text("Chiaroscuro").size(18).font(typography::SANS_SEMIBOLD),
            text(translations.get(Text::DesktopTelemetryInterface)).size(15),
            text(format!(
                "{} {}",
                tr(Text::Version),
                env!("CARGO_PKG_VERSION")
            ))
            .size(15),
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
    let translations = Translations::new(state.locale);
    let body = column![
        text(translations.get(Text::DialogPreviewBody)),
        text(translations.get(Text::DialogPreviewInstructions)),
        text(translations.get(Text::DialogPreviewNavigation)),
    ]
    .spacing(10);
    let footer = row![
        button(text(translations.get(Text::Cancel)))
            .variant(ButtonVariant::Outline)
            .on_press(map(SettingsMessage::CloseDialogPreview)),
        button(text(translations.get(Text::Confirm)))
            .on_press(map(SettingsMessage::ConfirmDialogPreview)),
    ]
    .spacing(8);

    dialog(base, body)
        .open(state.is_dialog_preview_open())
        .title(translations.get(Text::DialogPreview))
        .description(translations.get(Text::DialogPreviewDescription))
        .footer(footer)
        .width(520)
        .close_label(translations.get(Text::CloseDialog))
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
