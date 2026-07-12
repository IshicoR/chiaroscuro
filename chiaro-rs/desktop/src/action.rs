use crate::{appearance, navigation::Page};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Navigate(Page),
    Back,
    SetConnected(bool),
    SetTheme(appearance::Mode),
    CloseWindow,
}
