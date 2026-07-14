use crate::{appearance, navigation::Screen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Navigate(Screen),
    Back,
    SetConnected(bool),
    SetTheme(appearance::Mode),
    CloseWindow,
}
