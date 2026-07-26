#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Telemetry,
    CarSetup,
    Settings,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Telemetry => tr(Text::Telemetry),
            Self::CarSetup => tr(Text::CarSetup),
            Self::Settings => tr(Text::Settings),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Navigate(Screen),
    OpenIbt,
    OpenReferenceIbt,
    ClearReferenceIbt,
    SetConnected(bool),
    ShowWindow,
    ExitApplication,
}
use chiaro_i18n::{Text, tr};
