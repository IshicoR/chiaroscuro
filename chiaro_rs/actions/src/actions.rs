use chiaro_i18n::{Text, tr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IbtLoadState {
    #[default]
    Idle,
    Selecting,
    Loading,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceIbtState {
    load_state: IbtLoadState,
    error: Option<String>,
}

impl ReferenceIbtState {
    pub const fn load_state(&self) -> IbtLoadState {
        self.load_state
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub const fn is_idle(&self) -> bool {
        matches!(self.load_state, IbtLoadState::Idle)
    }

    pub fn begin_selection(&mut self) {
        self.load_state = IbtLoadState::Selecting;
        self.error = None;
    }

    pub fn begin_load(&mut self) {
        self.load_state = IbtLoadState::Loading;
        self.error = None;
    }

    pub fn finish_load(&mut self) {
        self.load_state = IbtLoadState::Idle;
    }

    pub fn mark_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn clear(&mut self) {
        self.load_state = IbtLoadState::Idle;
        self.error = None;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Telemetry,
    CarSetup,
    Regulations,
    Settings,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Telemetry => tr(Text::Telemetry),
            Self::CarSetup => tr(Text::CarSetup),
            Self::Regulations => tr(Text::Regulations),
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

#[cfg(test)]
mod tests {
    use super::{IbtLoadState, ReferenceIbtState};

    #[test]
    fn reference_ibt_state_tracks_selection_loading_errors_and_clear() {
        let mut state = ReferenceIbtState::default();
        assert!(state.is_idle());

        state.begin_selection();
        assert_eq!(state.load_state(), IbtLoadState::Selecting);

        state.begin_load();
        assert_eq!(state.load_state(), IbtLoadState::Loading);

        state.finish_load();
        state.mark_error("invalid IBT".to_owned());
        assert!(state.is_idle());
        assert_eq!(state.error(), Some("invalid IBT"));

        state.clear();
        assert!(state.is_idle());
        assert_eq!(state.error(), None);
    }
}
