use iced::{
    Element, Event, Length, Renderer, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    event::{self, Status},
    mouse,
    widget::{button, container, row, text, tooltip},
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
pub struct State {
    expanded: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    ToggleExpanded,
    Dismiss,
    MenuInteraction,
    Select(Page),
    Back,
    Exit,
}

pub fn update(state: &mut State, message: Message) -> Option<Action> {
    match message {
        Message::ToggleExpanded => {
            state.expanded = !state.expanded;
            None
        },
        Message::Dismiss => {
            state.expanded = false;
            None
        },
        Message::MenuInteraction => None,
        Message::Select(page) => {
            state.expanded = false;
            Some(Action::Navigate(page))
        },
        Message::Back => {
            state.expanded = false;
            Some(Action::Back)
        },
        Message::Exit => {
            state.expanded = false;
            Some(Action::CloseWindow)
        },
    }
}

pub fn subscription(state: &State) -> Subscription<Message> {
    if !state.expanded {
        return Subscription::none();
    }

    event::listen_with(|event, status, _window| match event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            if status == Status::Ignored =>
        {
            Some(Message::Dismiss)
        },
        Event::Window(window::Event::Unfocused) => Some(Message::Dismiss),
        _ => None,
    })
}

pub fn is_expanded(state: &State) -> bool {
    state.expanded
}

pub fn dismiss(state: &mut State) {
    state.expanded = false;
}

pub fn view(state: &State, current: Page, can_go_back: bool) -> Element<'_, Message> {
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
        .on_press(Message::ToggleExpanded),
        text(if state.expanded {
            "Hide menu"
        } else {
            "Show menu"
        })
        .size(12),
        tooltip::Position::Bottom,
    );

    let mut menu_row = row![toggle]
        .align_y(Vertical::Center)
        .spacing(4)
        .height(appearance::TITLE_BAR_HEIGHT);

    if state.expanded {
        let file = drop_down(menu_items!((menu_item("Exit", Message::Exit))));
        let view = drop_down(menu_items!(
            (menu_item_maybe("Back", can_go_back.then_some(Message::Back))),
            (menu_item(
                page_label("Dashboard", Page::Dashboard, current),
                Message::Select(Page::Dashboard)
            )),
            (menu_item(
                page_label("Settings", Page::Settings, current),
                Message::Select(Page::Settings)
            )),
        ));
        let help = drop_down(menu_items!(
            (menu_item(
                page_label("About", Page::About, current),
                Message::Select(Page::About)
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

    container(menu_row)
        .height(appearance::TITLE_BAR_HEIGHT)
        .style(appearance::title_bar)
        .into()
}

fn drop_down(items: Vec<Item<'_, Message, Theme, Renderer>>) -> Menu<'_, Message, Theme, Renderer> {
    Menu::new(items)
        .width(DROP_DOWN_WIDTH)
        .offset(6.0)
        .padding(6.0)
        .spacing(3.0)
}

fn menu_root(label: &str) -> Element<'_, Message> {
    button(text(label).size(13))
        .padding([6, 10])
        .style(appearance::menu_button)
        .on_press(Message::MenuInteraction)
        .into()
}

fn menu_item(label: impl Into<String>, message: Message) -> Element<'static, Message> {
    menu_item_maybe(label, Some(message))
}

fn menu_item_maybe(
    label: impl Into<String>,
    message: Option<Message>,
) -> Element<'static, Message> {
    button(text(label.into()).size(13).width(Length::Fill))
        .width(Length::Fill)
        .padding([7, 10])
        .style(appearance::menu_button)
        .on_press_maybe(message)
        .into()
}

fn page_label(label: &str, page: Page, current: Page) -> String {
    if page == current {
        format!("{label}  •")
    } else {
        label.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, State, is_expanded, update};
    use crate::navigation::Page;

    #[test]
    fn dismiss_closes_the_expanded_menu() {
        let mut state = State::default();
        update(&mut state, Message::ToggleExpanded);

        update(&mut state, Message::Dismiss);

        assert!(!is_expanded(&state));
    }

    #[test]
    fn selecting_a_page_closes_the_expanded_menu() {
        let mut state = State::default();
        update(&mut state, Message::ToggleExpanded);

        update(&mut state, Message::Select(Page::Settings));

        assert!(!is_expanded(&state));
    }
}
