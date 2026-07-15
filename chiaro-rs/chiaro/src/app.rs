use iced::{
    Element,
    Length::Fill,
    Subscription, Task, Theme,
    widget::{column, container},
};

use crate::{
    action::Action,
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
    theme::{self, ThemeMode},
    tray,
    window::{self, WindowMessage, WindowState},
};

const CONTENT_PADDING: f32 = 24.0;

#[derive(Debug, Default)]
pub struct Chiaroscuro {
    navigation: Navigation,
    session: Session,
    theme_mode: ThemeMode,
    menu: MenuState,
    window: WindowState,
    dashboard: DashboardState,
    settings: SettingsState,
    about: AboutState,
    tray: tray::TrayState,
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
    Tray(tray::Message),
}

impl Chiaroscuro {
    pub fn new() -> Self {
        let mut app = Self::default();
        match tray::TrayState::new() {
            Ok(tray) => app.tray = tray,
            Err(error) => eprintln!("failed to initialize system tray: {error}"),
        }

        match DesktopConfig::load() {
            Ok(config) => {
                app.session.set_server_addr(config.server_addr);
                app.theme_mode = if config.dark_theme {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                };
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
        self.theme_mode.theme()
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
            tray::subscription(&self.tray).map(AppMessage::Tray),
            dashboard::subscription(
                wants_connection
                    && self.navigation.current() == Screen::Dashboard
                    && !self.window.is_backgrounded(),
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
                window::update(&mut self.window, message, self.tray.is_available())
                    .map(AppMessage::Window)
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
            AppMessage::Tray(message) => {
                let action = tray::update(&self.tray, message);
                self.handle_action(action)
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
                self.theme_mode,
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
            .padding(CONTENT_PADDING)
            .width(Fill)
            .height(Fill)
            .style(theme::content);

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
                self.theme_mode = mode;
                Task::none()
            },
            Action::ShowWindow => window::show(&mut self.window).map(AppMessage::Window),
            Action::ExitApplication => iced::exit(),
        }
    }

    fn save_configuration(&mut self) {
        let config = DesktopConfig {
            server_addr: self.session.server_addr().to_owned(),
            dark_theme: self.theme_mode == ThemeMode::Dark,
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
