use std::time::{Duration, Instant};

use iced::{
    Animation, Background, Border, Color, Element, Event, Length, Renderer, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    event::{self, Status},
    mouse::{Button as MouseButton, Event as MouseEvent},
    time,
    widget::{
        button,
        button::{Status as ButtonStatus, Style as ButtonStyle},
        mouse_area, row, text, tooltip,
    },
    window::Event as WindowEvent,
};
use iced_aw::{
    menu::{DrawPath, Item, Menu, Style as MenuStyle},
    menu_bar, menu_items,
    style::{menu_bar, status::Status as MenuStatus},
};
use iced_fonts::lucide;

use crate::{action::Action, navigation::Screen};

const MENU_BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: u32 = 12;
const MENU_CORNER_RADIUS: f32 = 6.0;
const DROP_DOWN_WIDTH: f32 = 190.0;
const CONTROL_TRANSITION_DURATION: Duration = Duration::from_millis(140);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);
const TOOLTIP_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct MenuState {
    expanded: bool,
    hover: [Animation<bool>; MenuControl::COUNT],
    now: Instant,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            expanded: false,
            hover: std::array::from_fn(|_| hover_animation()),
            now: Instant::now(),
        }
    }
}

impl MenuState {
    fn animation(&self, control: MenuControl) -> &Animation<bool> {
        &self.hover[control as usize]
    }

    fn animation_mut(&mut self, control: MenuControl) -> &mut Animation<bool> {
        &mut self.hover[control as usize]
    }

    fn set_hover(&mut self, control: MenuControl, hovered: bool, now: Instant) {
        if hovered && control.is_drop_down() {
            for other in MenuControl::DROP_DOWN_CONTROLS {
                if other != control {
                    self.animation_mut(other).go_mut(false, now);
                }
            }
        }

        self.animation_mut(control).go_mut(hovered, now);
    }

    fn is_animating(&self) -> bool {
        self.hover
            .iter()
            .any(|animation| animation.is_animating(self.now))
    }

    fn has_hover(&self) -> bool {
        self.hover.iter().any(Animation::value)
    }

    fn collapse(&mut self) {
        self.expanded = false;

        for control in MenuControl::DROP_DOWN_CONTROLS {
            self.hover[control as usize] = hover_animation();
        }
    }

    fn dismiss(&mut self) {
        self.expanded = false;
        self.hover = std::array::from_fn(|_| hover_animation());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum MenuControl {
    Toggle,
    Quit,
    Back,
    Dashboard,
    Settings,
    About,
}

impl MenuControl {
    const COUNT: usize = 6;
    const DROP_DOWN_CONTROLS: [Self; 5] = [
        Self::Quit,
        Self::Back,
        Self::Dashboard,
        Self::Settings,
        Self::About,
    ];

    fn is_drop_down(self) -> bool {
        self != Self::Toggle
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuMessage {
    ToggleExpanded,
    Dismiss,
    MenuInteraction,
    Select(Screen),
    Back,
    Exit,
    Hover(MenuControl, bool),
    AnimationFrame(Instant),
}

pub fn subscription(state: &MenuState) -> Subscription<MenuMessage> {
    let dismissal = if state.expanded || state.has_hover() {
        event::listen_with(|event, status, _window| match event {
            Event::Mouse(MouseEvent::ButtonReleased(MouseButton::Left))
                if status == Status::Ignored =>
            {
                Some(MenuMessage::Dismiss)
            },
            Event::Window(WindowEvent::Unfocused) => Some(MenuMessage::Dismiss),
            _ => None,
        })
    } else {
        Subscription::none()
    };
    let animation = if state.is_animating() {
        time::every(ANIMATION_FRAME_INTERVAL).map(MenuMessage::AnimationFrame)
    } else {
        Subscription::none()
    };

    Subscription::batch([dismissal, animation])
}

pub fn update(state: &mut MenuState, message: MenuMessage) -> Option<Action> {
    match message {
        MenuMessage::ToggleExpanded => {
            if state.expanded {
                state.collapse();
            } else {
                state.expanded = true;
            }
            None
        },
        MenuMessage::Dismiss => {
            state.dismiss();
            None
        },
        MenuMessage::MenuInteraction => None,
        MenuMessage::Select(page) => {
            state.dismiss();
            Some(Action::Navigate(page))
        },
        MenuMessage::Back => {
            state.dismiss();
            Some(Action::Back)
        },
        MenuMessage::Exit => {
            state.dismiss();
            Some(Action::ExitApplication)
        },
        MenuMessage::Hover(control, hovered) => {
            let now = Instant::now();
            state.now = now;
            state.set_hover(control, hovered, now);

            None
        },
        MenuMessage::AnimationFrame(now) => {
            state.now = now;

            None
        },
    }
}

pub fn view(state: &MenuState, current: Screen, can_go_back: bool) -> Element<'_, MenuMessage> {
    let expanded = state.expanded;
    let toggle_hover = state
        .animation(MenuControl::Toggle)
        .interpolate(0.0, 1.0, state.now);
    let toggle = mouse_area(
        button(
            lucide::menu()
                .size(ICON_SIZE)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .width(MENU_BUTTON_SIZE)
        .height(MENU_BUTTON_SIZE)
        .padding(0)
        .style(move |theme, status| menu_toggle(theme, status, expanded, toggle_hover))
        .on_press(MenuMessage::ToggleExpanded),
    )
    .on_enter(MenuMessage::Hover(MenuControl::Toggle, true))
    .on_exit(MenuMessage::Hover(MenuControl::Toggle, false));
    let toggle = tooltip(
        toggle,
        text(if state.expanded {
            "Hide menu"
        } else {
            "Show menu"
        })
        .size(12),
        tooltip::Position::Bottom,
    )
    .delay(TOOLTIP_DELAY);

    let mut menu_row = row![toggle].align_y(Vertical::Center).spacing(4);

    if state.expanded {
        let file = drop_down(menu_items!(
            (menu_item(
                "Quit",
                MenuMessage::Exit,
                state.animation(MenuControl::Quit),
                state.now,
                MenuControl::Quit,
                true,
            ))
        ));
        let view = drop_down(menu_items!(
            (menu_item_maybe(
                "Back",
                can_go_back.then_some(MenuMessage::Back),
                state.animation(MenuControl::Back),
                state.now,
                MenuControl::Back,
                false,
            )),
            (menu_item(
                page_label(Screen::Dashboard, current),
                MenuMessage::Select(Screen::Dashboard),
                state.animation(MenuControl::Dashboard),
                state.now,
                MenuControl::Dashboard,
                false,
            )),
            (menu_item(
                page_label(Screen::Settings, current),
                MenuMessage::Select(Screen::Settings),
                state.animation(MenuControl::Settings),
                state.now,
                MenuControl::Settings,
                false,
            )),
        ));
        let help = drop_down(menu_items!(
            (menu_item(
                page_label(Screen::About, current),
                MenuMessage::Select(Screen::About),
                state.animation(MenuControl::About),
                state.now,
                MenuControl::About,
                false,
            ))
        ));

        let menus = menu_bar!(
            (menu_root("File"), file),
            (menu_root("View"), view),
            (menu_root("Help"), help),
        )
        .spacing(6.0)
        .padding([3, 2])
        .draw_path(DrawPath::Backdrop)
        .style(bar_style);

        menu_row = menu_row.push(menus);
    }

    menu_row.into()
}

pub fn is_expanded(state: &MenuState) -> bool {
    state.expanded
}

fn menu_root(label: &'static str) -> Element<'static, MenuMessage> {
    button(text(label).size(13))
        .padding([6, 10])
        .style(menu_root_style)
        .on_press(MenuMessage::MenuInteraction)
        .into()
}

fn menu_root_style(theme: &Theme, status: ButtonStatus) -> ButtonStyle {
    let hover_progress = f32::from(status == ButtonStatus::Hovered);

    button_style(theme, status, false, hover_progress)
}

fn bar_style(theme: &Theme, status: MenuStatus) -> MenuStyle {
    let palette = theme.extended_palette();

    MenuStyle {
        bar_background: Background::Color(Color::TRANSPARENT),
        bar_border: Border::default(),
        menu_background: Background::Color(palette.background.base.color),
        menu_border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: MENU_CORNER_RADIUS.into(),
        },
        path: Background::Color(Color::TRANSPARENT),
        path_border: Border::default(),
        ..menu_bar::primary(theme, status)
    }
}

fn menu_toggle(
    theme: &Theme,
    status: ButtonStatus,
    expanded: bool,
    hover_progress: f32,
) -> ButtonStyle {
    button_style(
        theme,
        status,
        false,
        if expanded { 1.0 } else { hover_progress },
    )
}

fn button_style(
    theme: &Theme,
    status: ButtonStatus,
    destructive: bool,
    hover_progress: f32,
) -> ButtonStyle {
    let palette = theme.extended_palette();
    let base = palette.background.base;
    let target = if destructive {
        palette.danger.base
    } else {
        palette.background.strong
    };
    let progress = match status {
        ButtonStatus::Pressed => 1.0,
        ButtonStatus::Disabled => 0.0,
        _ => hover_progress.clamp(0.0, 1.0),
    };

    let mut style = ButtonStyle {
        background: Some(Background::Color(target.color.scale_alpha(progress))),
        text_color: mix_color(base.text, target.text, progress),
        border: Border {
            radius: MENU_CORNER_RADIUS.into(),
            ..Border::default()
        },
        ..ButtonStyle::default()
    };

    if status == ButtonStatus::Disabled {
        style.text_color = style.text_color.scale_alpha(0.45);
    }

    style
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

fn menu_item(
    label: impl Into<String>,
    message: MenuMessage,
    hover: &Animation<bool>,
    now: Instant,
    control_kind: MenuControl,
    destructive: bool,
) -> Element<'static, MenuMessage> {
    menu_item_maybe(label, Some(message), hover, now, control_kind, destructive)
}

fn menu_item_maybe(
    label: impl Into<String>,
    message: Option<MenuMessage>,
    hover: &Animation<bool>,
    now: Instant,
    control_kind: MenuControl,
    destructive: bool,
) -> Element<'static, MenuMessage> {
    let enabled = message.is_some();
    let hover_progress = if enabled {
        hover.interpolate(0.0, 1.0, now)
    } else {
        0.0
    };
    let button = button(text(label.into()).size(13).width(Length::Fill))
        .width(Length::Fill)
        .padding([7, 10])
        .style(move |theme, status| button_style(theme, status, destructive, hover_progress))
        .on_press_maybe(message);

    if enabled {
        mouse_area(button)
            .on_enter(MenuMessage::Hover(control_kind, true))
            .on_exit(MenuMessage::Hover(control_kind, false))
            .into()
    } else {
        button.into()
    }
}

fn page_label(page: Screen, current: Screen) -> String {
    let label = page.title();
    if page == current {
        format!("{label}  •")
    } else {
        label.to_owned()
    }
}

fn mix_color(start: Color, end: Color, amount: f32) -> Color {
    Color {
        r: start.r + (end.r - start.r) * amount,
        g: start.g + (end.g - start.g) * amount,
        b: start.b + (end.b - start.b) * amount,
        a: start.a + (end.a - start.a) * amount,
    }
}

fn hover_animation() -> Animation<bool> {
    Animation::new(false).duration(CONTROL_TRANSITION_DURATION)
}

#[cfg(test)]
mod tests {
    use iced::{Background, Theme, widget::button::Status as ButtonStatus};

    use super::{MenuControl, MenuMessage, MenuState, is_expanded, menu_root_style, update};
    use crate::{action::Action, navigation::Screen};

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

        update(&mut state, MenuMessage::Select(Screen::Settings));

        assert!(!is_expanded(&state));
    }

    #[test]
    fn exit_requests_application_shutdown() {
        let mut state = MenuState::default();

        let action = update(&mut state, MenuMessage::Exit);

        assert_eq!(action, Some(Action::ExitApplication));
    }

    #[test]
    fn hover_messages_change_the_matching_animation_target() {
        let mut state = MenuState::default();

        update(&mut state, MenuMessage::Hover(MenuControl::Settings, true));

        assert!(state.animation(MenuControl::Settings).value());
        assert!(!state.animation(MenuControl::Dashboard).value());
        assert!(!state.animation(MenuControl::Toggle).value());
    }

    #[test]
    fn only_one_drop_down_control_can_be_hovered() {
        let mut state = MenuState::default();
        update(&mut state, MenuMessage::Hover(MenuControl::Dashboard, true));

        update(&mut state, MenuMessage::Hover(MenuControl::Settings, true));

        assert!(!state.animation(MenuControl::Dashboard).value());
        assert!(state.animation(MenuControl::Settings).value());
    }

    #[test]
    fn dismissing_clears_all_hover_targets() {
        let mut state = MenuState::default();
        update(&mut state, MenuMessage::ToggleExpanded);
        update(&mut state, MenuMessage::Hover(MenuControl::Settings, true));
        update(&mut state, MenuMessage::Hover(MenuControl::Toggle, true));

        update(&mut state, MenuMessage::Dismiss);

        assert!(!state.animation(MenuControl::Settings).value());
        assert!(!state.animation(MenuControl::Toggle).value());
    }

    #[test]
    fn menu_root_background_is_only_visible_while_hovered() {
        let active = menu_root_style(&Theme::Light, ButtonStatus::Active);
        let hovered = menu_root_style(&Theme::Light, ButtonStatus::Hovered);

        assert_eq!(background_alpha(active.background), 0.0);
        assert!(background_alpha(hovered.background) > 0.0);
    }

    fn background_alpha(background: Option<Background>) -> f32 {
        match background {
            Some(Background::Color(color)) => color.a,
            _ => panic!("expected a color background"),
        }
    }
}
