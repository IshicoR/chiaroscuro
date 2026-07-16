#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Dashboard,
    Settings,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
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
