use chiaro_actions::Screen;
use iced::{
    Background, Border, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Space, Text, button, column, container, row, rule, text},
};
use iced_fonts::lucide;

const NAVIGATION_WIDTH: f32 = 76.0;
const DESTINATION_HEIGHT: f32 = 62.0;
const ICON_SIZE: u32 = 20;
const BUTTON_CORNER_RADIUS: f32 = 6.0;
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
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([12, 6])
        .style(move |theme| sidebar_style(theme, rounded))
        .clip(true);

    row![sidebar, rule::vertical(1)]
        .width(Length::Fixed(NAVIGATION_WIDTH))
        .height(Length::Fill)
        .clip(true)
        .into()
}

fn destination(
    icon: Text<'static>,
    screen: Screen,
    current: Screen,
) -> Element<'static, NavigationMessage> {
    let selected = screen == current;
    let label = container(
        column![
            icon.width(Length::Fill)
                .height(Length::Fixed(22.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            text(screen.title())
                .size(12)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        ]
        .spacing(4)
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .center(Length::Fill);

    button(label)
        .width(Length::Fill)
        .height(Length::Fixed(DESTINATION_HEIGHT))
        .padding(0)
        .on_press(NavigationMessage::Navigate(screen))
        .style(move |theme, status| destination_style(theme, status, selected))
        .clip(true)
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

fn destination_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let colors = if selected {
        Some(palette.primary.weak)
    } else if status == button::Status::Hovered || status == button::Status::Pressed {
        Some(palette.background.neutral)
    } else {
        None
    };

    button::Style {
        background: colors.map(|colors| Background::Color(colors.color)),
        text_color: colors.map_or(palette.background.weaker.text, |colors| colors.text),
        border: Border {
            radius: BUTTON_CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Navigation, NavigationMessage, Screen, update};

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
