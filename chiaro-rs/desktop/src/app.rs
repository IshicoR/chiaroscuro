use iced::{
    Element,
    Length::Fill,
    Subscription, Task, Theme,
    widget::{column, container},
};

use crate::{
    action::Action,
    appearance, menu,
    navigation::{Navigation, Page},
    screen::{about, dashboard, settings},
    session::Session,
    telemetry::{self, SourceConfig},
    window,
};

#[derive(Debug, Default)]
pub struct App {
    appearance: appearance::State,
    navigation: Navigation,
    session: Session,
    menu: menu::State,
    window: window::State,
    dashboard: dashboard::State,
    settings: settings::State,
    about: about::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(menu::Message),
    Window(window::Message),
    Dashboard(dashboard::Message),
    Settings(settings::Message),
    About(about::Message),
    Telemetry(telemetry::Event),
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        match SourceConfig::load() {
            Ok(config) => app.session.set_server_addr(config.server_addr),
            Err(error) => eprintln!("failed to load desktop telemetry settings: {error}"),
        }
        app
    }

    pub fn title(&self) -> String {
        let page = match self.navigation.current() {
            Page::Dashboard => "Dashboard",
            Page::Settings => "Settings",
            Page::About => "About",
        };

        format!("Chiaroscuro - {page}")
    }

    pub fn theme(&self) -> Theme {
        self.appearance.theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let telemetry = if self.session.wants_connection() {
            telemetry::subscription(self.session.server_addr().to_owned()).map(Message::Telemetry)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            window::subscription().map(Message::Window),
            menu::subscription(&self.menu).map(Message::Menu),
            telemetry,
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(&message, Message::Menu(_)) {
            menu::dismiss(&mut self.menu);
        }

        match message {
            Message::Menu(message) => {
                let action = menu::update(&mut self.menu, message);
                self.handle_action(action)
            },
            Message::Window(message) => {
                window::update(&mut self.window, message).map(Message::Window)
            },
            Message::Dashboard(message) => {
                let (task, action) = dashboard::update(&mut self.dashboard, &self.session, message);

                Task::batch([task.map(Message::Dashboard), self.handle_action(action)])
            },
            Message::Settings(message) => {
                let (task, action) = settings::update(&mut self.settings, message);

                Task::batch([task.map(Message::Settings), self.handle_action(action)])
            },
            Message::About(message) => {
                let (task, action) = about::update(&mut self.about, message);

                Task::batch([task.map(Message::About), self.handle_action(action)])
            },
            Message::Telemetry(event) => {
                match event {
                    telemetry::Event::Waiting => self.session.mark_waiting(),
                    telemetry::Event::Connected => self.session.mark_connected(),
                    telemetry::Event::Sample(sample) => {
                        self.session.record_sample(sample);
                        dashboard::sync_telemetry(&mut self.dashboard, &self.session);
                    },
                    telemetry::Event::Error(error) => self.session.mark_error(error),
                }
                Task::none()
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let page = match self.navigation.current() {
            Page::Dashboard => {
                dashboard::view(&self.dashboard, &self.session).map(Message::Dashboard)
            },
            Page::Settings => {
                settings::view(&self.settings, self.appearance.mode()).map(Message::Settings)
            },
            Page::About => about::view(&self.about).map(Message::About),
        };

        let content = container(page)
            .padding(appearance::CONTENT_PADDING)
            .width(Fill)
            .height(Fill)
            .style(appearance::content);

        let application_menu = menu::view(
            &self.menu,
            self.navigation.current(),
            self.navigation.previous().is_some(),
        )
        .map(Message::Menu);

        column![
            window::title_bar(
                application_menu,
                !menu::is_expanded(&self.menu),
                Message::Window,
            ),
            content,
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn handle_action(&mut self, action: Option<Action>) -> Task<Message> {
        let Some(action) = action else {
            return Task::none();
        };

        match action {
            Action::Navigate(page) => {
                self.navigation.navigate(page);
                Task::none()
            },
            Action::Back => {
                self.navigation.back();
                Task::none()
            },
            Action::SetConnected(connected) => {
                self.session.set_connection_requested(connected);
                Task::none()
            },
            Action::SetTheme(mode) => {
                self.appearance.set_mode(mode);
                Task::none()
            },
            Action::CloseWindow => window::close().map(Message::Window),
        }
    }
}
