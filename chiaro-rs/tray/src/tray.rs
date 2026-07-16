use std::fmt::{self, Debug};

use chiaro_actions::Action;
use iced::{
    Subscription,
    futures::{Stream, channel::mpsc},
};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo.png");
const TRAY_ICON_SIZE: u32 = 32;

#[derive(Debug, Clone)]
pub enum Message {
    Menu(MenuEvent),
    ShowWindow,
}

#[derive(Default)]
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
        let icon = Icon::from_rgba(icon_rgba()?, TRAY_ICON_SIZE, TRAY_ICON_SIZE)
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

impl Debug for TrayState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrayState")
            .field("available", &self.is_available())
            .finish()
    }
}

pub fn subscription(state: &TrayState) -> Subscription<Message> {
    if state.is_available() {
        Subscription::run(event_stream)
    } else {
        Subscription::none()
    }
}

pub fn update(state: &TrayState, msg: Message) -> Option<Action> {
    match msg {
        Message::Menu(event) => {
            if state.quit_id.as_ref() == Some(&event.id) {
                Some(Action::ExitApplication)
            } else if state.open_id.as_ref() == Some(&event.id) {
                Some(Action::ShowWindow)
            } else {
                None
            }
        },
        Message::ShowWindow => Some(Action::ShowWindow),
    }
}

fn event_stream() -> impl Stream<Item = Message> + 'static {
    let (sender, receiver) = mpsc::unbounded();
    let menu_sender = sender.clone();

    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_sender.unbounded_send(Message::Menu(event));
    }));
    TrayIconEvent::set_event_handler(Some(move |event| {
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
            let _ = sender.unbounded_send(Message::ShowWindow);
        }
    }));

    receiver
}

fn icon_rgba() -> Result<Vec<u8>, String> {
    let logo = image::load_from_memory(LOGO_BYTES)
        .map_err(|error| format!("failed to decode tray icon: {error}"))?;
    let logo = logo.resize_exact(
        TRAY_ICON_SIZE,
        TRAY_ICON_SIZE,
        image::imageops::FilterType::Lanczos3,
    );

    Ok(logo.into_rgba8().into_raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_menu_ids() -> TrayState {
        TrayState {
            tray_icon: None,
            open_id: Some(MenuId::new("open")),
            quit_id: Some(MenuId::new("quit")),
        }
    }

    #[test]
    fn embedded_logo_decodes_to_a_tray_icon() {
        let rgba = icon_rgba().expect("the embedded logo should decode");

        assert_eq!(rgba.len(), (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize);
    }

    #[test]
    fn menu_events_are_routed_without_polling() {
        let state = state_with_menu_ids();

        assert_eq!(
            update(
                &state,
                Message::Menu(MenuEvent {
                    id: MenuId::new("open"),
                }),
            ),
            Some(Action::ShowWindow)
        );
        assert_eq!(
            update(
                &state,
                Message::Menu(MenuEvent {
                    id: MenuId::new("quit"),
                }),
            ),
            Some(Action::ExitApplication)
        );
        assert_eq!(
            update(
                &state,
                Message::Menu(MenuEvent {
                    id: MenuId::new("unknown"),
                }),
            ),
            None
        );
        assert_eq!(
            update(&state, Message::ShowWindow),
            Some(Action::ShowWindow)
        );
    }
}
