use iced::{
    Element,
    Length::Fill,
    Subscription, Task, Theme,
    widget::{column, container},
};

use crate::{
    action::Action,
    appearance::{self, AppearanceState},
    configuration::DesktopConfig,
    menu::{self, MenuMessage, MenuState},
    navigation::{Navigation, Screen},
    screen::{
        about::{self, AboutMessage, AboutState},
        dashboard::{self, DashboardMessage, DashboardState},
        settings::{self, SettingsMessage, SettingsState},
    },
    session::Session,
    telemetry,
    window::{self, WindowMessage, WindowState},
};

#[derive(Debug, Default)]
pub struct Chiaroscuro {
    navigation: Navigation,
    session: Session,
    appearance: AppearanceState,
    menu: MenuState,
    window: WindowState,
    dashboard: DashboardState,
    settings: SettingsState,
    about: AboutState,
    configuration_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Menu(MenuMessage),
    Window(WindowMessage),
    Dashboard(DashboardMessage),
    Settings(SettingsMessage),
    About(AboutMessage),
    Telemetry(telemetry::Event),
}

impl Chiaroscuro {
    pub fn new() -> Self {
        let mut app = Self::default();
        match DesktopConfig::load() {
            Ok(config) => {
                app.session.set_server_addr(config.server_addr);
                app.appearance.set_mode(if config.dark_theme {
                    appearance::Mode::Dark
                } else {
                    appearance::Mode::Light
                });
                app.settings.set_show_diagnostics(config.show_diagnostics);
            },
            Err(error) => {
                app.configuration_error = Some(format!("failed to load desktop settings: {error}"));
            },
        }

        app
    }

    pub fn title(&self) -> String {
        format!("Chiaroscuro - {}", self.navigation.current().title())
    }

    pub fn theme(&self) -> Theme {
        self.appearance.theme()
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        let wants_connection = self.session.wants_connection();
        let telemetry = if wants_connection {
            telemetry::subscription(self.session.server_addr().to_owned())
                .map(AppMessage::Telemetry)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            window::subscription(&self.window).map(AppMessage::Window),
            menu::subscription(&self.menu).map(AppMessage::Menu),
            dashboard::subscription(
                wants_connection && self.navigation.current() == Screen::Dashboard,
            )
            .map(AppMessage::Dashboard),
            telemetry,
        ])
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Menu(message) => {
                let action = menu::update(&mut self.menu, message);
                self.handle_action(action)
            },
            AppMessage::Window(message) => {
                window::update(&mut self.window, message).map(AppMessage::Window)
            },
            AppMessage::Dashboard(message) => {
                let action = dashboard::update(&mut self.dashboard, &self.session, message);
                self.handle_action(action)
            },
            AppMessage::Settings(message) => {
                let action = settings::update(&mut self.settings, message);
                let action = self.handle_action(action);
                self.save_configuration();

                action
            },
            AppMessage::About(message) => {
                let action = about::update(&mut self.about, message);
                self.handle_action(action)
            },
            AppMessage::Telemetry(event) => {
                match event {
                    telemetry::Event::Waiting => self.session.mark_waiting(),
                    telemetry::Event::Connected => self.session.mark_connected(),
                    telemetry::Event::Sample(sample) => {
                        self.session.record_sample(sample);
                    },
                    telemetry::Event::Error(error) => self.session.mark_error(error),
                }
                Task::none()
            },
        }
    }

    pub fn view(&self) -> Element<'_, AppMessage> {
        let screen = match self.navigation.current() {
            Screen::Dashboard => {
                dashboard::view(&self.dashboard, &self.session).map(AppMessage::Dashboard)
            },
            Screen::Settings => settings::view(
                &self.settings,
                self.appearance.mode(),
                &self.session,
                self.configuration_error.as_deref(),
            )
            .map(AppMessage::Settings),
            Screen::About => about::view(&self.about).map(AppMessage::About),
        };

        let application_menu = menu::view(
            &self.menu,
            self.navigation.current(),
            self.navigation.previous().is_some(),
        )
        .map(AppMessage::Menu);

        let content = container(screen)
            .padding(appearance::CONTENT_PADDING)
            .width(Fill)
            .height(Fill)
            .style(appearance::content);

        column![
            window::view(
                &self.window,
                application_menu,
                !menu::is_expanded(&self.menu),
                AppMessage::Window,
            ),
            content,
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn handle_action(&mut self, action: Option<Action>) -> Task<AppMessage> {
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
                dashboard::sync_telemetry(&mut self.dashboard, &self.session);
                Task::none()
            },
            Action::SetTheme(mode) => {
                self.appearance.set_mode(mode);
                Task::none()
            },
            Action::CloseWindow => window::close().map(AppMessage::Window),
        }
    }

    fn save_configuration(&mut self) {
        let config = DesktopConfig {
            server_addr: self.session.server_addr().to_owned(),
            dark_theme: self.appearance.mode() == appearance::Mode::Dark,
            show_diagnostics: self.settings.show_diagnostics(),
        };

        self.configuration_error = config
            .save()
            .err()
            .map(|error| format!("failed to save desktop settings: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::{AppMessage, Chiaroscuro};
    use crate::{
        menu::{self, MenuMessage},
        telemetry,
    };

    #[test]
    fn telemetry_does_not_dismiss_the_open_menu() {
        let mut app = Chiaroscuro::default();
        menu::update(&mut app.menu, MenuMessage::ToggleExpanded);

        drop(app.update(AppMessage::Telemetry(telemetry::Event::Waiting)));

        assert!(menu::is_expanded(&app.menu));
    }
}
