use iced::{
    Element, Event, Length, Renderer, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    event::{self, Status},
    mouse,
    widget::{button, row, text, tooltip},
    window,
};
use iced_aw::{
    menu::{Item, Menu},
    menu_bar, menu_items,
};
use iced_fonts::lucide;

use crate::{action::Action, appearance, navigation::Page};

const DROP_DOWN_WIDTH: f32 = 190.0;

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    expanded: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuMessage {
    ToggleExpanded,
    Dismiss,
    MenuInteraction,
    Select(Page),
    Back,
    Exit,
}

pub fn update(state: &mut MenuState, message: MenuMessage) -> Option<Action> {
    match message {
        MenuMessage::ToggleExpanded => {
            state.expanded = !state.expanded;
            None
        },
        MenuMessage::Dismiss => {
            state.expanded = false;
            None
        },
        MenuMessage::MenuInteraction => None,
        MenuMessage::Select(page) => {
            state.expanded = false;
            Some(Action::Navigate(page))
        },
        MenuMessage::Back => {
            state.expanded = false;
            Some(Action::Back)
        },
        MenuMessage::Exit => {
            state.expanded = false;
            Some(Action::CloseWindow)
        },
    }
}

pub fn subscription(state: &MenuState) -> Subscription<MenuMessage> {
    if !state.expanded {
        return Subscription::none();
    }

    event::listen_with(|event, status, _window| match event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            if status == Status::Ignored =>
        {
            Some(MenuMessage::Dismiss)
        },
        Event::Window(window::Event::Unfocused) => Some(MenuMessage::Dismiss),
        _ => None,
    })
}

pub fn is_expanded(state: &MenuState) -> bool {
    state.expanded
}

pub fn view(state: &MenuState, current: Page, can_go_back: bool) -> Element<'_, MenuMessage> {
    let expanded = state.expanded;
    let toggle = tooltip(
        button(
            lucide::menu()
                .size(16)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .width(appearance::MENU_BUTTON_SIZE)
        .height(appearance::MENU_BUTTON_SIZE)
        .padding(0)
        .style(move |theme, status| appearance::menu_toggle(theme, status, expanded))
        .on_press(MenuMessage::ToggleExpanded),
        text(if state.expanded {
            "Hide menu"
        } else {
            "Show menu"
        })
        .size(12),
        tooltip::Position::Bottom,
    );

    let mut menu_row = row![toggle].align_y(Vertical::Center).spacing(4);

    if state.expanded {
        let file = drop_down(menu_items!((menu_item("Exit", MenuMessage::Exit))));
        let view = drop_down(menu_items!(
            (menu_item_maybe("Back", can_go_back.then_some(MenuMessage::Back))),
            (menu_item(
                page_label(Page::Dashboard, current),
                MenuMessage::Select(Page::Dashboard)
            )),
            (menu_item(
                page_label(Page::Settings, current),
                MenuMessage::Select(Page::Settings)
            )),
        ));
        let help = drop_down(menu_items!(
            (menu_item(
                page_label(Page::About, current),
                MenuMessage::Select(Page::About)
            ))
        ));

        let menus = menu_bar!(
            (menu_root("File"), file),
            (menu_root("View"), view),
            (menu_root("Help"), help),
        )
        .spacing(6.0)
        .padding([3, 2])
        .style(appearance::menu_bar);

        menu_row = menu_row.push(menus);
    }

    menu_row.into()
}

fn drop_down(
    items: Vec<Item<'_, MenuMessage, Theme, Renderer>>,
) -> Menu<'_, MenuMessage, Theme, Renderer> {
    Menu::new(items)
        .width(DROP_DOWN_WIDTH)
        .offset(6.0)
        .padding(6.0)
        .spacing(3.0)
}

fn menu_root(label: &str) -> Element<'_, MenuMessage> {
    button(text(label).size(13))
        .padding([6, 10])
        .style(appearance::menu_button)
        .on_press(MenuMessage::MenuInteraction)
        .into()
}

fn menu_item(label: impl Into<String>, message: MenuMessage) -> Element<'static, MenuMessage> {
    menu_item_maybe(label, Some(message))
}

fn menu_item_maybe(
    label: impl Into<String>,
    message: Option<MenuMessage>,
) -> Element<'static, MenuMessage> {
    button(text(label.into()).size(13).width(Length::Fill))
        .width(Length::Fill)
        .padding([7, 10])
        .style(appearance::menu_button)
        .on_press_maybe(message)
        .into()
}

fn page_label(page: Page, current: Page) -> String {
    let label = page.title();
    if page == current {
        format!("{label}  •")
    } else {
        label.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuMessage, MenuState, is_expanded, update};
    use crate::navigation::Page;

    #[test]
    fn dismiss_closes_the_expanded_menu() {
        let mut state = MenuState::default();
        update(&mut state, MenuMessage::ToggleExpanded);

        update(&mut state, MenuMessage::Dismiss);

        assert!(!is_expanded(&state));
    }

    #[test]
    fn selecting_a_page_closes_the_expanded_menu() {
        let mut state = MenuState::default();
        update(&mut state, MenuMessage::ToggleExpanded);

        update(&mut state, MenuMessage::Select(Page::Settings));

        assert!(!is_expanded(&state));
    }
}
