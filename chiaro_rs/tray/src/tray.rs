use std::fmt::{self, Debug};

use chiaro_actions::Action;
#[cfg(target_os = "windows")]
use chiaro_i18n::{Text, tr};
use iced::Subscription;
#[cfg(target_os = "windows")]
use iced::futures::{Stream, channel::mpsc};
#[cfg(target_os = "windows")]
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[cfg(target_os = "windows")]
const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo.png");
#[cfg(target_os = "windows")]
const TRAY_ICON_SIZE: u32 = 32;
#[cfg(target_os = "windows")]
const TRAY_ICON_CORNER_RADIUS: f32 = TRAY_ICON_SIZE as f32 * 0.2;
#[cfg(target_os = "windows")]
const EDGE_FEATHER: f32 = 0.5;
#[cfg(target_os = "windows")]
const OPEN_MENU_ID: &str = "open";
#[cfg(target_os = "windows")]
const QUIT_MENU_ID: &str = "quit";

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ShowWindow,
    ExitApplication,
}

#[derive(Default)]
pub struct TrayState {
    #[cfg(target_os = "windows")]
    tray_icon: Option<TrayIcon>,
}

impl TrayState {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        {
            let open = MenuItem::with_id(OPEN_MENU_ID, tr(Text::OpenChiaroscuro), true, None);
            let separator = PredefinedMenuItem::separator();
            let quit = MenuItem::with_id(QUIT_MENU_ID, tr(Text::Quit), true, None);
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
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self::default())
        }
    }

    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.tray_icon.is_some()
        }

        #[cfg(not(target_os = "windows"))]
        {
            false
        }
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

pub fn subscription(_state: &TrayState) -> Subscription<TrayMessage> {
    #[cfg(target_os = "windows")]
    {
        if _state.is_available() {
            return Subscription::run(event_stream);
        }
    }

    Subscription::none()
}

pub fn update(_state: &TrayState, msg: TrayMessage) -> Option<Action> {
    match msg {
        TrayMessage::ShowWindow => Some(Action::ShowWindow),
        TrayMessage::ExitApplication => Some(Action::ExitApplication),
    }
}

#[cfg(target_os = "windows")]
fn event_stream() -> impl Stream<Item = TrayMessage> + 'static {
    let (sender, receiver) = mpsc::unbounded();
    let menu_sender = sender.clone();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let message = match event.id.0.as_str() {
            OPEN_MENU_ID => Some(TrayMessage::ShowWindow),
            QUIT_MENU_ID => Some(TrayMessage::ExitApplication),
            _ => None,
        };
        if let Some(message) = message {
            let _ = menu_sender.unbounded_send(message);
        }
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
            let _ = sender.unbounded_send(TrayMessage::ShowWindow);
        }
    }));

    receiver
}

#[cfg(target_os = "windows")]
fn icon_rgba() -> Result<Vec<u8>, String> {
    let logo = image::load_from_memory(LOGO_BYTES)
        .map_err(|error| format!("failed to decode tray icon: {error}"))?;
    let logo = logo.resize_exact(
        TRAY_ICON_SIZE,
        TRAY_ICON_SIZE,
        image::imageops::FilterType::Lanczos3,
    );

    let mut logo = logo.into_rgba8();
    apply_rounded_alpha(
        logo.as_mut(),
        TRAY_ICON_SIZE,
        TRAY_ICON_SIZE,
        TRAY_ICON_CORNER_RADIUS,
    );

    Ok(logo.into_raw())
}

#[cfg(target_os = "windows")]
fn apply_rounded_alpha(rgba: &mut [u8], width: u32, height: u32, radius: f32) {
    let right_center = width as f32 - radius;
    let bottom_center = height as f32 - radius;

    for y in 0..height {
        let pixel_y = y as f32 + 0.5;
        let distance_y = if pixel_y < radius {
            radius - pixel_y
        } else if pixel_y > bottom_center {
            pixel_y - bottom_center
        } else {
            0.0
        };

        for x in 0..width {
            let pixel_x = x as f32 + 0.5;
            let distance_x = if pixel_x < radius {
                radius - pixel_x
            } else if pixel_x > right_center {
                pixel_x - right_center
            } else {
                0.0
            };
            let distance = distance_x.hypot(distance_y);
            let coverage = (radius + EDGE_FEATHER - distance).clamp(0.0, 1.0);
            let alpha_index = ((y * width + x) * 4 + 3) as usize;

            rgba[alpha_index] = (f32::from(rgba[alpha_index]) * coverage).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn embedded_logo_decodes_to_a_tray_icon() {
        let rgba = icon_rgba().expect("the embedded logo should decode");

        assert_eq!(rgba.len(), (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize);
        assert_eq!(alpha_at(&rgba, 0, 0), 0);
        assert_eq!(alpha_at(&rgba, TRAY_ICON_SIZE - 1, 0), 0);
        assert_eq!(alpha_at(&rgba, 0, TRAY_ICON_SIZE - 1), 0);
        assert_eq!(alpha_at(&rgba, TRAY_ICON_SIZE - 1, TRAY_ICON_SIZE - 1), 0);
        assert_eq!(alpha_at(&rgba, TRAY_ICON_SIZE / 2, TRAY_ICON_SIZE / 2), 255);
        assert!(alpha_at(&rgba, 1, 2) > 0);
        assert!(alpha_at(&rgba, 1, 2) < 255);
    }

    #[cfg(target_os = "windows")]
    fn alpha_at(rgba: &[u8], x: u32, y: u32) -> u8 {
        rgba[((y * TRAY_ICON_SIZE + x) * 4 + 3) as usize]
    }

    #[test]
    fn tray_commands_map_to_application_actions() {
        let state = TrayState::default();
        assert_eq!(
            update(&state, TrayMessage::ShowWindow),
            Some(Action::ShowWindow)
        );
        assert_eq!(
            update(&state, TrayMessage::ExitApplication),
            Some(Action::ExitApplication)
        );
    }
}
