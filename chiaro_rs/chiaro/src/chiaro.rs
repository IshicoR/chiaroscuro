use std::path::PathBuf;

use chiaro_actions::{Action, Screen};
use chiaro_config::DesktopConfig;
use chiaro_dashboard_ui::{self as dashboard, DashboardMessage, DashboardState};
use chiaro_ibt_picker as ibt;
use chiaro_live_telemetry::{self as telemetry, LiveTelemetryMessage};
use chiaro_navigation_ui::{self as navigation, Navigation};
use chiaro_settings_ui::{self as settings, SettingsMessage, SettingsState};
use chiaro_theme::{self as theme, typography};
use chiaro_tray::{self as tray, TrayMessage};
use chiaro_window::{self as window, WindowMessage, WindowState};
use chiaro_telemetry::{LoadedIbt, Session};
use iced::{
    Element,
    Length::Fill,
    Subscription, Task, Theme,
    widget::{self, column, container},
};
use iced_fonts::LUCIDE_FONT_BYTES;

const CONTENT_PADDING: f32 = 24.0;
const APP_TITLE: &str = "Chiaroscuro";

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

fn main() -> iced::Result {
    Chiaroscuro::run()
}

#[derive(Debug, Default)]
struct Chiaroscuro {
    navigation: Navigation,
    session: Session,
    reference_session: Option<Session>,
    window: WindowState,
    dashboard: DashboardState,
    settings: SettingsState,
    tray: tray::TrayState,
    configuration_error: Option<String>,
}

#[derive(Debug, Clone)]
enum AppMessage {
    Navigation(navigation::NavigationMessage),
    Window(WindowMessage),
    Dashboard(DashboardMessage),
    Settings(SettingsMessage),
    Telemetry(LiveTelemetryMessage),
    Tray(TrayMessage),
    IbtSelected(Option<PathBuf>),
    IbtLoaded(Result<LoadedIbt, String>),
    ReferenceIbtSelected(Option<PathBuf>),
    ReferenceIbtLoaded(Result<LoadedIbt, String>),
}

impl Chiaroscuro {
    fn run() -> iced::Result {
        iced::application(Chiaroscuro::new, Chiaroscuro::update, Chiaroscuro::view)
            .settings(iced::Settings {
                default_text_size: 20.0.into(),
                ..iced::Settings::default()
            })
            .title(Chiaroscuro::title)
            .theme(Chiaroscuro::theme)
            .style(|_, app_theme| theme::application(app_theme))
            .subscription(Chiaroscuro::subscription)
            .font(LUCIDE_FONT_BYTES)
            .font(typography::SANS_JP_REGULAR_BYTES)
            .default_font(typography::SANS_SEMIBOLD)
            .window(window::settings())
            .antialiasing(true)
            .centered()
            .run()
    }

    fn new() -> Self {
        let mut app = Self::default();
        match tray::TrayState::new() {
            Ok(tray) => app.tray = tray,
            Err(error) => eprintln!("failed to initialize system tray: {error}"),
        }

        match DesktopConfig::load() {
            Ok(config) => {
                app.settings.set_show_diagnostics(config.show_diagnostics);
            },
            Err(error) => {
                app.configuration_error = Some(format!("failed to load desktop settings: {error}"));
            },
        }

        app
    }

    fn title(&self) -> String {
        APP_TITLE.to_owned()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        let wants_connection = self.session.wants_connection();
        let telemetry = if wants_connection {
            telemetry::subscription().map(AppMessage::Telemetry)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            window::subscription(&self.window).map(AppMessage::Window),
            tray::subscription(&self.tray).map(AppMessage::Tray),
            dashboard::subscription(
                &self.dashboard,
                wants_connection
                    && self.navigation.current() == Screen::Dashboard
                    && !self.window.is_backgrounded(),
            )
            .map(AppMessage::Dashboard),
            telemetry,
        ])
    }

    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Navigation(message) => {
                let action =
                    navigation::update(&mut self.navigation, message).map(Action::Navigate);
                self.handle_action(action)
            },
            AppMessage::Window(message) => {
                window::update(&mut self.window, message, self.tray.is_available())
                    .map(AppMessage::Window)
            },
            AppMessage::Dashboard(message) => {
                let action = dashboard::update(
                    &mut self.dashboard,
                    &self.session,
                    self.reference_session.as_ref(),
                    message,
                );
                self.handle_action(action)
            },
            AppMessage::Settings(message) => {
                let action = settings::update(&mut self.settings, message);
                let action = self.handle_action(action);
                self.save_configuration();

                action
            },
            AppMessage::Telemetry(event) => {
                match event {
                    telemetry::LiveTelemetryMessage::Waiting => self.session.mark_waiting(),
                    telemetry::LiveTelemetryMessage::Connected => self.session.mark_connected(),
                    telemetry::LiveTelemetryMessage::Snapshot {
                        snapshot,
                        session_info,
                    } => self.session.record_snapshot(snapshot, session_info),
                    telemetry::LiveTelemetryMessage::Error(error) => self.session.mark_error(error),
                }
                Task::none()
            },
            AppMessage::IbtSelected(path) => {
                let Some(path) = path else {
                    self.dashboard.finish_ibt_load();
                    return Task::none();
                };

                self.dashboard.begin_ibt_load();
                self.session.begin_ibt_load();
                Task::perform(ibt::load(path), AppMessage::IbtLoaded)
            },
            AppMessage::IbtLoaded(result) => {
                self.dashboard.finish_ibt_load();
                match result {
                    Ok(recording) => {
                        self.session.load_ibt(recording);
                        dashboard::reset_telemetry(
                            &mut self.dashboard,
                            &self.session,
                            self.reference_session.as_ref(),
                        );
                    },
                    Err(error) => self.session.mark_ibt_error(error),
                }
                Task::none()
            },
            AppMessage::ReferenceIbtSelected(path) => {
                let Some(path) = path else {
                    self.dashboard.finish_reference_ibt_load();
                    return Task::none();
                };

                self.dashboard.begin_reference_ibt_load();
                Task::perform(ibt::load(path), AppMessage::ReferenceIbtLoaded)
            },
            AppMessage::ReferenceIbtLoaded(result) => {
                self.dashboard.finish_reference_ibt_load();
                match result {
                    Ok(recording) => {
                        let mut reference = Session::default();
                        reference.load_ibt(recording);
                        self.reference_session = Some(reference);
                        dashboard::reset_reference_telemetry(
                            &mut self.dashboard,
                            &self.session,
                            self.reference_session.as_ref(),
                        );
                    },
                    Err(error) => self.dashboard.mark_reference_ibt_error(error),
                }
                Task::none()
            },
            AppMessage::Tray(message) => {
                let action = tray::update(&self.tray, message);
                self.handle_action(action)
            },
        }
    }

    fn view(&self) -> Element<'_, AppMessage> {
        let rounded = self.window.uses_rounded_corners();
        let screen = match self.navigation.current() {
            Screen::Dashboard => dashboard::view(
                &self.dashboard,
                &self.session,
                self.reference_session.as_ref(),
            )
            .map(AppMessage::Dashboard),
            Screen::Settings => settings::view(
                &self.settings,
                &self.session,
                self.configuration_error.as_deref(),
            )
            .map(AppMessage::Settings),
        };

        let navigation = navigation::view(&self.navigation, rounded).map(AppMessage::Navigation);
        let content = container(screen)
            .padding(CONTENT_PADDING)
            .width(Fill)
            .height(Fill)
            .style(move |theme| theme::content(theme, rounded));

        let layout = column![
            window::view(&self.window, APP_TITLE, AppMessage::Window),
            widget::row![navigation, content].height(Fill),
        ]
        .width(Fill)
        .height(Fill);

        widget::stack([
            layout.into(),
            window::focus_border(&self.window).map(AppMessage::Window),
            window::resize_handles().map(AppMessage::Window),
        ])
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
            Action::OpenIbt => {
                self.dashboard.begin_ibt_selection();
                Task::perform(ibt::select_file(), AppMessage::IbtSelected)
            },
            Action::OpenReferenceIbt => {
                self.dashboard.begin_reference_ibt_selection();
                Task::perform(ibt::select_file(), AppMessage::ReferenceIbtSelected)
            },
            Action::ClearReferenceIbt => {
                self.reference_session = None;
                dashboard::reset_reference_telemetry(
                    &mut self.dashboard,
                    &self.session,
                    self.reference_session.as_ref(),
                );
                Task::none()
            },
            Action::SetConnected(connected) => {
                self.session.set_connection_requested(connected);
                dashboard::reset_telemetry(
                    &mut self.dashboard,
                    &self.session,
                    self.reference_session.as_ref(),
                );
                Task::none()
            },
            Action::ShowWindow => window::show(&mut self.window).map(AppMessage::Window),
            Action::ExitApplication => iced::exit(),
        }
    }

    fn save_configuration(&mut self) {
        let config = DesktopConfig {
            show_diagnostics: self.settings.show_diagnostics(),
        };

        self.configuration_error = config
            .save()
            .err()
            .map(|error| format!("failed to save desktop settings: {error}"));
    }
}
