use crate::{navigation::Screen, theme::ThemeMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Navigate(Screen),
    Back,
    SetConnected(bool),
    SetTheme(ThemeMode),
    ShowWindow,
    ExitApplication,
}
