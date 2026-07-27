use std::collections::BTreeMap;

use chiaro_actions::{Action, Screen};
use chiaro_car_setup_ui::{
    self as car_setup, CarSetupLayout, CarSetupLayoutFlag, CarSetupMessage, CarSetupState,
};
use chiaro_config::{DASHBOARD_LAYOUT_SCHEMA_VERSION, DashboardLayoutConfig, DesktopConfig};
use chiaro_i18n::{Text, Translations, set_locale, tr};
use chiaro_ibt_picker as ibt;
use chiaro_live_telemetry::{self as live_telemetry, LiveTelemetryMessage, LiveTelemetrySource};
use chiaro_navigation_ui::{self as navigation, Navigation};
use chiaro_settings_ui::{self as settings, SettingsMessage, SettingsState};
use chiaro_telemetry::{LoadedIbt, RecordingSource, Session};
use chiaro_telemetry_ui::{
    self as telemetry_ui, TelemetryLayout, TelemetryLayoutFlag, TelemetryMessage, TelemetryState,
};
use chiaro_tray::{self as tray, TrayMessage};
use chiaro_widgets::{surface, typography};
use chiaro_window::{self as window, WindowMessage, WindowState};
use iced::{
    Color, Element,
    Length::Fill,
    Subscription, Task, Theme, keyboard,
    theme::Palette,
    widget::{self, column, container},
};
use iced_fonts::LUCIDE_FONT_BYTES;

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

const CONTENT_PADDING: f32 = 24.0;
const APP_TITLE: &str = "Chiaroscuro";
const APPLICATION_ID: &str = "io.github.IshicoR.Chiaroscuro";
const COOL_CARBON_DARK: Palette = Palette {
    background: Color::from_rgb8(0x15, 0x17, 0x1B),
    text: Color::from_rgb8(0xF4, 0xF4, 0xF4),
    primary: Color::from_rgb8(0x45, 0x89, 0xFF),
    success: Color::from_rgb8(0x42, 0xBE, 0x65),
    warning: Color::from_rgb8(0xF1, 0xC2, 0x1B),
    danger: Color::from_rgb8(0xFA, 0x4D, 0x56),
};

fn main() -> iced::Result {
    // Must run before Iced starts: the installed launcher can use this process
    // for install, update, and restart lifecycle operations.
    velopack::VelopackApp::build().run();
    Chiaroscuro::run()
}

#[derive(Debug, Default)]
struct Chiaroscuro {
    navigation: Navigation,
    session: Session,
    reference_session: Option<Session>,
    window: WindowState,
    telemetry: TelemetryState,
    car_setup: CarSetupState,
    settings: SettingsState,
    tray: tray::TrayState,
    live_telemetry_source: LiveTelemetrySource,
    configuration: DesktopConfig,
    configuration_error: Option<String>,
}

#[derive(Debug, Clone)]
enum AppMessage {
    Navigation(navigation::NavigationMessage),
    Window(WindowMessage),
    Telemetry(TelemetryMessage),
    CarSetup(CarSetupMessage),
    Settings(SettingsMessage),
    LiveTelemetry(LiveTelemetryMessage),
    Tray(TrayMessage),
    IbtSelected(Option<RecordingSource>),
    IbtLoaded(Result<LoadedIbt, String>),
    ReferenceIbtSelected(Option<RecordingSource>),
    ReferenceIbtLoaded(Result<LoadedIbt, String>),
    FocusNext,
    FocusPrevious,
}

impl Chiaroscuro {
    fn run() -> iced::Result {
        iced::application(Chiaroscuro::new, Chiaroscuro::update, Chiaroscuro::view)
            .settings(iced::Settings {
                id: Some(APPLICATION_ID.to_owned()),
                default_text_size: 20.0.into(),
                ..iced::Settings::default()
            })
            .title(Chiaroscuro::title)
            .theme(Chiaroscuro::theme)
            .style(|_, app_theme| surface::application(app_theme))
            .subscription(Chiaroscuro::subscription)
            .font(LUCIDE_FONT_BYTES)
            .font(typography::SANS_REGULAR_BYTES)
            .font(typography::SANS_SEMIBOLD_BYTES)
            .font(typography::MONO_REGULAR_BYTES)
            .default_font(typography::SANS)
            .window(window::settings())
            .antialiasing(true)
            .centered()
            .run()
    }

    fn new() -> Self {
        let configuration = DesktopConfig::load();
        if let Ok(config) = &configuration {
            set_locale(config.locale);
        }
        let mut app = Self::default();
        match tray::TrayState::new() {
            Ok(tray) => app.tray = tray,
            Err(error) => eprintln!("failed to initialize system tray: {error}"),
        }

        match configuration {
            Ok(config) => {
                app.settings.set_show_diagnostics(config.show_diagnostics);
                app.settings.set_locale(config.locale);
                if let Some(layout) = config
                    .dashboard
                    .as_ref()
                    .and_then(telemetry_layout_from_config)
                {
                    app.telemetry.apply_layout(&layout);
                }
                if let Some(layout) = config
                    .dashboard
                    .as_ref()
                    .and_then(car_setup_layout_from_config)
                {
                    app.car_setup.apply_layout(&layout);
                }
                app.configuration = config;
            },
            Err(error) => {
                app.configuration_error = Some(format!(
                    "{}: {error}",
                    tr(Text::FailedToLoadDesktopSettings)
                ));
            },
        }

        app
    }

    fn title(&self) -> String {
        APP_TITLE.to_owned()
    }

    fn theme(&self) -> Theme {
        // Theme::custom("Cool Carbon Dark", COOL_CARBON_DARK)
        Theme::TokyoNightStorm
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        let wants_connection = self.session.wants_connection();
        let live_source_available = self.live_telemetry_source.info().is_available();
        let window_is_backgrounded = self.window.is_backgrounded();
        let telemetry_active = screen_is_foreground(
            self.navigation.current(),
            Screen::Telemetry,
            window_is_backgrounded,
        );
        let car_setup_active = screen_is_foreground(
            self.navigation.current(),
            Screen::CarSetup,
            window_is_backgrounded,
        );
        let live_telemetry = if wants_connection && live_source_available {
            live_telemetry::subscription(&self.live_telemetry_source).map(AppMessage::LiveTelemetry)
        } else {
            Subscription::none()
        };
        let focus_navigation = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            } if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
                Some(if modifiers.shift() {
                    AppMessage::FocusPrevious
                } else {
                    AppMessage::FocusNext
                })
            },
            _ => None,
        });

        Subscription::batch([
            window::subscription().map(AppMessage::Window),
            tray::subscription(&self.tray).map(AppMessage::Tray),
            telemetry_ui::subscription(&self.telemetry, telemetry_active)
                .map(AppMessage::Telemetry),
            car_setup::subscription(&self.car_setup, car_setup_active).map(AppMessage::CarSetup),
            live_telemetry,
            focus_navigation,
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
                let was_backgrounded = self.window.is_backgrounded();
                let task = window::update(&mut self.window, message, self.tray.is_available());
                if window_became_foreground(was_backgrounded, self.window.is_backgrounded()) {
                    self.refresh_workspace();
                }
                task.map(AppMessage::Window)
            },
            AppMessage::Telemetry(message) => self.update_telemetry(message),
            AppMessage::CarSetup(message) => {
                self.update_car_setup(message);
                Task::none()
            },
            AppMessage::Settings(message) => {
                let persists_configuration = message.persists_configuration();
                let changes_locale = matches!(
                    message,
                    SettingsMessage::SetLocale(locale) if locale != self.settings.locale()
                );
                let action = settings::update(&mut self.settings, message);
                let action = self.handle_action(action);

                if persists_configuration {
                    self.configuration.show_diagnostics = self.settings.show_diagnostics();
                    self.configuration.locale = self.settings.locale();
                    self.save_configuration();
                }
                if changes_locale {
                    self.relocalize();
                }

                action
            },
            AppMessage::LiveTelemetry(event) => {
                let refreshes_workspace = telemetry_refreshes_workspace(&event);
                match event {
                    live_telemetry::LiveTelemetryMessage::Waiting => self.session.mark_waiting(),
                    live_telemetry::LiveTelemetryMessage::Connected => {
                        self.session.mark_connected();
                    },
                    live_telemetry::LiveTelemetryMessage::Batch(batch) => {
                        self.session.record_live_batch(
                            batch
                                .samples
                                .into_iter()
                                .flatten()
                                .map(|captured| (captured.captured_at, captured.sample)),
                            batch.latest_frame,
                            batch.session_info,
                        );
                    },
                    live_telemetry::LiveTelemetryMessage::Snapshot {
                        snapshot,
                        session_info,
                    } => self.session.record_snapshot(snapshot, session_info),
                    live_telemetry::LiveTelemetryMessage::Error(error) => {
                        self.session.mark_error(error);
                    },
                }
                if refreshes_workspace && self.workspace_is_foreground() {
                    self.refresh_workspace();
                }
                Task::none()
            },
            AppMessage::IbtSelected(path) => {
                let Some(path) = path else {
                    self.telemetry.finish_ibt_load();
                    return Task::none();
                };

                self.telemetry.begin_ibt_load();
                self.session.begin_ibt_load();
                Task::perform(ibt::load(path), AppMessage::IbtLoaded)
            },
            AppMessage::IbtLoaded(result) => {
                self.telemetry.finish_ibt_load();
                match result {
                    Ok(recording) => {
                        self.session.load_ibt(recording);
                        let telemetry_active = self.telemetry_is_active();
                        let car_setup_active = self.car_setup_is_active();
                        telemetry_ui::reset_session(
                            &mut self.telemetry,
                            &self.session,
                            self.reference_session.as_ref(),
                            telemetry_active,
                        );
                        car_setup::reset_session(
                            &mut self.car_setup,
                            &self.session,
                            car_setup_active,
                        );
                    },
                    Err(error) => self.session.mark_ibt_error(error),
                }
                Task::none()
            },
            AppMessage::ReferenceIbtSelected(path) => {
                let Some(path) = path else {
                    self.telemetry.finish_reference_ibt_load();
                    return Task::none();
                };

                self.telemetry.begin_reference_ibt_load();
                Task::perform(ibt::load(path), AppMessage::ReferenceIbtLoaded)
            },
            AppMessage::ReferenceIbtLoaded(result) => {
                self.telemetry.finish_reference_ibt_load();
                match result {
                    Ok(recording) => {
                        let mut reference = Session::default();
                        reference.load_ibt(recording);
                        self.reference_session = Some(reference);
                        let telemetry_active = self.telemetry_is_active();
                        telemetry_ui::reset_reference(
                            &mut self.telemetry,
                            &self.session,
                            self.reference_session.as_ref(),
                            telemetry_active,
                        );
                    },
                    Err(error) => self.telemetry.mark_reference_ibt_error(error),
                }
                Task::none()
            },
            AppMessage::Tray(message) => {
                let action = tray::update(&self.tray, message);
                self.handle_action(action)
            },
            AppMessage::FocusNext => widget::operation::focus_next(),
            AppMessage::FocusPrevious => widget::operation::focus_previous(),
        }
    }

    fn view(&self) -> Element<'_, AppMessage> {
        let rounded = self.window.uses_rounded_corners();
        let current_screen = self.navigation.current();
        let screen = match current_screen {
            Screen::Telemetry => telemetry_ui::view(
                &self.telemetry,
                &self.session,
                self.reference_session.as_ref(),
                self.live_telemetry_source.info(),
            )
            .map(AppMessage::Telemetry),
            Screen::CarSetup => car_setup::view(
                &self.car_setup,
                &self.session,
                self.live_telemetry_source.info(),
            )
            .map(AppMessage::CarSetup),
            Screen::Settings => settings::view(
                &self.settings,
                &self.session,
                self.configuration_error.as_deref(),
                self.live_telemetry_source.info(),
            )
            .map(AppMessage::Settings),
        };
        let content_padding = screen_content_padding(current_screen);
        let navigation_width = navigation::WIDTH;

        let navigation = navigation::view(
            &self.navigation,
            rounded,
            Translations::new(self.settings.locale()),
        )
        .map(AppMessage::Navigation);
        let content = container(screen)
            .padding(content_padding)
            .width(Fill)
            .height(Fill)
            .style(move |theme| surface::content(theme, rounded));
        let workspace = container(widget::row![navigation, content].height(Fill))
            .width(Fill)
            .height(Fill)
            .style(move |theme| surface::workspace(theme, rounded));

        let layout = column![
            window::view(&self.window, navigation_width, AppMessage::Window),
            workspace,
        ]
        .width(Fill)
        .height(Fill);

        let layout = widget::stack([
            layout.into(),
            window::resize_handles().map(AppMessage::Window),
        ])
        .width(Fill)
        .height(Fill);

        match current_screen {
            Screen::Telemetry | Screen::CarSetup => layout.into(),
            Screen::Settings => {
                settings::with_dialog_preview(layout, &self.settings, AppMessage::Settings)
            },
        }
    }

    fn handle_action(&mut self, action: Option<Action>) -> Task<AppMessage> {
        let Some(action) = action else {
            return Task::none();
        };

        match action {
            Action::Navigate(page) => self.navigate_to(page),
            Action::OpenIbt => {
                self.telemetry.begin_ibt_selection();
                Task::perform(ibt::select_file(), AppMessage::IbtSelected)
            },
            Action::OpenReferenceIbt => {
                self.telemetry.begin_reference_ibt_selection();
                Task::perform(ibt::select_file(), AppMessage::ReferenceIbtSelected)
            },
            Action::ClearReferenceIbt => {
                self.reference_session = None;
                let telemetry_active = self.telemetry_is_active();
                telemetry_ui::reset_reference(
                    &mut self.telemetry,
                    &self.session,
                    self.reference_session.as_ref(),
                    telemetry_active,
                );
                Task::none()
            },
            Action::SetConnected(connected) => {
                if connected && !self.live_telemetry_source.info().is_available() {
                    return Task::none();
                }
                self.session.set_connection_requested(connected);
                let telemetry_active = self.telemetry_is_active();
                let car_setup_active = self.car_setup_is_active();
                telemetry_ui::reset_session(
                    &mut self.telemetry,
                    &self.session,
                    self.reference_session.as_ref(),
                    telemetry_active,
                );
                car_setup::reset_session(&mut self.car_setup, &self.session, car_setup_active);
                Task::none()
            },
            Action::ShowWindow => {
                let was_backgrounded = self.window.is_backgrounded();
                let task = window::show(&mut self.window);
                if window_became_foreground(was_backgrounded, self.window.is_backgrounded()) {
                    self.refresh_workspace();
                }
                task.map(AppMessage::Window)
            },
            Action::ExitApplication => iced::exit(),
        }
    }

    fn navigate_to(&mut self, screen: Screen) -> Task<AppMessage> {
        let current = self.navigation.current();
        if screen == current {
            return Task::none();
        }

        match current {
            Screen::Telemetry => telemetry_ui::deactivate(&mut self.telemetry),
            Screen::CarSetup => car_setup::deactivate(&mut self.car_setup),
            Screen::Settings => {},
        }
        self.navigation.navigate(screen);
        match screen {
            Screen::Telemetry => telemetry_ui::activate(
                &mut self.telemetry,
                &self.session,
                self.reference_session.as_ref(),
            ),
            Screen::CarSetup => {
                car_setup::activate(&mut self.car_setup, &self.session);
            },
            Screen::Settings => {},
        }
        Task::none()
    }

    fn save_configuration(&mut self) {
        self.configuration_error = self
            .configuration
            .save()
            .err()
            .map(|error| format!("{}: {error}", tr(Text::FailedToSaveDesktopSettings)));
    }

    fn telemetry_is_active(&self) -> bool {
        self.navigation.current() == Screen::Telemetry
    }

    fn car_setup_is_active(&self) -> bool {
        self.navigation.current() == Screen::CarSetup
    }

    fn workspace_is_foreground(&self) -> bool {
        workspace_is_foreground(self.navigation.current(), self.window.is_backgrounded())
    }

    fn refresh_workspace(&mut self) {
        match self.navigation.current() {
            Screen::Telemetry => telemetry_ui::refresh(
                &mut self.telemetry,
                &self.session,
                self.reference_session.as_ref(),
            ),
            Screen::CarSetup => car_setup::refresh(&mut self.car_setup, &self.session),
            Screen::Settings => {},
        }
    }

    fn relocalize(&mut self) {
        let telemetry_layout = self.telemetry.layout_snapshot();
        let car_setup_layout = self.car_setup.layout_snapshot();

        self.telemetry = TelemetryState::default();
        self.telemetry.apply_layout(&telemetry_layout);
        telemetry_ui::refresh(
            &mut self.telemetry,
            &self.session,
            self.reference_session.as_ref(),
        );

        self.car_setup = CarSetupState::default();
        self.car_setup.apply_layout(&car_setup_layout);
        car_setup::refresh(&mut self.car_setup, &self.session);

        match tray::TrayState::new() {
            Ok(tray) => self.tray = tray,
            Err(error) => eprintln!("failed to refresh system tray locale: {error}"),
        }
    }

    fn update_telemetry(&mut self, message: TelemetryMessage) -> Task<AppMessage> {
        let layout_revision = self.telemetry.layout_revision();
        let resets_layout = message.resets_layout();
        let action = telemetry_ui::update(
            &mut self.telemetry,
            &self.session,
            self.reference_session.as_ref(),
            message,
        );

        let changed_layout = (self.telemetry.layout_revision() != layout_revision)
            .then(|| self.telemetry.layout_snapshot());
        if update_persisted_telemetry_layout(&mut self.configuration, resets_layout, changed_layout)
        {
            self.save_configuration();
        }

        self.handle_action(action)
    }

    fn update_car_setup(&mut self, message: CarSetupMessage) {
        let layout_revision = self.car_setup.layout_revision();
        let resets_layout = message.resets_layout();
        car_setup::update(&mut self.car_setup, message);

        let changed_layout = (self.car_setup.layout_revision() != layout_revision)
            .then(|| self.car_setup.layout_snapshot());
        if update_persisted_car_setup_layout(&mut self.configuration, resets_layout, changed_layout)
        {
            self.save_configuration();
        }
    }
}

fn telemetry_layout_from_config(config: &DashboardLayoutConfig) -> Option<TelemetryLayout> {
    (config.schema_version == DASHBOARD_LAYOUT_SCHEMA_VERSION).then(|| TelemetryLayout {
        chart_order: config.chart_order.clone(),
        chart_visibility: telemetry_flags_from_config(&config.chart_visibility),
        chart_collapsed: telemetry_flags_from_config(&config.chart_collapsed),
        chart_columns: config.chart_columns,
        setup_card_order: config.setup_card_order.clone(),
        setup_card_collapsed: telemetry_flags_from_config(&config.setup_card_collapsed),
        lap_analysis_order: config.lap_analysis_order.clone(),
        lap_analysis_collapsed: telemetry_flags_from_config(&config.lap_analysis_collapsed),
    })
}

fn car_setup_layout_from_config(config: &DashboardLayoutConfig) -> Option<CarSetupLayout> {
    (config.schema_version == DASHBOARD_LAYOUT_SCHEMA_VERSION).then(|| CarSetupLayout {
        card_order: config.car_setup_card_order.clone(),
        card_collapsed: config
            .car_setup_card_collapsed
            .iter()
            .map(|(key, value)| CarSetupLayoutFlag {
                key: key.clone(),
                value: *value,
            })
            .collect(),
    })
}

fn telemetry_flags_from_config(flags: &BTreeMap<String, bool>) -> Vec<TelemetryLayoutFlag> {
    flags
        .iter()
        .map(|(key, value)| TelemetryLayoutFlag {
            key: key.clone(),
            value: *value,
        })
        .collect()
}

fn telemetry_flags_to_config(flags: Vec<TelemetryLayoutFlag>) -> BTreeMap<String, bool> {
    flags
        .into_iter()
        .map(|flag| (flag.key, flag.value))
        .collect()
}

fn apply_telemetry_layout(config: &mut DashboardLayoutConfig, layout: TelemetryLayout) {
    config.chart_order = layout.chart_order;
    config.chart_visibility = telemetry_flags_to_config(layout.chart_visibility);
    config.chart_collapsed = telemetry_flags_to_config(layout.chart_collapsed);
    config.chart_columns = layout.chart_columns;
    config.setup_card_order = layout.setup_card_order;
    config.setup_card_collapsed = telemetry_flags_to_config(layout.setup_card_collapsed);
    config.lap_analysis_order = layout.lap_analysis_order;
    config.lap_analysis_collapsed = telemetry_flags_to_config(layout.lap_analysis_collapsed);
}

fn apply_car_setup_layout(config: &mut DashboardLayoutConfig, layout: CarSetupLayout) {
    config.car_setup_card_order = layout.card_order;
    config.car_setup_card_collapsed = layout
        .card_collapsed
        .into_iter()
        .map(|flag| (flag.key, flag.value))
        .collect();
}

fn reset_telemetry_layout(config: &mut DashboardLayoutConfig) {
    let defaults = DashboardLayoutConfig::default();
    config.chart_order = defaults.chart_order;
    config.chart_visibility = defaults.chart_visibility;
    config.chart_collapsed = defaults.chart_collapsed;
    config.chart_columns = defaults.chart_columns;
    config.setup_card_order = defaults.setup_card_order;
    config.setup_card_collapsed = defaults.setup_card_collapsed;
    config.lap_analysis_order = defaults.lap_analysis_order;
    config.lap_analysis_collapsed = defaults.lap_analysis_collapsed;
}

fn reset_car_setup_layout(config: &mut DashboardLayoutConfig) {
    let defaults = DashboardLayoutConfig::default();
    config.car_setup_card_order = defaults.car_setup_card_order;
    config.car_setup_card_collapsed = defaults.car_setup_card_collapsed;
}

fn dashboard_config_for_update(configuration: &mut DesktopConfig) -> &mut DashboardLayoutConfig {
    let uses_known_schema = configuration
        .dashboard
        .as_ref()
        .is_none_or(|config| config.schema_version == DASHBOARD_LAYOUT_SCHEMA_VERSION);
    if !uses_known_schema {
        configuration.dashboard = None;
    }
    configuration
        .dashboard
        .get_or_insert_with(DashboardLayoutConfig::default)
}

fn remove_default_dashboard_config(configuration: &mut DesktopConfig) {
    if configuration.dashboard.as_ref() == Some(&DashboardLayoutConfig::default()) {
        configuration.dashboard = None;
    }
}

fn update_persisted_telemetry_layout(
    configuration: &mut DesktopConfig,
    resets_layout: bool,
    changed_layout: Option<TelemetryLayout>,
) -> bool {
    let changed = changed_layout.is_some();
    let previous = configuration.dashboard.clone();
    if resets_layout {
        reset_telemetry_layout(dashboard_config_for_update(configuration));
    } else if let Some(layout) = changed_layout {
        apply_telemetry_layout(dashboard_config_for_update(configuration), layout);
    } else {
        return false;
    }

    remove_default_dashboard_config(configuration);
    changed || configuration.dashboard != previous
}

fn update_persisted_car_setup_layout(
    configuration: &mut DesktopConfig,
    resets_layout: bool,
    changed_layout: Option<CarSetupLayout>,
) -> bool {
    let changed = changed_layout.is_some();
    let previous = configuration.dashboard.clone();
    if resets_layout {
        reset_car_setup_layout(dashboard_config_for_update(configuration));
    } else if let Some(layout) = changed_layout {
        apply_car_setup_layout(dashboard_config_for_update(configuration), layout);
    } else {
        return false;
    }

    remove_default_dashboard_config(configuration);
    changed || configuration.dashboard != previous
}

fn workspace_is_foreground(screen: Screen, window_is_backgrounded: bool) -> bool {
    matches!(screen, Screen::Telemetry | Screen::CarSetup) && !window_is_backgrounded
}

fn window_became_foreground(was_backgrounded: bool, is_backgrounded: bool) -> bool {
    was_backgrounded && !is_backgrounded
}

fn screen_is_foreground(current: Screen, target: Screen, window_is_backgrounded: bool) -> bool {
    current == target && !window_is_backgrounded
}

fn screen_content_padding(screen: Screen) -> f32 {
    match screen {
        Screen::Telemetry | Screen::CarSetup => 0.0,
        Screen::Settings => CONTENT_PADDING,
    }
}

fn telemetry_refreshes_workspace(event: &LiveTelemetryMessage) -> bool {
    matches!(
        event,
        LiveTelemetryMessage::Batch(_) | LiveTelemetryMessage::Snapshot { .. }
    )
}

#[cfg(test)]
mod tests {
    use chiaro_actions::Screen;
    use chiaro_car_setup_ui::{CarSetupLayout, CarSetupLayoutFlag};
    use chiaro_config::{DASHBOARD_LAYOUT_SCHEMA_VERSION, DashboardLayoutConfig, DesktopConfig};
    use chiaro_live_telemetry::{LiveTelemetryBatch, LiveTelemetryMessage};
    use chiaro_telemetry_ui::{TelemetryLayout, TelemetryState};

    use super::{
        car_setup_layout_from_config, screen_content_padding, screen_is_foreground,
        telemetry_layout_from_config, telemetry_refreshes_workspace,
        update_persisted_car_setup_layout, update_persisted_telemetry_layout,
        window_became_foreground, workspace_is_foreground,
    };

    fn car_setup_layout() -> CarSetupLayout {
        CarSetupLayout {
            card_order: vec![
                "summary".to_owned(),
                "vehicle:specifications".to_owned(),
                "setup:tires".to_owned(),
            ],
            card_collapsed: vec![CarSetupLayoutFlag {
                key: "setup:tires".to_owned(),
                value: true,
            }],
        }
    }

    fn assert_telemetry_layout_equivalent(
        actual: Option<TelemetryLayout>,
        expected: &TelemetryLayout,
    ) {
        let mut state = TelemetryState::default();
        state.apply_layout(&actual.expect("known Telemetry layout"));
        assert_eq!(&state.layout_snapshot(), expected);
    }

    #[test]
    fn legacy_dashboard_config_restores_both_screen_layouts() {
        let telemetry = TelemetryLayout {
            chart_columns: 2,
            ..TelemetryLayout::default()
        };
        let car_setup = car_setup_layout();
        let mut configuration = DesktopConfig::default();

        assert!(update_persisted_telemetry_layout(
            &mut configuration,
            false,
            Some(telemetry.clone()),
        ));
        assert!(update_persisted_car_setup_layout(
            &mut configuration,
            false,
            Some(car_setup.clone()),
        ));

        let persisted = configuration.dashboard.as_ref().unwrap();
        assert_telemetry_layout_equivalent(telemetry_layout_from_config(persisted), &telemetry);
        assert_eq!(car_setup_layout_from_config(persisted), Some(car_setup));
    }

    #[test]
    fn layouts_from_an_unknown_schema_are_ignored() {
        let config = DashboardLayoutConfig {
            schema_version: DASHBOARD_LAYOUT_SCHEMA_VERSION + 1,
            ..DashboardLayoutConfig::default()
        };

        assert_eq!(telemetry_layout_from_config(&config), None);
        assert_eq!(car_setup_layout_from_config(&config), None);
    }

    #[test]
    fn telemetry_changes_preserve_car_setup_configuration() {
        let mut configuration = DesktopConfig::default();
        let car_setup = car_setup_layout();
        update_persisted_car_setup_layout(&mut configuration, false, Some(car_setup.clone()));

        update_persisted_telemetry_layout(
            &mut configuration,
            false,
            Some(TelemetryLayout {
                chart_columns: 2,
                ..TelemetryLayout::default()
            }),
        );

        assert_eq!(
            car_setup_layout_from_config(configuration.dashboard.as_ref().unwrap()),
            Some(car_setup)
        );
    }

    #[test]
    fn car_setup_changes_preserve_telemetry_configuration() {
        let mut configuration = DesktopConfig::default();
        let telemetry = TelemetryLayout {
            chart_columns: 2,
            ..TelemetryLayout::default()
        };
        update_persisted_telemetry_layout(&mut configuration, false, Some(telemetry.clone()));

        update_persisted_car_setup_layout(&mut configuration, false, Some(car_setup_layout()));

        assert_telemetry_layout_equivalent(
            telemetry_layout_from_config(configuration.dashboard.as_ref().unwrap()),
            &telemetry,
        );
    }

    #[test]
    fn screen_resets_remove_only_their_own_override() {
        let mut configuration = DesktopConfig::default();
        let car_setup = car_setup_layout();
        update_persisted_telemetry_layout(
            &mut configuration,
            false,
            Some(TelemetryLayout {
                chart_columns: 2,
                ..TelemetryLayout::default()
            }),
        );
        update_persisted_car_setup_layout(&mut configuration, false, Some(car_setup.clone()));

        assert!(update_persisted_telemetry_layout(
            &mut configuration,
            true,
            Some(TelemetryLayout::default()),
        ));
        assert_eq!(
            car_setup_layout_from_config(configuration.dashboard.as_ref().unwrap()),
            Some(car_setup)
        );

        assert!(update_persisted_car_setup_layout(
            &mut configuration,
            true,
            Some(CarSetupLayout::default()),
        ));
        assert_eq!(configuration.dashboard, None);
    }

    #[test]
    fn only_the_selected_screen_is_foreground() {
        assert!(screen_is_foreground(
            Screen::Telemetry,
            Screen::Telemetry,
            false
        ));
        assert!(!screen_is_foreground(
            Screen::CarSetup,
            Screen::Telemetry,
            false
        ));
        assert!(!screen_is_foreground(
            Screen::Telemetry,
            Screen::Telemetry,
            true
        ));
        assert!(workspace_is_foreground(Screen::CarSetup, false));
        assert!(!workspace_is_foreground(Screen::Settings, false));
    }

    #[test]
    fn restoring_a_backgrounded_window_requests_a_workspace_refresh() {
        assert!(window_became_foreground(true, false));
        assert!(!window_became_foreground(false, false));
        assert!(!window_became_foreground(false, true));
        assert!(!window_became_foreground(true, true));
    }

    #[test]
    fn workspace_screens_own_their_content_padding() {
        assert_eq!(screen_content_padding(Screen::Telemetry), 0.0);
        assert_eq!(screen_content_padding(Screen::CarSetup), 0.0);
        assert!(screen_content_padding(Screen::Settings) > 0.0);
    }

    #[test]
    fn only_telemetry_events_with_samples_refresh_the_visible_screen() {
        assert!(telemetry_refreshes_workspace(&LiveTelemetryMessage::Batch(
            LiveTelemetryBatch::default()
        )));
        assert!(!telemetry_refreshes_workspace(
            &LiveTelemetryMessage::Waiting
        ));
        assert!(!telemetry_refreshes_workspace(
            &LiveTelemetryMessage::Connected
        ));
        assert!(!telemetry_refreshes_workspace(
            &LiveTelemetryMessage::Error("test".to_owned())
        ));
    }
}
