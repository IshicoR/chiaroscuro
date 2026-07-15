use std::{fmt, time::Duration};

use iced::{Subscription, time};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::action::Action;

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Poll,
}

pub struct TrayState {
    tray_icon: Option<TrayIcon>,
    open_id: Option<MenuId>,
    quit_id: Option<MenuId>,
}

impl TrayState {
    pub fn new() -> Result<Self, String> {
        let open = MenuItem::new("Open Chiaroscuro", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::with_items(&[&open, &separator, &quit])
            .map_err(|error| format!("failed to build tray menu: {error}"))?;
        let icon = Icon::from_rgba(icon_rgba(), 32, 32)
            .map_err(|error| format!("failed to build tray icon: {error}"))?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("Chiaroscuro")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_icon(icon)
            .build()
            .map_err(|error| format!("failed to create tray icon: {error}"))?;

        Ok(Self {
            tray_icon: Some(tray_icon),
            open_id: Some(open.id().clone()),
            quit_id: Some(quit.id().clone()),
        })
    }

    pub fn is_available(&self) -> bool {
        self.tray_icon.is_some()
    }
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            tray_icon: None,
            open_id: None,
            quit_id: None,
        }
    }
}

impl fmt::Debug for TrayState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrayState")
            .field("available", &self.is_available())
            .finish()
    }
}

pub fn subscription(state: &TrayState) -> Subscription<Message> {
    if state.is_available() {
        time::every(EVENT_POLL_INTERVAL).map(|_| Message::Poll)
    } else {
        Subscription::none()
    }
}

pub fn update(state: &TrayState, message: Message) -> Option<Action> {
    match message {
        Message::Poll => poll_action(state),
    }
}

fn poll_action(state: &TrayState) -> Option<Action> {
    let mut action = None;

    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if state.quit_id.as_ref() == Some(&event.id) {
            return Some(Action::ExitApplication);
        }
        if state.open_id.as_ref() == Some(&event.id) {
            action = Some(Action::ShowWindow);
        }
    }

    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            }
        ) {
            action = Some(Action::ShowWindow);
        }
    }

    action
}

fn icon_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);

    for y in 0..32 {
        for x in 0..32 {
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            let distance = dx.hypot(dy);
            let ring = (8.0..=13.0).contains(&distance);
            let opening = x >= 18 && (y as i32 - 16).abs() <= 5;
            let color = if ring && !opening {
                [244, 244, 245, 255]
            } else {
                [24, 24, 27, 255]
            };
            rgba.extend_from_slice(&color);
        }
    }

    rgba
}
