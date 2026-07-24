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
            Self::Telemetry => "Telemetry",
            Self::CarSetup => "Car setup",
            Self::Settings => "Settings",
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
