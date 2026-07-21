use std::collections::BTreeMap;

use chiaro_actions::{Action, Screen};
use chiaro_config::{DASHBOARD_LAYOUT_SCHEMA_VERSION, DashboardLayoutConfig, DesktopConfig};
use chiaro_dashboard_ui::{
    self as dashboard, DashboardLayout, DashboardLayoutFlag, DashboardMessage, DashboardState,
};
use chiaro_ibt_picker as ibt;
use chiaro_live_telemetry::{self as telemetry, LiveTelemetryMessage, LiveTelemetrySource};
use chiaro_navigation_ui::{self as navigation, Navigation};
use chiaro_settings_ui::{self as settings, SettingsMessage, SettingsState};
use chiaro_telemetry::{LoadedIbt, RecordingSource, Session};
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
#[cfg(target_os = "windows")]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

const CONTENT_PADDING: f32 = 24.0;
const APP_TITLE: &str = "Chiaroscuro";
const COOL_CARBON_DARK: Palette = Palette {
    background: Color::from_rgb8(0x15, 0x17, 0x1B),
    text: Color::from_rgb8(0xF4, 0xF4, 0xF4),
    primary: Color::from_rgb8(0x45, 0x89, 0xFF),
    success: Color::from_rgb8(0x42, 0xBE, 0x65),
    warning: Color::from_rgb8(0xF1, 0xC2, 0x1B),
    danger: Color::from_rgb8(0xFA, 0x4D, 0x56),
};

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
    live_telemetry_source: LiveTelemetrySource,
    configuration: DesktopConfig,
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
                default_text_size: 20.0.into(),
                ..iced::Settings::default()
            })
            .title(Chiaroscuro::title)
            .theme(Chiaroscuro::theme)
            .style(|_, app_theme| surface::application(app_theme))
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
                if let Some(layout) = config
                    .dashboard
                    .as_ref()
                    .and_then(dashboard_layout_from_config)
                {
                    app.dashboard.apply_layout(&layout);
                }
                app.configuration = config;
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
        Theme::custom("Cool Carbon Dark", COOL_CARBON_DARK)
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        let wants_connection = self.session.wants_connection();
        let live_source_available = self.live_telemetry_source.info().is_available();
        let dashboard_active = self.dashboard_is_foreground();
        let telemetry = if wants_connection && live_source_available {
            telemetry::subscription(&self.live_telemetry_source).map(AppMessage::Telemetry)
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
            dashboard::subscription(&self.dashboard, dashboard_active).map(AppMessage::Dashboard),
            telemetry,
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
                window::update(&mut self.window, message, self.tray.is_available())
                    .map(AppMessage::Window)
            },
            AppMessage::Dashboard(message) => self.update_dashboard(message),
            AppMessage::Settings(message) => {
                let persists_configuration = message.persists_configuration();
                let action = settings::update(&mut self.settings, message);
                let action = self.handle_action(action);

                if persists_configuration {
                    self.configuration.show_diagnostics = self.settings.show_diagnostics();
                    self.save_configuration();
                }

                action
            },
            AppMessage::Telemetry(event) => {
                let refreshes_dashboard = telemetry_refreshes_dashboard(&event);
                match event {
                    telemetry::LiveTelemetryMessage::Waiting => self.session.mark_waiting(),
                    telemetry::LiveTelemetryMessage::Connected => self.session.mark_connected(),
                    telemetry::LiveTelemetryMessage::Batch(batch) => {
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
                    telemetry::LiveTelemetryMessage::Snapshot {
                        snapshot,
                        session_info,
                    } => self.session.record_snapshot(snapshot, session_info),
                    telemetry::LiveTelemetryMessage::Error(error) => self.session.mark_error(error),
                }
                if refreshes_dashboard && self.dashboard_is_foreground() {
                    self.update_dashboard(DashboardMessage::Refresh)
                } else {
                    Task::none()
                }
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
            AppMessage::FocusNext => widget::operation::focus_next(),
            AppMessage::FocusPrevious => widget::operation::focus_previous(),
        }
    }

    fn view(&self) -> Element<'_, AppMessage> {
        let rounded = self.window.uses_rounded_corners();
        let current_screen = self.navigation.current();
        let title_bar_tabs = match current_screen {
            Screen::Dashboard => {
                Some(dashboard::tab_bar(&self.dashboard).map(AppMessage::Dashboard))
            },
            Screen::Settings => None,
        };
        let screen = match current_screen {
            Screen::Dashboard => dashboard::view(
                &self.dashboard,
                &self.session,
                self.reference_session.as_ref(),
                self.live_telemetry_source.info(),
            )
            .map(AppMessage::Dashboard),
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

        let navigation = navigation::view(&self.navigation, rounded).map(AppMessage::Navigation);
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
            window::view(
                &self.window,
                navigation_width,
                title_bar_tabs,
                AppMessage::Window,
            ),
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
            Screen::Dashboard => layout.into(),
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
            Action::Navigate(page) => {
                let entering_dashboard =
                    page == Screen::Dashboard && self.navigation.current() != Screen::Dashboard;
                self.navigation.navigate(page);
                if entering_dashboard {
                    self.update_dashboard(DashboardMessage::Refresh)
                } else {
                    Task::none()
                }
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
                if connected && !self.live_telemetry_source.info().is_available() {
                    return Task::none();
                }
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
        self.configuration_error = self
            .configuration
            .save()
            .err()
            .map(|error| format!("failed to save desktop settings: {error}"));
    }

    fn dashboard_is_foreground(&self) -> bool {
        dashboard_is_foreground(self.navigation.current(), self.window.is_backgrounded())
    }

    fn update_dashboard(&mut self, message: DashboardMessage) -> Task<AppMessage> {
        let layout_revision = self.dashboard.layout_revision();
        let resets_layout = message.resets_layout();
        let action = dashboard::update(
            &mut self.dashboard,
            &self.session,
            self.reference_session.as_ref(),
            message,
        );

        let changed_layout = (self.dashboard.layout_revision() != layout_revision)
            .then(|| self.dashboard.layout_snapshot());
        if update_persisted_dashboard_layout(&mut self.configuration, resets_layout, changed_layout)
        {
            self.save_configuration();
        }

        self.handle_action(action)
    }
}

fn dashboard_layout_from_config(config: &DashboardLayoutConfig) -> Option<DashboardLayout> {
    (config.schema_version == DASHBOARD_LAYOUT_SCHEMA_VERSION).then(|| DashboardLayout {
        chart_order: config.chart_order.clone(),
        chart_visibility: layout_flags_from_config(&config.chart_visibility),
        chart_collapsed: layout_flags_from_config(&config.chart_collapsed),
        chart_columns: config.chart_columns,
        setup_card_order: config.setup_card_order.clone(),
        setup_card_collapsed: layout_flags_from_config(&config.setup_card_collapsed),
        lap_analysis_order: config.lap_analysis_order.clone(),
        lap_analysis_collapsed: layout_flags_from_config(&config.lap_analysis_collapsed),
        car_setup_card_order: config.car_setup_card_order.clone(),
        car_setup_card_collapsed: layout_flags_from_config(&config.car_setup_card_collapsed),
    })
}

fn dashboard_layout_to_config(layout: DashboardLayout) -> DashboardLayoutConfig {
    DashboardLayoutConfig {
        schema_version: DASHBOARD_LAYOUT_SCHEMA_VERSION,
        chart_order: layout.chart_order,
        chart_visibility: layout_flags_to_config(layout.chart_visibility),
        chart_collapsed: layout_flags_to_config(layout.chart_collapsed),
        chart_columns: layout.chart_columns,
        setup_card_order: layout.setup_card_order,
        setup_card_collapsed: layout_flags_to_config(layout.setup_card_collapsed),
        lap_analysis_order: layout.lap_analysis_order,
        lap_analysis_collapsed: layout_flags_to_config(layout.lap_analysis_collapsed),
        car_setup_card_order: layout.car_setup_card_order,
        car_setup_card_collapsed: layout_flags_to_config(layout.car_setup_card_collapsed),
    }
}

fn layout_flags_from_config(flags: &BTreeMap<String, bool>) -> Vec<DashboardLayoutFlag> {
    flags
        .iter()
        .map(|(key, value)| DashboardLayoutFlag {
            key: key.clone(),
            value: *value,
        })
        .collect()
}

fn layout_flags_to_config(flags: Vec<DashboardLayoutFlag>) -> BTreeMap<String, bool> {
    flags
        .into_iter()
        .map(|flag| (flag.key, flag.value))
        .collect()
}

fn update_persisted_dashboard_layout(
    configuration: &mut DesktopConfig,
    resets_layout: bool,
    changed_layout: Option<DashboardLayout>,
) -> bool {
    if resets_layout {
        let removed_override = configuration.dashboard.take().is_some();
        return removed_override || changed_layout.is_some();
    }
    let Some(layout) = changed_layout else {
        return false;
    };

    configuration.dashboard = Some(dashboard_layout_to_config(layout));
    true
}

fn dashboard_is_foreground(screen: Screen, window_is_backgrounded: bool) -> bool {
    screen == Screen::Dashboard && !window_is_backgrounded
}

fn screen_content_padding(screen: Screen) -> f32 {
    match screen {
        Screen::Dashboard => 0.0,
        Screen::Settings => CONTENT_PADDING,
    }
}

fn telemetry_refreshes_dashboard(event: &LiveTelemetryMessage) -> bool {
    matches!(
        event,
        LiveTelemetryMessage::Batch(_) | LiveTelemetryMessage::Snapshot { .. }
    )
}

#[cfg(test)]
mod tests {
    use chiaro_actions::Screen;
    use chiaro_config::DesktopConfig;
    use chiaro_dashboard_ui::{DashboardLayout, DashboardLayoutFlag};
    use chiaro_live_telemetry::{LiveTelemetryBatch, LiveTelemetryMessage};

    use super::{
        dashboard_is_foreground, dashboard_layout_from_config, dashboard_layout_to_config,
        screen_content_padding, telemetry_refreshes_dashboard, update_persisted_dashboard_layout,
    };

    #[test]
    fn dashboard_layout_survives_the_configuration_boundary() {
        let layout = DashboardLayout {
            car_setup_card_order: vec![
                "summary".to_owned(),
                "vehicle:specifications".to_owned(),
                "setup:tires".to_owned(),
            ],
            car_setup_card_collapsed: vec![DashboardLayoutFlag {
                key: "setup:tires".to_owned(),
                value: true,
            }],
            ..DashboardLayout::default()
        };
        let config = dashboard_layout_to_config(layout.clone());
        let restored = dashboard_layout_from_config(&config).expect("known layout schema");

        assert_eq!(restored.car_setup_card_order, layout.car_setup_card_order);
        assert_eq!(
            restored.car_setup_card_collapsed,
            layout.car_setup_card_collapsed
        );
        assert_eq!(dashboard_layout_to_config(restored), config);
    }

    #[test]
    fn ignores_dashboard_layouts_from_an_unknown_schema() {
        let mut config = dashboard_layout_to_config(DashboardLayout::default());
        config.schema_version += 1;

        assert_eq!(dashboard_layout_from_config(&config), None);
    }

    #[test]
    fn dashboard_configuration_is_written_only_for_real_layout_changes() {
        let mut configuration = DesktopConfig::default();

        assert!(!update_persisted_dashboard_layout(
            &mut configuration,
            false,
            None,
        ));
        assert_eq!(configuration.dashboard, None);

        assert!(update_persisted_dashboard_layout(
            &mut configuration,
            false,
            Some(DashboardLayout::default()),
        ));
        assert!(configuration.dashboard.is_some());
    }

    #[test]
    fn resetting_layout_removes_an_explicit_dashboard_override() {
        let mut configuration = DesktopConfig {
            dashboard: Some(dashboard_layout_to_config(DashboardLayout::default())),
            ..DesktopConfig::default()
        };

        assert!(update_persisted_dashboard_layout(
            &mut configuration,
            true,
            None,
        ));
        assert_eq!(configuration.dashboard, None);
        assert!(update_persisted_dashboard_layout(
            &mut configuration,
            true,
            Some(DashboardLayout::default()),
        ));
    }

    #[test]
    fn dashboard_refreshes_only_while_it_is_foreground() {
        assert!(dashboard_is_foreground(Screen::Dashboard, false));
        assert!(!dashboard_is_foreground(Screen::Dashboard, true));
        assert!(!dashboard_is_foreground(Screen::Settings, false));
    }

    #[test]
    fn dashboard_owns_the_padding_beneath_the_integrated_title_bar_tabs() {
        assert_eq!(screen_content_padding(Screen::Dashboard), 0.0);
        assert!(screen_content_padding(Screen::Settings) > 0.0);
    }

    #[test]
    fn only_telemetry_events_with_samples_request_a_dashboard_refresh() {
        assert!(telemetry_refreshes_dashboard(&LiveTelemetryMessage::Batch(
            LiveTelemetryBatch::default()
        )));
        assert!(!telemetry_refreshes_dashboard(
            &LiveTelemetryMessage::Waiting
        ));
        assert!(!telemetry_refreshes_dashboard(
            &LiveTelemetryMessage::Connected
        ));
        assert!(!telemetry_refreshes_dashboard(
            &LiveTelemetryMessage::Error("test".to_owned())
        ));
    }
}
