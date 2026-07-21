use chiaro_actions::Screen;
use chiaro_widgets::navigation_item;
use iced::{
    Background, Border, Element, Length, Theme,
    widget::{Space, column, container},
};
use iced_fonts::lucide;

/// Width shared by the navigation rail and the title-bar brand area.
pub const WIDTH: f32 = 48.0;
const ICON_SIZE: u32 = 20;
const RAIL_VERTICAL_PADDING: u16 = 8;
const RAIL_HORIZONTAL_PADDING: u16 = 4;
const WINDOW_CORNER_RADIUS: f32 = 10.0;

#[derive(Debug, Clone, Default)]
pub struct Navigation {
    current: Screen,
}

impl Navigation {
    pub fn current(&self) -> Screen {
        self.current
    }

    pub fn navigate(&mut self, page: Screen) {
        self.current = page;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NavigationMessage {
    Navigate(Screen),
}

pub fn update(_state: &mut Navigation, message: NavigationMessage) -> Option<Screen> {
    match message {
        NavigationMessage::Navigate(screen) => Some(screen),
    }
}

pub fn view(state: &Navigation, rounded: bool) -> Element<'_, NavigationMessage> {
    let primary_destinations = column![destination(
        lucide::gauge().size(ICON_SIZE),
        Screen::Dashboard,
        state.current,
    ),]
    .width(Length::Fill);
    let application_destinations = column![destination(
        lucide::settings().size(ICON_SIZE),
        Screen::Settings,
        state.current,
    ),]
    .width(Length::Fill);
    let destinations = column![
        primary_destinations,
        Space::new().height(Length::Fill),
        application_destinations,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true);

    let sidebar = container(destinations)
        .width(Length::Fixed(WIDTH))
        .height(Length::Fill)
        .padding([RAIL_VERTICAL_PADDING, RAIL_HORIZONTAL_PADDING])
        .style(move |theme| sidebar_style(theme, rounded))
        .clip(true);

    sidebar.into()
}

fn destination(
    icon: impl Into<Element<'static, NavigationMessage>>,
    screen: Screen,
    current: Screen,
) -> Element<'static, NavigationMessage> {
    let selected = screen == current;

    navigation_item(icon, screen.title())
        .selected(selected)
        .on_press(NavigationMessage::Navigate(screen))
        .into()
}

fn sidebar_style(theme: &Theme, rounded: bool) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weaker.color)),
        text_color: Some(palette.background.weaker.text),
        border: Border {
            radius: if rounded {
                iced::border::Radius {
                    bottom_left: WINDOW_CORNER_RADIUS,
                    ..iced::border::Radius::default()
                }
            } else {
                iced::border::Radius::default()
            },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Navigation, NavigationMessage, RAIL_HORIZONTAL_PADDING, RAIL_VERTICAL_PADDING, Screen,
        WIDTH, update,
    };

    #[test]
    fn compact_rail_keeps_a_40_pixel_target_inside_48_pixels() {
        assert_eq!(WIDTH, 48.0);
        assert_eq!(RAIL_HORIZONTAL_PADDING, 4);
        assert_eq!(RAIL_VERTICAL_PADDING, 8);
        assert_eq!(WIDTH - f32::from(RAIL_HORIZONTAL_PADDING) * 2.0, 40.0);
    }

    #[test]
    fn navigation_selects_a_page() {
        let mut navigation = Navigation::default();

        navigation.navigate(Screen::Settings);

        assert_eq!(navigation.current(), Screen::Settings);
    }

    #[test]
    fn destination_message_requests_navigation() {
        let mut navigation = Navigation::default();

        assert_eq!(
            update(
                &mut navigation,
                NavigationMessage::Navigate(Screen::Settings),
            ),
            Some(Screen::Settings)
        );
    }
}
