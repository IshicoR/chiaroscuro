mod analysis_card;
mod bounds_reporter;
mod chart_card;
mod lap_choice;
mod setup_view;
mod tyre_status;

use std::collections::BTreeMap;

use analysis_card::{pane_card, pane_card_with_maximize};
use bounds_reporter::bounds_reporter;
use chart_card::{CARD_HEADER_HEIGHT, CardTitle, chart_card};
use chiaro_actions::Action;
use chiaro_irsdk::{SessionInfoDocument, SessionScalar, TelemetrySample, variables};
use chiaro_telemetry::{
    ConnectionStatus, FocusedTelemetry, HISTORY_WINDOW, LAP_DISTANCE_AXIS_MAX,
    LiveTelemetrySourceInfo, Session,
};
use chiaro_time_series_chart::{
    AxisSpec, LineSeries, TimeSeriesChart, TimeSeriesMessage, TimeSeriesSpec,
};
use chiaro_widgets::{
    BadgeVariant, ButtonSize, ButtonVariant, badge, button as action_button, callout,
    checkbox_style, icon_button, icon_toggle_button, icon_tooltip_style, tab, tabs, typography,
};
use iced::{
    Color, Element, Length, Padding, Point, Rectangle, Subscription, Vector,
    alignment::{Horizontal, Vertical},
    keyboard, mouse,
    widget::{
        Space, checkbox, column, container, float, grid, mouse_area, row, rule, scrollable, stack,
        text, tooltip,
    },
};
use iced_fonts::lucide;
use iced_plot::{AxisLink, LineStyle};
use lap_choice::{LapChoice, format_lap_time, lap_choice_list};

const CHART_HEIGHT: f32 = 360.0;
const DASHBOARD_CONTENT_PADDING: f32 = 24.0;
const DASHBOARD_TAB_WIDTH: f32 = 120.0;
const DASHBOARD_TAB_TOP_PADDING: f32 =
    chiaro_widgets::tabs::BAR_HEIGHT - chiaro_widgets::tabs::HEIGHT;
const DASHBOARD_TAB_LEFT_PADDING: f32 = DASHBOARD_CONTENT_PADDING;
const DASHBOARD_TAB_ICON_SIZE: u32 = 14;
const PRIMARY_CHART_LINE_WIDTH: f32 = 1.8;
const DYNAMICS_CHART_LINE_WIDTH: f32 = 1.6;
const WHEEL_CHART_LINE_WIDTH: f32 = 1.4;
const BRAKE_PRESSURE_CHART_LINE_WIDTH: f32 = 1.2;
const REFERENCE_CHART_LINE_WIDTH: f32 = 1.1;
const SETUP_PANEL_WIDTH: f32 = 248.0;
const ANALYSIS_PANEL_WIDTH: f32 = 280.0;
const INPUT_BADGE_WIDTH: f32 = 72.0;
const DATA_ROW_HEIGHT: f32 = 34.0;
const SESSION_METADATA_ROW_HEIGHT: f32 = 42.0;
const DATA_SEPARATOR_WIDTH: f32 = 1.0;
const DATA_TEXT_INSET: f32 = 8.0;
const CARD_TITLE_ICON_SIZE: f32 = 16.0;
// Carbon Gray 100 uses the same semantic palette everywhere outside of the
// charts. Chart colors stay independent: they identify telemetry signals.
const STATUS_INFO: Color = Color::from_rgb(0.27, 0.54, 1.0);
const STATUS_MUTED: Color = Color::from_rgb(0.55, 0.55, 0.55);
const STATUS_WARNING: Color = Color::from_rgb(0.95, 0.76, 0.11);
const STATUS_SUCCESS: Color = Color::from_rgb(0.26, 0.75, 0.40);
const STATUS_ERROR: Color = Color::from_rgb(0.98, 0.30, 0.34);
const TEXT_SECONDARY: Color = Color::from_rgb(0.78, 0.78, 0.78);
const REFERENCE_AMBER: Color = Color::from_rgb(0.96, 0.76, 0.28);
const REFERENCE_CYAN: Color = Color::from_rgb(0.30, 0.78, 0.88);
const REFERENCE_PINK: Color = Color::from_rgb(0.94, 0.42, 0.66);
const REFERENCE_LAVENDER: Color = Color::from_rgb(0.69, 0.56, 0.94);
const THROTTLE_LINE_COLOR: Color = Color::from_rgb(0.12, 0.72, 0.38);
const BRAKE_LINE_COLOR: Color = Color::from_rgb(0.90, 0.24, 0.24);
const STEERING_LINE_COLOR: Color = Color::from_rgb(0.20, 0.72, 0.68);
const STEERING_METER_HALF_RANGE_RADIANS: f32 = std::f32::consts::PI;
const CAR_SETUP_CARD_SPACING: f32 = 12.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DashboardTab {
    #[default]
    Telemetry,
    CarSetup,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChartColumns {
    #[default]
    One,
    Two,
}

impl ChartColumns {
    const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }

    const fn persisted_value(self) -> u8 {
        self.count() as u8
    }

    const fn from_persisted_value(value: u8) -> Self {
        match value {
            2 => Self::Two,
            _ => Self::One,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartId {
    Speed,
    Pedal,
    BrakePressure,
    Abs,
    Steering,
    SteeringTorque,
    Rpm,
    Gear,
    Dynamics,
    Yaw,
    WheelSlip,
    Tyre,
    Suspension,
    Fuel,
    Delta,
}

impl ChartId {
    const ALL: [Self; 15] = [
        Self::Speed,
        Self::Pedal,
        Self::BrakePressure,
        Self::Abs,
        Self::Steering,
        Self::SteeringTorque,
        Self::Rpm,
        Self::Gear,
        Self::Dynamics,
        Self::Yaw,
        Self::WheelSlip,
        Self::Tyre,
        Self::Suspension,
        Self::Fuel,
        Self::Delta,
    ];
    const COUNT: usize = Self::ALL.len();
    const DEFAULT_ORDER: [Self; Self::COUNT] = [
        Self::Speed,
        Self::Delta,
        Self::Pedal,
        Self::BrakePressure,
        Self::Abs,
        Self::Steering,
        Self::SteeringTorque,
        Self::Gear,
        Self::Rpm,
        Self::Dynamics,
        Self::Yaw,
        Self::WheelSlip,
        Self::Suspension,
        Self::Tyre,
        Self::Fuel,
    ];
    const DEFAULT_VISIBLE: [Self; 3] = [Self::Speed, Self::Pedal, Self::Steering];

    const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Speed => "speed",
            Self::Pedal => "pedal",
            Self::BrakePressure => "brake_pressure",
            Self::Abs => "abs",
            Self::Steering => "steering",
            Self::SteeringTorque => "steering_torque",
            Self::Rpm => "rpm",
            Self::Gear => "gear",
            Self::Dynamics => "dynamics",
            Self::Yaw => "yaw",
            Self::WheelSlip => "wheel_slip",
            Self::Tyre => "tyre",
            Self::Suspension => "suspension",
            Self::Fuel => "fuel",
            Self::Delta => "delta",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|chart| chart.key() == key)
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Speed => "Speed",
            Self::Pedal => "Pedal",
            Self::BrakePressure => "Brake pressure (IBT)",
            Self::Abs => "ABS activity",
            Self::Steering => "Steering",
            Self::SteeringTorque => "Steering torque",
            Self::Rpm => "Engine RPM",
            Self::Gear => "Gear",
            Self::Dynamics => "Vehicle dynamics",
            Self::Yaw => "Yaw rate",
            Self::WheelSlip => "Wheel slip",
            Self::Tyre => "Tyre temperature",
            Self::Suspension => "Suspension travel",
            Self::Fuel => "Fuel used",
            Self::Delta => "Delta",
        }
    }

    fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Speed => lucide::gauge(),
            Self::Pedal => lucide::sliders_vertical(),
            Self::BrakePressure => lucide::disc_three(),
            Self::Abs => lucide::shield_check(),
            Self::Steering => lucide::ship_wheel(),
            Self::SteeringTorque => lucide::rotate_cw(),
            Self::Rpm => lucide::circle_gauge(),
            Self::Gear => lucide::cog(),
            Self::Dynamics => lucide::activity(),
            Self::Yaw => lucide::rotate_cw(),
            Self::WheelSlip => lucide::circle_dot_dashed(),
            Self::Tyre => lucide::thermometer(),
            Self::Suspension => lucide::move_vertical(),
            Self::Fuel => lucide::fuel(),
            Self::Delta => lucide::diff(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LapAnalysisCardId {
    Cursor,
    ReferenceCursor,
    Vehicle,
    Inputs,
    Dynamics,
    Tyres,
    Wheels,
}

impl LapAnalysisCardId {
    const ALL: [Self; 7] = [
        Self::Cursor,
        Self::ReferenceCursor,
        Self::Vehicle,
        Self::Inputs,
        Self::Dynamics,
        Self::Tyres,
        Self::Wheels,
    ];
    const COUNT: usize = Self::ALL.len();

    const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::ReferenceCursor => "reference_cursor",
            Self::Vehicle => "vehicle",
            Self::Inputs => "inputs",
            Self::Dynamics => "dynamics",
            Self::Tyres => "tyres",
            Self::Wheels => "wheels",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|card| card.key() == key)
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::ReferenceCursor => "Reference cursor",
            Self::Vehicle => "Vehicle",
            Self::Inputs => "Inputs",
            Self::Dynamics => "Dynamics",
            Self::Tyres => "Tyres",
            Self::Wheels => "Wheels",
        }
    }

    fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Cursor => lucide::crosshair(),
            Self::ReferenceCursor => lucide::locate_fixed(),
            Self::Vehicle => lucide::car_front(),
            Self::Inputs => lucide::sliders_horizontal(),
            Self::Dynamics => lucide::activity(),
            Self::Tyres => lucide::circle_dashed(),
            Self::Wheels => lucide::disc_three(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCardId {
    Session,
    Reference,
    Laps,
    Charts,
}

impl SetupCardId {
    const ALL: [Self; 4] = [Self::Session, Self::Reference, Self::Laps, Self::Charts];
    const COUNT: usize = Self::ALL.len();

    const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Reference => "reference",
            Self::Laps => "laps",
            Self::Charts => "charts",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|card| card.key() == key)
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Reference => "Reference",
            Self::Laps => "My laps",
            Self::Charts => "Charts",
        }
    }

    fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Session => lucide::clipboard_list(),
            Self::Reference => lucide::target(),
            Self::Laps => lucide::timer(),
            Self::Charts => lucide::chart_line(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

/// A stable, UI-independent boolean value in a persisted dashboard layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardLayoutFlag {
    pub key: String,
    pub value: bool,
}

/// The user-customizable parts of the telemetry dashboard layout.
///
/// Keys are stable across display-name changes. Unknown, duplicate, or missing
/// keys are normalized by [`DashboardState::apply_layout`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardLayout {
    pub chart_order: Vec<String>,
    pub chart_visibility: Vec<DashboardLayoutFlag>,
    pub chart_collapsed: Vec<DashboardLayoutFlag>,
    pub chart_columns: u8,
    pub setup_card_order: Vec<String>,
    pub setup_card_collapsed: Vec<DashboardLayoutFlag>,
    pub lap_analysis_order: Vec<String>,
    pub lap_analysis_collapsed: Vec<DashboardLayoutFlag>,
    pub car_setup_card_order: Vec<String>,
    pub car_setup_card_collapsed: Vec<DashboardLayoutFlag>,
}

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            chart_order: ChartId::DEFAULT_ORDER
                .map(|chart| chart.key().to_owned())
                .to_vec(),
            chart_visibility: ChartId::ALL
                .map(|chart| DashboardLayoutFlag {
                    key: chart.key().to_owned(),
                    value: ChartId::DEFAULT_VISIBLE.contains(&chart),
                })
                .to_vec(),
            chart_collapsed: ChartId::ALL
                .map(|chart| DashboardLayoutFlag {
                    key: chart.key().to_owned(),
                    value: false,
                })
                .to_vec(),
            chart_columns: ChartColumns::One.persisted_value(),
            setup_card_order: SetupCardId::ALL.map(|card| card.key().to_owned()).to_vec(),
            setup_card_collapsed: SetupCardId::ALL
                .map(|card| DashboardLayoutFlag {
                    key: card.key().to_owned(),
                    value: false,
                })
                .to_vec(),
            lap_analysis_order: LapAnalysisCardId::ALL
                .map(|card| card.key().to_owned())
                .to_vec(),
            lap_analysis_collapsed: LapAnalysisCardId::ALL
                .map(|card| DashboardLayoutFlag {
                    key: card.key().to_owned(),
                    value: false,
                })
                .to_vec(),
            car_setup_card_order: Vec::new(),
            car_setup_card_collapsed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum IbtLoadState {
    #[default]
    Idle,
    Selecting,
    Loading,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CardLayout {
    bounds: Rectangle,
    visible_bounds: Option<Rectangle>,
}

#[derive(Debug, Clone, Default)]
struct SessionMetadata {
    track_name: Option<String>,
    track_config: Option<String>,
    track_length: Option<String>,
    track_turns: Option<i32>,
    track_type: Option<String>,
    car_name: Option<String>,
    car_class: Option<String>,
    session_type: Option<String>,
    session_time: Option<String>,
    date_time: Option<String>,
    weather: Option<String>,
    air_temperature: Option<String>,
    surface_temperature: Option<String>,
    humidity: Option<String>,
    wind: Option<String>,
}

#[derive(Debug)]
pub struct DashboardState {
    active_tab: DashboardTab,
    speed_chart: TimeSeriesChart,
    pedal_chart: TimeSeriesChart,
    brake_pressure_chart: TimeSeriesChart,
    abs_chart: TimeSeriesChart,
    steering_chart: TimeSeriesChart,
    steering_torque_chart: TimeSeriesChart,
    rpm_chart: TimeSeriesChart,
    gear_chart: TimeSeriesChart,
    dynamics_chart: TimeSeriesChart,
    yaw_chart: TimeSeriesChart,
    wheel_slip_chart: TimeSeriesChart,
    tyre_chart: TimeSeriesChart,
    suspension_chart: TimeSeriesChart,
    fuel_chart: TimeSeriesChart,
    delta_chart: TimeSeriesChart,
    rendered_packets: u64,
    chart_packet_cursors: [Option<u64>; ChartId::COUNT],
    ibt_load_state: IbtLoadState,
    reference_ibt_load_state: IbtLoadState,
    reference_ibt_error: Option<String>,
    lap_choices: Vec<LapChoice>,
    selected_lap_index: Option<usize>,
    reference_lap_choices: Vec<LapChoice>,
    selected_reference_lap_index: Option<usize>,
    cached_session_info_revision: Option<u64>,
    session_metadata: SessionMetadata,
    car_setup: setup_view::SetupViewData,
    focus_x: Option<f64>,
    focused: Option<FocusedTelemetry>,
    focus_from_cursor: bool,
    live_follow: bool,
    layout_revision: u64,
    chart_order: Vec<ChartId>,
    chart_visibility: [bool; ChartId::COUNT],
    chart_collapsed: [bool; ChartId::COUNT],
    chart_layouts: [Option<CardLayout>; ChartId::COUNT],
    chart_layout_generation: u64,
    chart_list_layouts: [Option<CardLayout>; ChartId::COUNT],
    chart_list_layout_generation: u64,
    chart_columns: ChartColumns,
    maximized_chart: Option<ChartId>,
    dragging_chart: Option<ChartId>,
    drop_target: Option<ChartId>,
    drag_origin: Option<Point>,
    drag_cursor: Option<Point>,
    drag_source_bounds: Option<Rectangle>,
    dragging_chart_list_item: Option<ChartId>,
    chart_list_drop_target: Option<ChartId>,
    chart_list_drag_origin: Option<Point>,
    chart_list_drag_cursor: Option<Point>,
    chart_list_drag_source_bounds: Option<Rectangle>,
    lap_analysis_order: Vec<LapAnalysisCardId>,
    lap_analysis_collapsed: [bool; LapAnalysisCardId::COUNT],
    lap_analysis_layouts: [Option<CardLayout>; LapAnalysisCardId::COUNT],
    dragging_lap_analysis_card: Option<LapAnalysisCardId>,
    lap_analysis_drop_target: Option<LapAnalysisCardId>,
    lap_analysis_drag_origin: Option<Point>,
    lap_analysis_drag_cursor: Option<Point>,
    lap_analysis_drag_source_bounds: Option<Rectangle>,
    setup_card_order: Vec<SetupCardId>,
    setup_card_collapsed: [bool; SetupCardId::COUNT],
    setup_card_layouts: [Option<CardLayout>; SetupCardId::COUNT],
    dragging_setup_card: Option<SetupCardId>,
    setup_card_drop_target: Option<SetupCardId>,
    setup_card_drag_origin: Option<Point>,
    setup_card_drag_cursor: Option<Point>,
    setup_card_drag_source_bounds: Option<Rectangle>,
    car_setup_card_order: Vec<String>,
    car_setup_card_collapsed: BTreeMap<String, bool>,
    car_setup_card_layouts: BTreeMap<String, CardLayout>,
    maximized_car_setup_card: Option<String>,
    dragging_car_setup_card: Option<String>,
    car_setup_drop_target: Option<String>,
    car_setup_drag_origin: Option<Point>,
    car_setup_drag_cursor: Option<Point>,
    car_setup_drag_source_bounds: Option<Rectangle>,
    modifiers: keyboard::Modifiers,
}

impl Default for DashboardState {
    fn default() -> Self {
        let x_axis_link = AxisLink::new();
        Self {
            active_tab: DashboardTab::Telemetry,
            speed_chart: build_speed_chart(x_axis_link.clone()),
            pedal_chart: build_pedal_chart(x_axis_link.clone()),
            brake_pressure_chart: build_brake_pressure_chart(x_axis_link.clone()),
            abs_chart: build_abs_chart(x_axis_link.clone()),
            steering_chart: build_steering_chart(x_axis_link.clone()),
            steering_torque_chart: build_steering_torque_chart(x_axis_link.clone()),
            rpm_chart: build_rpm_chart(x_axis_link.clone()),
            gear_chart: build_gear_chart(x_axis_link.clone()),
            dynamics_chart: build_dynamics_chart(x_axis_link.clone()),
            yaw_chart: build_yaw_chart(x_axis_link.clone()),
            wheel_slip_chart: build_wheel_slip_chart(x_axis_link.clone()),
            tyre_chart: build_tyre_chart(x_axis_link.clone()),
            suspension_chart: build_suspension_chart(x_axis_link.clone()),
            fuel_chart: build_fuel_chart(x_axis_link.clone()),
            delta_chart: build_delta_chart(x_axis_link),
            rendered_packets: 0,
            chart_packet_cursors: [None; ChartId::COUNT],
            ibt_load_state: IbtLoadState::Idle,
            reference_ibt_load_state: IbtLoadState::Idle,
            reference_ibt_error: None,
            lap_choices: Vec::new(),
            selected_lap_index: None,
            reference_lap_choices: Vec::new(),
            selected_reference_lap_index: None,
            cached_session_info_revision: None,
            session_metadata: SessionMetadata::default(),
            car_setup: setup_view::SetupViewData::default(),
            focus_x: None,
            focused: None,
            focus_from_cursor: false,
            live_follow: true,
            layout_revision: 0,
            chart_order: ChartId::DEFAULT_ORDER.to_vec(),
            chart_visibility: ChartId::ALL.map(|chart| ChartId::DEFAULT_VISIBLE.contains(&chart)),
            chart_collapsed: [false; ChartId::COUNT],
            chart_layouts: [None; ChartId::COUNT],
            chart_layout_generation: 0,
            chart_list_layouts: [None; ChartId::COUNT],
            chart_list_layout_generation: 0,
            chart_columns: ChartColumns::One,
            maximized_chart: None,
            dragging_chart: None,
            drop_target: None,
            drag_origin: None,
            drag_cursor: None,
            drag_source_bounds: None,
            dragging_chart_list_item: None,
            chart_list_drop_target: None,
            chart_list_drag_origin: None,
            chart_list_drag_cursor: None,
            chart_list_drag_source_bounds: None,
            lap_analysis_order: LapAnalysisCardId::ALL.to_vec(),
            lap_analysis_collapsed: [false; LapAnalysisCardId::COUNT],
            lap_analysis_layouts: [None; LapAnalysisCardId::COUNT],
            dragging_lap_analysis_card: None,
            lap_analysis_drop_target: None,
            lap_analysis_drag_origin: None,
            lap_analysis_drag_cursor: None,
            lap_analysis_drag_source_bounds: None,
            setup_card_order: SetupCardId::ALL.to_vec(),
            setup_card_collapsed: [false; SetupCardId::COUNT],
            setup_card_layouts: [None; SetupCardId::COUNT],
            dragging_setup_card: None,
            setup_card_drop_target: None,
            setup_card_drag_origin: None,
            setup_card_drag_cursor: None,
            setup_card_drag_source_bounds: None,
            car_setup_card_order: Vec::new(),
            car_setup_card_collapsed: BTreeMap::new(),
            car_setup_card_layouts: BTreeMap::new(),
            maximized_car_setup_card: None,
            dragging_car_setup_card: None,
            car_setup_drop_target: None,
            car_setup_drag_origin: None,
            car_setup_drag_cursor: None,
            car_setup_drag_source_bounds: None,
            modifiers: keyboard::Modifiers::NONE,
        }
    }
}

impl DashboardState {
    pub const fn active_tab(&self) -> DashboardTab {
        self.active_tab
    }

    pub const fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    pub fn layout_snapshot(&self) -> DashboardLayout {
        DashboardLayout {
            chart_order: self
                .chart_order
                .iter()
                .map(|chart| chart.key().to_owned())
                .collect(),
            chart_visibility: ChartId::ALL
                .map(|chart| DashboardLayoutFlag {
                    key: chart.key().to_owned(),
                    value: self.chart_visibility[chart.index()],
                })
                .to_vec(),
            chart_collapsed: ChartId::ALL
                .map(|chart| DashboardLayoutFlag {
                    key: chart.key().to_owned(),
                    value: self.chart_collapsed[chart.index()],
                })
                .to_vec(),
            chart_columns: self.chart_columns.persisted_value(),
            setup_card_order: self
                .setup_card_order
                .iter()
                .map(|card| card.key().to_owned())
                .collect(),
            setup_card_collapsed: SetupCardId::ALL
                .map(|card| DashboardLayoutFlag {
                    key: card.key().to_owned(),
                    value: self.setup_card_collapsed[card.index()],
                })
                .to_vec(),
            lap_analysis_order: self
                .lap_analysis_order
                .iter()
                .map(|card| card.key().to_owned())
                .collect(),
            lap_analysis_collapsed: LapAnalysisCardId::ALL
                .map(|card| DashboardLayoutFlag {
                    key: card.key().to_owned(),
                    value: self.lap_analysis_collapsed[card.index()],
                })
                .to_vec(),
            car_setup_card_order: self.car_setup_card_order.clone(),
            car_setup_card_collapsed: self
                .car_setup_card_collapsed
                .iter()
                .map(|(key, value)| DashboardLayoutFlag {
                    key: key.clone(),
                    value: *value,
                })
                .collect(),
        }
    }

    /// Restores a persisted layout without marking it as a user edit.
    pub fn apply_layout(&mut self, layout: &DashboardLayout) {
        self.chart_order = normalize_layout_order(
            &layout.chart_order,
            ChartId::DEFAULT_ORDER,
            ChartId::from_key,
        );
        self.chart_visibility = normalize_layout_flags(
            &layout.chart_visibility,
            ChartId::ALL.map(|chart| ChartId::DEFAULT_VISIBLE.contains(&chart)),
            ChartId::from_key,
            ChartId::index,
        );
        self.chart_collapsed = normalize_layout_flags(
            &layout.chart_collapsed,
            [false; ChartId::COUNT],
            ChartId::from_key,
            ChartId::index,
        );
        self.chart_columns = ChartColumns::from_persisted_value(layout.chart_columns);
        self.setup_card_order = normalize_layout_order(
            &layout.setup_card_order,
            SetupCardId::ALL,
            SetupCardId::from_key,
        );
        self.setup_card_collapsed = normalize_layout_flags(
            &layout.setup_card_collapsed,
            [false; SetupCardId::COUNT],
            SetupCardId::from_key,
            SetupCardId::index,
        );
        self.lap_analysis_order = normalize_layout_order(
            &layout.lap_analysis_order,
            LapAnalysisCardId::ALL,
            LapAnalysisCardId::from_key,
        );
        self.lap_analysis_collapsed = normalize_layout_flags(
            &layout.lap_analysis_collapsed,
            [false; LapAnalysisCardId::COUNT],
            LapAnalysisCardId::from_key,
            LapAnalysisCardId::index,
        );
        self.car_setup_card_order = normalize_dynamic_order(&layout.car_setup_card_order);
        self.car_setup_card_collapsed = normalize_dynamic_flags(&layout.car_setup_card_collapsed);

        self.maximized_chart = None;
        self.maximized_car_setup_card = None;
        self.clear_chart_drag();
        self.clear_chart_list_drag();
        self.clear_lap_analysis_drag();
        self.clear_setup_card_drag();
        self.clear_car_setup_card_drag();
        self.cancel_chart_interactions();
        self.invalidate_chart_layouts();
        self.invalidate_chart_list_layouts();
        self.lap_analysis_layouts = [None; LapAnalysisCardId::COUNT];
        self.setup_card_layouts = [None; SetupCardId::COUNT];
        self.car_setup_card_layouts.clear();
    }

    fn mark_layout_changed(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
    }

    pub fn begin_ibt_selection(&mut self) {
        self.ibt_load_state = IbtLoadState::Selecting;
    }

    pub fn begin_ibt_load(&mut self) {
        self.ibt_load_state = IbtLoadState::Loading;
    }

    pub fn finish_ibt_load(&mut self) {
        self.ibt_load_state = IbtLoadState::Idle;
    }

    pub fn begin_reference_ibt_selection(&mut self) {
        self.reference_ibt_load_state = IbtLoadState::Selecting;
        self.reference_ibt_error = None;
    }

    pub fn begin_reference_ibt_load(&mut self) {
        self.reference_ibt_load_state = IbtLoadState::Loading;
        self.reference_ibt_error = None;
    }

    pub fn finish_reference_ibt_load(&mut self) {
        self.reference_ibt_load_state = IbtLoadState::Idle;
    }

    pub fn mark_reference_ibt_error(&mut self, error: String) {
        self.reference_ibt_error = Some(error);
    }

    fn clear_chart_drag(&mut self) {
        self.dragging_chart = None;
        self.drop_target = None;
        self.drag_origin = None;
        self.drag_cursor = None;
        self.drag_source_bounds = None;
    }

    fn clear_chart_list_drag(&mut self) {
        self.dragging_chart_list_item = None;
        self.chart_list_drop_target = None;
        self.chart_list_drag_origin = None;
        self.chart_list_drag_cursor = None;
        self.chart_list_drag_source_bounds = None;
    }

    fn chart_mut(&mut self, chart: ChartId) -> &mut TimeSeriesChart {
        match chart {
            ChartId::Speed => &mut self.speed_chart,
            ChartId::Pedal => &mut self.pedal_chart,
            ChartId::BrakePressure => &mut self.brake_pressure_chart,
            ChartId::Abs => &mut self.abs_chart,
            ChartId::Steering => &mut self.steering_chart,
            ChartId::SteeringTorque => &mut self.steering_torque_chart,
            ChartId::Rpm => &mut self.rpm_chart,
            ChartId::Gear => &mut self.gear_chart,
            ChartId::Dynamics => &mut self.dynamics_chart,
            ChartId::Yaw => &mut self.yaw_chart,
            ChartId::WheelSlip => &mut self.wheel_slip_chart,
            ChartId::Tyre => &mut self.tyre_chart,
            ChartId::Suspension => &mut self.suspension_chart,
            ChartId::Fuel => &mut self.fuel_chart,
            ChartId::Delta => &mut self.delta_chart,
        }
    }

    fn invalidate_chart_layouts(&mut self) {
        self.chart_layouts = [None; ChartId::COUNT];
        self.chart_layout_generation = self.chart_layout_generation.wrapping_add(1);
    }

    fn invalidate_chart_list_layouts(&mut self) {
        self.chart_list_layouts = [None; ChartId::COUNT];
        self.chart_list_layout_generation = self.chart_list_layout_generation.wrapping_add(1);
    }

    fn clear_lap_analysis_drag(&mut self) {
        self.dragging_lap_analysis_card = None;
        self.lap_analysis_drop_target = None;
        self.lap_analysis_drag_origin = None;
        self.lap_analysis_drag_cursor = None;
        self.lap_analysis_drag_source_bounds = None;
    }

    fn clear_setup_card_drag(&mut self) {
        self.dragging_setup_card = None;
        self.setup_card_drop_target = None;
        self.setup_card_drag_origin = None;
        self.setup_card_drag_cursor = None;
        self.setup_card_drag_source_bounds = None;
    }

    fn clear_car_setup_card_drag(&mut self) {
        self.dragging_car_setup_card = None;
        self.car_setup_drop_target = None;
        self.car_setup_drag_origin = None;
        self.car_setup_drag_cursor = None;
        self.car_setup_drag_source_bounds = None;
    }

    fn reconcile_car_setup_cards(&mut self) {
        let available = self.car_setup.card_keys();
        for key in &available {
            if !self.car_setup_card_order.contains(key) {
                self.car_setup_card_order.push(key.clone());
            }
        }
        if self
            .maximized_car_setup_card
            .as_ref()
            .is_some_and(|key| !available.contains(key))
        {
            self.maximized_car_setup_card = None;
        }
        if self
            .dragging_car_setup_card
            .as_ref()
            .is_some_and(|key| !available.contains(key))
        {
            self.clear_car_setup_card_drag();
        }
        self.car_setup_card_layouts.clear();
    }

    fn is_dragging_card(&self) -> bool {
        self.dragging_chart.is_some()
            || self.dragging_chart_list_item.is_some()
            || self.dragging_lap_analysis_card.is_some()
            || self.dragging_setup_card.is_some()
            || self.dragging_car_setup_card.is_some()
    }

    fn cancel_chart_interactions(&mut self) {
        for chart in [
            &mut self.speed_chart,
            &mut self.pedal_chart,
            &mut self.brake_pressure_chart,
            &mut self.abs_chart,
            &mut self.steering_chart,
            &mut self.steering_torque_chart,
            &mut self.rpm_chart,
            &mut self.gear_chart,
            &mut self.dynamics_chart,
            &mut self.yaw_chart,
            &mut self.wheel_slip_chart,
            &mut self.tyre_chart,
            &mut self.suspension_chart,
            &mut self.fuel_chart,
            &mut self.delta_chart,
        ] {
            chart.cancel_interaction();
        }
    }
}

fn normalize_layout_order<Id: Copy + Eq, const N: usize>(
    keys: &[String],
    default_order: [Id; N],
    from_key: impl Fn(&str) -> Option<Id>,
) -> Vec<Id> {
    let mut normalized = Vec::with_capacity(N);
    for key in keys {
        let Some(id) = from_key(key) else {
            continue;
        };
        if !normalized.contains(&id) {
            normalized.push(id);
        }
    }
    for id in default_order {
        if !normalized.contains(&id) {
            normalized.push(id);
        }
    }
    normalized
}

fn normalize_layout_flags<Id: Copy, const N: usize>(
    flags: &[DashboardLayoutFlag],
    mut normalized: [bool; N],
    from_key: impl Fn(&str) -> Option<Id>,
    index: impl Fn(Id) -> usize,
) -> [bool; N] {
    let mut seen = [false; N];
    for flag in flags {
        let Some(id) = from_key(&flag.key) else {
            continue;
        };
        let index = index(id);
        if !seen[index] {
            normalized[index] = flag.value;
            seen[index] = true;
        }
    }
    normalized
}

fn normalize_dynamic_order(keys: &[String]) -> Vec<String> {
    let mut normalized = Vec::with_capacity(keys.len());
    for key in keys {
        if !key.trim().is_empty() && !normalized.contains(key) {
            normalized.push(key.clone());
        }
    }
    normalized
}

fn normalize_dynamic_flags(flags: &[DashboardLayoutFlag]) -> BTreeMap<String, bool> {
    let mut normalized = BTreeMap::new();
    for flag in flags {
        if !flag.key.trim().is_empty() {
            normalized.entry(flag.key.clone()).or_insert(flag.value);
        }
    }
    normalized
}

fn current_car_setup_card_order(state: &DashboardState) -> Vec<String> {
    let available = state.car_setup.card_keys();
    let mut order = state
        .car_setup_card_order
        .iter()
        .filter(|key| available.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    for key in available {
        if !order.contains(&key) {
            order.push(key);
        }
    }
    order
}

fn merge_current_car_setup_order(state: &mut DashboardState, order: Vec<String>) {
    state
        .car_setup_card_order
        .retain(|key| !order.contains(key));
    state.car_setup_card_order.extend(order);
}

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    SelectTab(DashboardTab),
    CycleTab,
    ToggleConnection,
    OpenIbt,
    OpenReferenceIbt,
    ClearReferenceIbt,
    SelectLap(usize),
    SelectReferenceLap(usize),
    Refresh,
    SpeedPlot(TimeSeriesMessage),
    PedalPlot(TimeSeriesMessage),
    BrakePressurePlot(TimeSeriesMessage),
    AbsPlot(TimeSeriesMessage),
    SteeringPlot(TimeSeriesMessage),
    SteeringTorquePlot(TimeSeriesMessage),
    RpmPlot(TimeSeriesMessage),
    GearPlot(TimeSeriesMessage),
    DynamicsPlot(TimeSeriesMessage),
    YawPlot(TimeSeriesMessage),
    WheelSlipPlot(TimeSeriesMessage),
    TyrePlot(TimeSeriesMessage),
    SuspensionPlot(TimeSeriesMessage),
    FuelPlot(TimeSeriesMessage),
    DeltaPlot(TimeSeriesMessage),
    ToggleChart(ChartId, bool),
    ToggleChartCollapsed(ChartId),
    BeginChartDrag(ChartId),
    ChartLayoutChanged {
        chart: ChartId,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    BeginChartListDrag(ChartId),
    ChartListLayoutChanged {
        chart: ChartId,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    ToggleLapAnalysisCardCollapsed(LapAnalysisCardId),
    BeginLapAnalysisDrag(LapAnalysisCardId),
    LapAnalysisLayoutChanged {
        card: LapAnalysisCardId,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    ToggleSetupCardCollapsed(SetupCardId),
    BeginSetupCardDrag(SetupCardId),
    SetupCardLayoutChanged {
        card: SetupCardId,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    ToggleCarSetupCardCollapsed(String),
    ToggleCarSetupCardMaximized(String),
    BeginCarSetupCardDrag(String),
    CarSetupCardLayoutChanged {
        card: String,
        bounds: Rectangle,
        visible_bounds: Option<Rectangle>,
    },
    FinishCardDrag,
    SetChartColumns(ChartColumns),
    ResetDashboardLayout,
    ToggleChartMaximized(ChartId),
    DragCursor(Point),
    KeyboardModifiersChanged(keyboard::Modifiers),
    CancelPointerInteractions {
        reset_modifiers: bool,
    },
}

impl DashboardMessage {
    pub const fn resets_layout(&self) -> bool {
        matches!(self, Self::ResetDashboardLayout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveChartNavigation {
    None,
    Browse,
    Follow,
}

fn live_chart_navigation(message: &TimeSeriesMessage) -> LiveChartNavigation {
    match message {
        TimeSeriesMessage::PanX(_) | TimeSeriesMessage::ZoomX(_) => LiveChartNavigation::Browse,
        TimeSeriesMessage::ResetX => LiveChartNavigation::Follow,
        _ => LiveChartNavigation::None,
    }
}

fn update_chart_focus(
    state: &mut DashboardState,
    session: &Session,
    cursor_x: Option<f64>,
    live_navigation: LiveChartNavigation,
) {
    if session.ibt_info().is_none() {
        match live_navigation {
            LiveChartNavigation::Browse => state.live_follow = false,
            LiveChartNavigation::Follow => state.live_follow = true,
            LiveChartNavigation::None => {},
        }
        state.focus_from_cursor = false;

        if let Some((earliest, latest)) = session.live_chart_time_bounds() {
            let target = if state.live_follow {
                latest
            } else {
                state.speed_chart.x_view_max().clamp(earliest, latest)
            };
            focus_at(state, session, target);
        }
    } else if let Some(x) = cursor_x {
        state.focus_from_cursor = true;
        focus_at(state, session, x);
    }
}

pub fn update(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
    message: DashboardMessage,
) -> Option<Action> {
    match message {
        DashboardMessage::SelectTab(tab) => {
            if state.active_tab == tab {
                return None;
            }

            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.cancel_chart_interactions();
            state.active_tab = tab;

            match tab {
                DashboardTab::Telemetry => {
                    state.invalidate_chart_layouts();
                    sync_telemetry(state, session, reference_session);
                },
                DashboardTab::CarSetup => sync_session_metadata(state, session),
            }
            None
        },
        DashboardMessage::CycleTab => {
            let next = match state.active_tab {
                DashboardTab::Telemetry => DashboardTab::CarSetup,
                DashboardTab::CarSetup => DashboardTab::Telemetry,
            };
            update(
                state,
                session,
                reference_session,
                DashboardMessage::SelectTab(next),
            )
        },
        DashboardMessage::ToggleConnection => {
            Some(Action::SetConnected(!session.wants_connection()))
        },
        DashboardMessage::OpenIbt => Some(Action::OpenIbt),
        DashboardMessage::OpenReferenceIbt => Some(Action::OpenReferenceIbt),
        DashboardMessage::ClearReferenceIbt => Some(Action::ClearReferenceIbt),
        DashboardMessage::SelectLap(index) => {
            if index < session.laps().len() {
                state.selected_lap_index = Some(index);
                state.focus_x = None;
                state.focus_from_cursor = false;
                sync_telemetry(state, session, reference_session);
            }
            None
        },
        DashboardMessage::SelectReferenceLap(index) => {
            if reference_session.is_some_and(|reference| index < reference.laps().len()) {
                state.selected_reference_lap_index = Some(index);
                sync_telemetry(state, session, reference_session);
            }
            None
        },
        DashboardMessage::Refresh => {
            sync_session_metadata(state, session);
            if state.active_tab == DashboardTab::Telemetry
                && !state.is_dragging_card()
                && telemetry_sync_is_pending(state, session)
            {
                let scope = if session.ibt_info().is_none() {
                    TelemetrySyncScope::LiveVisible
                } else {
                    TelemetrySyncScope::All
                };
                sync_telemetry_with_scope(state, session, reference_session, scope);
            }
            None
        },
        DashboardMessage::SpeedPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.speed_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::PedalPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.pedal_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::BrakePressurePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.brake_pressure_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::AbsPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.abs_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::SteeringPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.steering_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::SteeringTorquePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.steering_torque_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::RpmPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.rpm_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::GearPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.gear_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::DynamicsPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.dynamics_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::YawPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.yaw_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::WheelSlipPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.wheel_slip_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::TyrePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.tyre_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::SuspensionPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.suspension_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::FuelPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.fuel_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::DeltaPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.delta_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        DashboardMessage::ToggleChart(chart, visible) => {
            if state.chart_visibility[chart.index()] == visible {
                return None;
            }
            state.chart_visibility[chart.index()] = visible;
            state.invalidate_chart_layouts();
            if !visible && state.maximized_chart == Some(chart) {
                state.maximized_chart = None;
            }
            if !visible && state.dragging_chart == Some(chart) {
                state.clear_chart_drag();
            } else {
                update_chart_drop_target(state);
            }
            state.mark_layout_changed();
            None
        },
        DashboardMessage::ToggleChartCollapsed(chart) => {
            let collapsed = &mut state.chart_collapsed[chart.index()];
            *collapsed = !*collapsed;
            if *collapsed && state.maximized_chart == Some(chart) {
                state.maximized_chart = None;
            }
            state.invalidate_chart_layouts();
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        DashboardMessage::BeginChartDrag(chart) => {
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.dragging_chart = Some(chart);
            state.drop_target = None;
            state.drag_origin = None;
            state.drag_cursor = None;
            state.drag_source_bounds =
                state.chart_layouts[chart.index()].map(|layout| layout.bounds);
            None
        },
        DashboardMessage::ChartLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        } => {
            state.chart_layouts[chart.index()] = Some(CardLayout {
                bounds,
                visible_bounds,
            });
            if state.dragging_chart == Some(chart) && state.drag_source_bounds.is_none() {
                state.drag_source_bounds = Some(bounds);
            }
            update_chart_drop_target(state);
            None
        },
        DashboardMessage::BeginChartListDrag(chart) => {
            state.clear_chart_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.dragging_chart_list_item = Some(chart);
            state.chart_list_drop_target = None;
            state.chart_list_drag_origin = None;
            state.chart_list_drag_cursor = None;
            state.chart_list_drag_source_bounds =
                state.chart_list_layouts[chart.index()].map(|layout| layout.bounds);
            None
        },
        DashboardMessage::ChartListLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        } => {
            state.chart_list_layouts[chart.index()] = Some(CardLayout {
                bounds,
                visible_bounds,
            });
            if state.dragging_chart_list_item == Some(chart)
                && state.chart_list_drag_source_bounds.is_none()
            {
                state.chart_list_drag_source_bounds = Some(bounds);
            }
            update_chart_list_drop_target(state);
            None
        },
        DashboardMessage::ToggleLapAnalysisCardCollapsed(card) => {
            let collapsed = &mut state.lap_analysis_collapsed[card.index()];
            *collapsed = !*collapsed;
            state.lap_analysis_layouts[card.index()] = None;
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        DashboardMessage::BeginLapAnalysisDrag(card) => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.dragging_lap_analysis_card = Some(card);
            state.lap_analysis_drop_target = None;
            state.lap_analysis_drag_origin = None;
            state.lap_analysis_drag_cursor = None;
            state.lap_analysis_drag_source_bounds =
                state.lap_analysis_layouts[card.index()].map(|layout| layout.bounds);
            None
        },
        DashboardMessage::LapAnalysisLayoutChanged {
            card,
            bounds,
            visible_bounds,
        } => {
            state.lap_analysis_layouts[card.index()] = Some(CardLayout {
                bounds,
                visible_bounds,
            });
            if state.dragging_lap_analysis_card == Some(card)
                && state.lap_analysis_drag_source_bounds.is_none()
            {
                state.lap_analysis_drag_source_bounds = Some(bounds);
            }
            update_lap_analysis_drop_target(state);
            None
        },
        DashboardMessage::ToggleSetupCardCollapsed(card) => {
            let collapsed = &mut state.setup_card_collapsed[card.index()];
            *collapsed = !*collapsed;
            state.setup_card_layouts[card.index()] = None;
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        DashboardMessage::BeginSetupCardDrag(card) => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_car_setup_card_drag();
            state.dragging_setup_card = Some(card);
            state.setup_card_drop_target = None;
            state.setup_card_drag_origin = None;
            state.setup_card_drag_cursor = None;
            state.setup_card_drag_source_bounds =
                state.setup_card_layouts[card.index()].map(|layout| layout.bounds);
            None
        },
        DashboardMessage::SetupCardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        } => {
            state.setup_card_layouts[card.index()] = Some(CardLayout {
                bounds,
                visible_bounds,
            });
            if state.dragging_setup_card == Some(card)
                && state.setup_card_drag_source_bounds.is_none()
            {
                state.setup_card_drag_source_bounds = Some(bounds);
            }
            update_setup_card_drop_target(state);
            None
        },
        DashboardMessage::ToggleCarSetupCardCollapsed(card) => {
            let collapsed = !state
                .car_setup_card_collapsed
                .get(&card)
                .copied()
                .unwrap_or(false);
            state
                .car_setup_card_collapsed
                .insert(card.clone(), collapsed);
            if collapsed && state.maximized_car_setup_card.as_ref() == Some(&card) {
                state.maximized_car_setup_card = None;
            }
            state.car_setup_card_layouts.remove(&card);
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        DashboardMessage::ToggleCarSetupCardMaximized(card) => {
            let maximizing = state.maximized_car_setup_card.as_ref() != Some(&card);
            state.maximized_car_setup_card = maximizing.then(|| card.clone());
            let expanded_collapsed = maximizing
                && state
                    .car_setup_card_collapsed
                    .get(&card)
                    .copied()
                    .unwrap_or(false);
            if expanded_collapsed {
                state.car_setup_card_collapsed.insert(card, false);
            }
            state.car_setup_card_layouts.clear();
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            if expanded_collapsed {
                state.mark_layout_changed();
            }
            None
        },
        DashboardMessage::BeginCarSetupCardDrag(card) => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.car_setup_drag_source_bounds = state
                .car_setup_card_layouts
                .get(&card)
                .map(|layout| layout.bounds);
            state.dragging_car_setup_card = Some(card);
            None
        },
        DashboardMessage::CarSetupCardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        } => {
            state.car_setup_card_layouts.insert(
                card.clone(),
                CardLayout {
                    bounds,
                    visible_bounds,
                },
            );
            if state.dragging_car_setup_card.as_ref() == Some(&card)
                && state.car_setup_drag_source_bounds.is_none()
            {
                state.car_setup_drag_source_bounds = Some(bounds);
            }
            update_car_setup_drop_target(state);
            None
        },
        DashboardMessage::FinishCardDrag => {
            let mut layout_changed = false;
            let mut chart_order_changed = false;
            if let (Some(dragging), Some(target)) = (state.dragging_chart, state.drop_target) {
                chart_order_changed = move_item_to(&mut state.chart_order, dragging, target);
            }
            if let (Some(dragging), Some(target)) =
                (state.dragging_chart_list_item, state.chart_list_drop_target)
            {
                chart_order_changed |= move_item_to(&mut state.chart_order, dragging, target);
            }
            layout_changed |= chart_order_changed;
            if let (Some(dragging), Some(target)) = (
                state.dragging_lap_analysis_card,
                state.lap_analysis_drop_target,
            ) {
                layout_changed |= move_item_to(&mut state.lap_analysis_order, dragging, target);
            }
            if let (Some(dragging), Some(target)) =
                (state.dragging_setup_card, state.setup_card_drop_target)
            {
                layout_changed |= move_item_to(&mut state.setup_card_order, dragging, target);
            }
            if let (Some(dragging), Some(target)) = (
                state.dragging_car_setup_card.clone(),
                state.car_setup_drop_target.clone(),
            ) {
                let mut order = current_car_setup_card_order(state);
                if move_dynamic_item_to(&mut order, &dragging, &target) {
                    merge_current_car_setup_order(state, order);
                    state.car_setup_card_layouts.clear();
                    layout_changed = true;
                }
            }
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            if chart_order_changed {
                state.invalidate_chart_layouts();
                state.invalidate_chart_list_layouts();
            }
            if layout_changed {
                state.mark_layout_changed();
            }
            None
        },
        DashboardMessage::SetChartColumns(columns) => {
            if state.chart_columns == columns {
                return None;
            }
            state.chart_columns = columns;
            state.invalidate_chart_layouts();
            state.mark_layout_changed();
            None
        },
        DashboardMessage::ResetDashboardLayout => {
            state.apply_layout(&DashboardLayout::default());
            // Reset also removes an explicit persisted override, so it is a
            // meaningful save operation even when the values were defaults.
            state.mark_layout_changed();
            None
        },
        DashboardMessage::ToggleChartMaximized(chart) => {
            let maximizing = state.maximized_chart != Some(chart);
            state.maximized_chart = maximizing.then_some(chart);
            let expanded_collapsed = maximizing && state.chart_collapsed[chart.index()];
            if expanded_collapsed {
                state.chart_collapsed[chart.index()] = false;
            }
            state.invalidate_chart_layouts();
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            if expanded_collapsed {
                state.mark_layout_changed();
            }
            None
        },
        DashboardMessage::DragCursor(position) => {
            if state.dragging_chart.is_some() {
                state.drag_origin.get_or_insert(position);
                state.drag_cursor = Some(position);
                update_chart_drop_target(state);
            }
            if state.dragging_chart_list_item.is_some() {
                state.chart_list_drag_origin.get_or_insert(position);
                state.chart_list_drag_cursor = Some(position);
                update_chart_list_drop_target(state);
            }
            if state.dragging_lap_analysis_card.is_some() {
                state.lap_analysis_drag_origin.get_or_insert(position);
                state.lap_analysis_drag_cursor = Some(position);
                update_lap_analysis_drop_target(state);
            }
            if state.dragging_setup_card.is_some() {
                state.setup_card_drag_origin.get_or_insert(position);
                state.setup_card_drag_cursor = Some(position);
                update_setup_card_drop_target(state);
            }
            if state.dragging_car_setup_card.is_some() {
                state.car_setup_drag_origin.get_or_insert(position);
                state.car_setup_drag_cursor = Some(position);
                update_car_setup_drop_target(state);
            }
            None
        },
        DashboardMessage::KeyboardModifiersChanged(modifiers) => {
            state.modifiers = modifiers;
            None
        },
        DashboardMessage::CancelPointerInteractions { reset_modifiers } => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.clear_car_setup_card_drag();
            state.cancel_chart_interactions();
            if reset_modifiers {
                state.modifiers = keyboard::Modifiers::NONE;
            }
            None
        },
    }
}

fn update_chart_drop_target(state: &mut DashboardState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_chart,
        state.drag_origin,
        state.drag_cursor,
        state.drag_source_bounds,
    ) else {
        state.drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.drop_target = select_drop_target(dragging, dragged_bounds, &state.chart_order, |chart| {
        state.chart_visibility[chart.index()]
            .then(|| state.chart_layouts[chart.index()].and_then(|layout| layout.visible_bounds))
            .flatten()
    });
}

fn update_chart_list_drop_target(state: &mut DashboardState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_chart_list_item,
        state.chart_list_drag_origin,
        state.chart_list_drag_cursor,
        state.chart_list_drag_source_bounds,
    ) else {
        state.chart_list_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.chart_list_drop_target =
        select_drop_target(dragging, dragged_bounds, &state.chart_order, |chart| {
            state.chart_list_layouts[chart.index()].and_then(|layout| layout.visible_bounds)
        });
}

fn update_lap_analysis_drop_target(state: &mut DashboardState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_lap_analysis_card,
        state.lap_analysis_drag_origin,
        state.lap_analysis_drag_cursor,
        state.lap_analysis_drag_source_bounds,
    ) else {
        state.lap_analysis_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.lap_analysis_drop_target = select_drop_target(
        dragging,
        dragged_bounds,
        &state.lap_analysis_order,
        |card| state.lap_analysis_layouts[card.index()].and_then(|layout| layout.visible_bounds),
    );
}

fn update_setup_card_drop_target(state: &mut DashboardState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_setup_card,
        state.setup_card_drag_origin,
        state.setup_card_drag_cursor,
        state.setup_card_drag_source_bounds,
    ) else {
        state.setup_card_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    state.setup_card_drop_target =
        select_drop_target(dragging, dragged_bounds, &state.setup_card_order, |card| {
            state.setup_card_layouts[card.index()].and_then(|layout| layout.visible_bounds)
        });
}

fn update_car_setup_drop_target(state: &mut DashboardState) {
    let (Some(dragging), Some(origin), Some(cursor), Some(source_bounds)) = (
        state.dragging_car_setup_card.as_ref(),
        state.car_setup_drag_origin,
        state.car_setup_drag_cursor,
        state.car_setup_drag_source_bounds,
    ) else {
        state.car_setup_drop_target = None;
        return;
    };

    let dragged_bounds = Rectangle {
        x: source_bounds.x + cursor.x - origin.x,
        y: source_bounds.y + cursor.y - origin.y,
        ..source_bounds
    };
    let order = current_car_setup_card_order(state);
    state.car_setup_drop_target =
        select_dynamic_drop_target(dragging, dragged_bounds, &order, |card| {
            state
                .car_setup_card_layouts
                .get(card)
                .and_then(|layout| layout.visible_bounds)
        });
}

fn select_drop_target<Id: Copy + Eq>(
    dragging: Id,
    dragged_bounds: Rectangle,
    order: &[Id],
    mut visible_bounds_for: impl FnMut(Id) -> Option<Rectangle>,
) -> Option<Id> {
    let mut best = None;

    for &candidate in order {
        if candidate == dragging {
            continue;
        }
        let Some(visible_bounds) = visible_bounds_for(candidate) else {
            continue;
        };
        let Some(overlap) = dragged_bounds.intersection(&visible_bounds) else {
            continue;
        };
        let area = overlap.width * overlap.height;

        if best.is_none_or(|(_, best_area)| area > best_area) {
            best = Some((candidate, area));
        }
    }

    best.map(|(item, _)| item)
}

fn select_dynamic_drop_target(
    dragging: &str,
    dragged_bounds: Rectangle,
    order: &[String],
    mut visible_bounds_for: impl FnMut(&str) -> Option<Rectangle>,
) -> Option<String> {
    let mut best = None;

    for candidate in order {
        if candidate == dragging {
            continue;
        }
        let Some(visible_bounds) = visible_bounds_for(candidate) else {
            continue;
        };
        let Some(overlap) = dragged_bounds.intersection(&visible_bounds) else {
            continue;
        };
        let area = overlap.width * overlap.height;

        if best.as_ref().is_none_or(|(_, best_area)| area > *best_area) {
            best = Some((candidate.clone(), area));
        }
    }

    best.map(|(item, _)| item)
}

fn move_item_to<Id: Copy + Eq>(order: &mut Vec<Id>, item: Id, target: Id) -> bool {
    let Some(from) = order.iter().position(|candidate| *candidate == item) else {
        return false;
    };
    let Some(to) = order.iter().position(|candidate| *candidate == target) else {
        return false;
    };
    if from == to {
        return false;
    }

    let item = order.remove(from);
    order.insert(to.min(order.len()), item);
    true
}

fn move_dynamic_item_to(order: &mut Vec<String>, item: &str, target: &str) -> bool {
    let Some(from) = order.iter().position(|candidate| candidate == item) else {
        return false;
    };
    let Some(to) = order.iter().position(|candidate| candidate == target) else {
        return false;
    };
    if from == to {
        return false;
    }

    let item = order.remove(from);
    order.insert(to.min(order.len()), item);
    true
}

pub fn subscription(state: &DashboardState, active: bool) -> Subscription<DashboardMessage> {
    if !active && !state.is_dragging_card() {
        return Subscription::none();
    }

    let tab_shortcut = if active {
        iced::event::listen_with(tab_shortcut_event)
    } else {
        Subscription::none()
    };
    let modifier_input = iced::event::listen_with(modifier_event);
    let card_input = if active || state.is_dragging_card() {
        iced::event::listen_with(card_input_event)
    } else {
        Subscription::none()
    };
    let drag_cursor = if state.is_dragging_card() {
        iced::event::listen_with(drag_cursor_event)
    } else {
        Subscription::none()
    };
    Subscription::batch([tab_shortcut, modifier_input, card_input, drag_cursor])
}

fn tab_shortcut_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<DashboardMessage> {
    match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Tab),
            modifiers,
            repeat: false,
            ..
        }) if modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(DashboardMessage::CycleTab)
        },
        _ => None,
    }
}

fn modifier_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<DashboardMessage> {
    match event {
        iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(DashboardMessage::KeyboardModifiersChanged(modifiers))
        },
        iced::Event::Window(iced::window::Event::Unfocused) => {
            Some(DashboardMessage::CancelPointerInteractions {
                reset_modifiers: true,
            })
        },
        _ => None,
    }
}

fn card_input_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<DashboardMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | iced::Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
            Some(DashboardMessage::FinishCardDrag)
        },
        iced::Event::Mouse(mouse::Event::CursorLeft)
        | iced::Event::Touch(iced::touch::Event::FingerLost { .. }) => {
            Some(DashboardMessage::CancelPointerInteractions {
                reset_modifiers: false,
            })
        },
        _ => None,
    }
}

fn drag_cursor_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<DashboardMessage> {
    match event {
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(DashboardMessage::DragCursor(position))
        },
        iced::Event::Touch(iced::touch::Event::FingerMoved { position, .. }) => {
            Some(DashboardMessage::DragCursor(position))
        },
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelemetrySyncScope {
    All,
    LiveVisible,
}

fn live_chart_is_active(state: &DashboardState, chart: ChartId) -> bool {
    let index = chart.index();
    let is_visible_in_viewport =
        state.chart_layouts[index].is_some_and(|layout| layout.visible_bounds.is_some());
    if state.maximized_chart.is_some() {
        return state.maximized_chart == Some(chart)
            && state.chart_visibility[index]
            && !state.chart_collapsed[index]
            && is_visible_in_viewport;
    }

    state.chart_visibility[index] && !state.chart_collapsed[index] && is_visible_in_viewport
}

fn chart_sync_targets(
    state: &DashboardState,
    is_live: bool,
    scope: TelemetrySyncScope,
) -> [bool; ChartId::COUNT] {
    if !is_live || scope == TelemetrySyncScope::All {
        return [true; ChartId::COUNT];
    }

    std::array::from_fn(|index| live_chart_is_active(state, ChartId::ALL[index]))
}

fn telemetry_sync_is_pending(state: &DashboardState, session: &Session) -> bool {
    let packets_received = session.packets_received();
    if state.rendered_packets != packets_received {
        return true;
    }
    if session.ibt_info().is_some() {
        return false;
    }

    ChartId::ALL.into_iter().any(|chart| {
        live_chart_is_active(state, chart)
            && state.chart_packet_cursors[chart.index()] != Some(packets_received)
    })
}

#[derive(Debug, Clone, Copy)]
struct LiveSeriesUpdate {
    cursor: u64,
    minimum_x: f64,
}

fn sync_series_points(
    chart: &mut TimeSeriesChart,
    series_index: usize,
    session: &Session,
    lap_index: Option<usize>,
    live_update: Option<LiveSeriesUpdate>,
    value: impl Fn(&TelemetrySample) -> f32,
) -> Option<Vec<[f64; 2]>> {
    if let Some(update) = live_update {
        let appended = session.live_points_since(update.cursor, value);
        chart.update_live_series_points(series_index, update.minimum_x, &appended);
        None
    } else {
        let points = session.points_in(lap_index, value);
        chart.set_series_points(series_index, &points);
        Some(points)
    }
}

fn sync_optional_series_points(
    chart: &mut TimeSeriesChart,
    series_index: usize,
    session: &Session,
    lap_index: Option<usize>,
    live_update: Option<LiveSeriesUpdate>,
    value: impl Fn(&TelemetrySample) -> Option<f32>,
) -> Option<Vec<[f64; 2]>> {
    if let Some(update) = live_update {
        let appended = session.live_points_since_optional(update.cursor, value);
        chart.update_live_series_points(series_index, update.minimum_x, &appended);
        None
    } else {
        let points = session.points_in_optional(lap_index, value);
        chart.set_series_points(series_index, &points);
        Some(points)
    }
}

pub fn sync_telemetry(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    sync_telemetry_with_scope(state, session, reference_session, TelemetrySyncScope::All);
}

fn sync_telemetry_with_scope(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
    scope: TelemetrySyncScope,
) {
    sync_session_metadata(state, session);

    let lap_index = state.selected_lap_index;
    let is_live = session.ibt_info().is_none();
    let incremental_live = is_live && scope == TelemetrySyncScope::LiveVisible;
    let packets_received = session.packets_received();
    let targets = chart_sync_targets(state, is_live, scope);
    let syncs = |chart: ChartId| targets[chart.index()];
    let clears_missing_comparison = scope == TelemetrySyncScope::All;
    let uses_lap_distance = chart_uses_lap_distance(state, session);
    let chart_duration = session.chart_duration_seconds_for(lap_index);
    let live_bounds = session.live_chart_time_bounds();
    let x_axis = if uses_lap_distance {
        lap_distance_axis()
    } else {
        time_axis()
    };
    let (chart_min, chart_max) = if uses_lap_distance {
        (0.0, LAP_DISTANCE_AXIS_MAX)
    } else if is_live {
        live_bounds.map_or((0.0, HISTORY_WINDOW.as_secs_f64()), |(first, latest)| {
            if first < latest {
                (first, latest)
            } else {
                (first, first + 0.001)
            }
        })
    } else {
        (0.0, chart_duration)
    };
    let chart_packet_cursors = state.chart_packet_cursors;
    let live_minimum_x = session.live_chart_minimum_x().unwrap_or(chart_min);
    let initializes_live_chart = |chart: ChartId| {
        incremental_live
            && chart_packet_cursors[chart.index()].is_none_or(|cursor| cursor > packets_received)
    };
    let clears_comparison_for =
        |chart: ChartId| clears_missing_comparison || initializes_live_chart(chart);
    let live_update = |chart: ChartId| {
        if !incremental_live {
            return None;
        }
        let cursor = chart_packet_cursors[chart.index()]?;
        if cursor > packets_received {
            return None;
        }
        Some(LiveSeriesUpdate {
            cursor,
            minimum_x: live_minimum_x,
        })
    };
    let comparison = selected_comparison(state, session, reference_session);

    if syncs(ChartId::Speed) {
        let _ = sync_series_points(
            &mut state.speed_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::Speed),
            |sample| sample.speed_kmh,
        );
        if !incremental_live && comparison.is_some() {
            let reference = comparison_points_for(session, comparison, |sample| sample.speed_kmh);
            state.speed_chart.set_series_points(1, &reference);
        } else if clears_comparison_for(ChartId::Speed) {
            state.speed_chart.set_series_points(1, &[]);
        }
    }

    if syncs(ChartId::Pedal) {
        let update = live_update(ChartId::Pedal);
        let _ = sync_series_points(
            &mut state.pedal_chart,
            0,
            session,
            lap_index,
            update,
            |sample| pedal_percent(sample.throttle),
        );
        let _ = sync_series_points(
            &mut state.pedal_chart,
            1,
            session,
            lap_index,
            update,
            |sample| pedal_percent(sample.brake),
        );
        if !incremental_live && comparison.is_some() {
            let reference_throttle =
                comparison_points_for(session, comparison, |sample| pedal_percent(sample.throttle));
            let reference_brake =
                comparison_points_for(session, comparison, |sample| pedal_percent(sample.brake));
            state.pedal_chart.set_series_points(2, &reference_throttle);
            state.pedal_chart.set_series_points(3, &reference_brake);
        } else if clears_comparison_for(ChartId::Pedal) {
            state.pedal_chart.set_series_points(2, &[]);
            state.pedal_chart.set_series_points(3, &[]);
        }
    }

    if syncs(ChartId::BrakePressure) {
        let update = live_update(ChartId::BrakePressure);
        let mut maximum = 120.0;
        let mut rebuilt = false;
        for wheel in 0..4 {
            if let Some(points) = sync_optional_series_points(
                &mut state.brake_pressure_chart,
                wheel,
                session,
                lap_index,
                update,
                move |sample| sample.brake_line_pressure_bar.get(wheel),
            ) {
                maximum = maximum_y(&points, maximum);
                rebuilt = true;
            }
        }
        if rebuilt {
            state.brake_pressure_chart.set_y_limits(0.0, maximum * 1.1);
        }
    }

    if syncs(ChartId::Abs) {
        let _ = sync_optional_series_points(
            &mut state.abs_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::Abs),
            |sample| sample.abs_active.map(abs_activity_percent),
        );
    }

    if syncs(ChartId::Steering) {
        let points = sync_series_points(
            &mut state.steering_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::Steering),
            |sample| sample.steering_angle.to_degrees(),
        );
        if let Some(points) = points {
            let reference = comparison.map(|_| {
                comparison_points_for(session, comparison, |sample| {
                    sample.steering_angle.to_degrees()
                })
            });
            let limits = symmetric_y_limits(
                points
                    .iter()
                    .chain(reference.iter().flatten())
                    .map(|point| point[1]),
                180.0,
            );
            state.steering_chart.set_y_limits(limits.0, limits.1);
            if let Some(reference) = reference {
                state.steering_chart.set_series_points(1, &reference);
            } else if clears_comparison_for(ChartId::Steering) {
                state.steering_chart.set_series_points(1, &[]);
            }
        } else {
            let range = session
                .live_value_range(|sample| sample.steering_angle.to_degrees())
                .into_iter()
                .flat_map(|(minimum, maximum)| [minimum, maximum]);
            let limits = symmetric_y_limits(range, 180.0);
            state.steering_chart.set_y_limits(limits.0, limits.1);
        }
    }

    if syncs(ChartId::SteeringTorque) {
        let points = sync_optional_series_points(
            &mut state.steering_torque_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::SteeringTorque),
            |sample| sample.steering_wheel_torque_nm.get(0),
        );
        if let Some(points) = points {
            let reference = comparison.map(|_| {
                comparison_points_for_optional(session, comparison, |sample| {
                    sample.steering_wheel_torque_nm.get(0)
                })
            });
            let limits = symmetric_y_limits(
                points
                    .iter()
                    .chain(reference.iter().flatten())
                    .map(|point| point[1]),
                30.0,
            );
            state.steering_torque_chart.set_y_limits(limits.0, limits.1);
            if let Some(reference) = reference {
                state.steering_torque_chart.set_series_points(1, &reference);
            } else if clears_comparison_for(ChartId::SteeringTorque) {
                state.steering_torque_chart.set_series_points(1, &[]);
            }
        } else {
            let range = session
                .live_value_range_optional(|sample| sample.steering_wheel_torque_nm.get(0))
                .into_iter()
                .flat_map(|(minimum, maximum)| [minimum, maximum]);
            let limits = symmetric_y_limits(range, 30.0);
            state.steering_torque_chart.set_y_limits(limits.0, limits.1);
        }
    }

    if syncs(ChartId::Rpm) {
        let points = sync_series_points(
            &mut state.rpm_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::Rpm),
            |sample| sample.rpm.max(0) as f32,
        );
        if let Some(points) = points {
            let reference = comparison.map(|_| {
                comparison_points_for(session, comparison, |sample| sample.rpm.max(0) as f32)
            });
            let maximum = maximum_y(
                &points,
                reference
                    .as_deref()
                    .map_or(8_000.0, |points| maximum_y(points, 8_000.0)),
            ) * 1.1;
            state.rpm_chart.set_y_limits(0.0, maximum);
            if let Some(reference) = reference {
                state.rpm_chart.set_series_points(1, &reference);
            } else if clears_comparison_for(ChartId::Rpm) {
                state.rpm_chart.set_series_points(1, &[]);
            }
        } else {
            let maximum = session
                .live_value_range(|sample| sample.rpm.max(0) as f32)
                .map_or(8_000.0, |(_, maximum)| maximum.max(8_000.0))
                * 1.1;
            state.rpm_chart.set_y_limits(0.0, maximum);
        }
    }

    if syncs(ChartId::Gear) {
        let points = session.gear_points(lap_index);
        state.gear_chart.set_series_points(0, &points);
        if let Some((lap, reference, reference_lap)) = comparison {
            let reference = session.comparison_gear_points(lap, reference, reference_lap);
            state.gear_chart.set_series_points(1, &reference);
        } else if clears_comparison_for(ChartId::Gear) {
            state.gear_chart.set_series_points(1, &[]);
        }
    }

    if syncs(ChartId::Dynamics) {
        let update = live_update(ChartId::Dynamics);
        let lateral = sync_series_points(
            &mut state.dynamics_chart,
            0,
            session,
            lap_index,
            update,
            |sample| sample.acceleration_g[0],
        );
        let longitudinal = sync_series_points(
            &mut state.dynamics_chart,
            1,
            session,
            lap_index,
            update,
            |sample| sample.acceleration_g[1],
        );
        if let (Some(lateral), Some(longitudinal)) = (lateral, longitudinal) {
            let reference_lateral = comparison.map(|_| {
                comparison_points_for(session, comparison, |sample| sample.acceleration_g[0])
            });
            let reference_longitudinal = comparison.map(|_| {
                comparison_points_for(session, comparison, |sample| sample.acceleration_g[1])
            });
            let limits = symmetric_y_limits(
                lateral
                    .iter()
                    .chain(&longitudinal)
                    .chain(reference_lateral.iter().flatten())
                    .chain(reference_longitudinal.iter().flatten())
                    .map(|point| point[1]),
                3.6,
            );
            state.dynamics_chart.set_y_limits(limits.0, limits.1);
            if let (Some(reference_lateral), Some(reference_longitudinal)) =
                (reference_lateral, reference_longitudinal)
            {
                state
                    .dynamics_chart
                    .set_series_points(2, &reference_lateral);
                state
                    .dynamics_chart
                    .set_series_points(3, &reference_longitudinal);
            } else if clears_comparison_for(ChartId::Dynamics) {
                state.dynamics_chart.set_series_points(2, &[]);
                state.dynamics_chart.set_series_points(3, &[]);
            }
        } else {
            let ranges = [
                session.live_value_range(|sample| sample.acceleration_g[0]),
                session.live_value_range(|sample| sample.acceleration_g[1]),
            ];
            let limits = symmetric_y_limits(
                ranges
                    .into_iter()
                    .flatten()
                    .flat_map(|(minimum, maximum)| [minimum, maximum]),
                3.6,
            );
            state.dynamics_chart.set_y_limits(limits.0, limits.1);
        }
    }

    if syncs(ChartId::Yaw) {
        let points = sync_series_points(
            &mut state.yaw_chart,
            0,
            session,
            lap_index,
            live_update(ChartId::Yaw),
            |sample| sample.yaw_rate_rad_s.to_degrees(),
        );
        if let Some(points) = points {
            let reference = comparison.map(|_| {
                comparison_points_for(session, comparison, |sample| {
                    sample.yaw_rate_rad_s.to_degrees()
                })
            });
            let limits = symmetric_y_limits(
                points
                    .iter()
                    .chain(reference.iter().flatten())
                    .map(|point| point[1]),
                60.0,
            );
            state.yaw_chart.set_y_limits(limits.0, limits.1);
            if let Some(reference) = reference {
                state.yaw_chart.set_series_points(1, &reference);
            } else if clears_comparison_for(ChartId::Yaw) {
                state.yaw_chart.set_series_points(1, &[]);
            }
        } else {
            let range = session
                .live_value_range(|sample| sample.yaw_rate_rad_s.to_degrees())
                .into_iter()
                .flat_map(|(minimum, maximum)| [minimum, maximum]);
            let limits = symmetric_y_limits(range, 60.0);
            state.yaw_chart.set_y_limits(limits.0, limits.1);
        }
    }

    if syncs(ChartId::WheelSlip) {
        if let Some(update) = live_update(ChartId::WheelSlip) {
            for wheel in 0..4 {
                let appended = session
                    .live_points_since(update.cursor, |sample| sample.wheel_slip[wheel] * 100.0);
                state.wheel_slip_chart.update_live_series_points(
                    wheel,
                    update.minimum_x,
                    &appended,
                );
            }
            let ranges: [Option<(f64, f64)>; 4] = std::array::from_fn(|wheel| {
                session.live_value_range(|sample| sample.wheel_slip[wheel] * 100.0)
            });
            let limits = symmetric_y_limits(
                ranges
                    .into_iter()
                    .flatten()
                    .flat_map(|(minimum, maximum)| [minimum, maximum]),
                20.0,
            );
            state.wheel_slip_chart.set_y_limits(limits.0, limits.1);
        } else {
            let points: [Vec<[f64; 2]>; 4] = std::array::from_fn(|wheel| {
                session.points_in(lap_index, |sample| sample.wheel_slip[wheel] * 100.0)
            });
            let reference: Option<[Vec<[f64; 2]>; 4]> = comparison.map(|_| {
                std::array::from_fn(|wheel| {
                    comparison_points_for(session, comparison, |sample| {
                        sample.wheel_slip[wheel] * 100.0
                    })
                })
            });
            let limits = symmetric_y_limits(
                points
                    .iter()
                    .chain(reference.iter().flatten())
                    .flat_map(|points| points.iter())
                    .map(|point| point[1]),
                20.0,
            );
            state.wheel_slip_chart.set_y_limits(limits.0, limits.1);
            for (wheel, points) in points.iter().enumerate() {
                state.wheel_slip_chart.set_series_points(wheel, points);
            }
            if let Some(reference) = reference {
                for (wheel, points) in reference.iter().enumerate() {
                    state.wheel_slip_chart.set_series_points(4 + wheel, points);
                }
            } else if clears_comparison_for(ChartId::WheelSlip) {
                for wheel in 0..4 {
                    state.wheel_slip_chart.set_series_points(4 + wheel, &[]);
                }
            }
        }
    }

    if syncs(ChartId::Tyre) {
        if let Some(update) = live_update(ChartId::Tyre) {
            for wheel in 0..4 {
                let appended = session.live_points_since(update.cursor, |sample| {
                    sample.tyre_core_temperature_c[wheel]
                });
                state
                    .tyre_chart
                    .update_live_series_points(wheel, update.minimum_x, &appended);
            }
        } else {
            for wheel in 0..4 {
                let points =
                    session.points_in(lap_index, |sample| sample.tyre_core_temperature_c[wheel]);
                state.tyre_chart.set_series_points(wheel, &points);
            }
            if comparison.is_some() {
                for wheel in 0..4 {
                    let reference = comparison_points_for(session, comparison, |sample| {
                        sample.tyre_core_temperature_c[wheel]
                    });
                    state.tyre_chart.set_series_points(4 + wheel, &reference);
                }
            } else if clears_comparison_for(ChartId::Tyre) {
                for wheel in 0..4 {
                    state.tyre_chart.set_series_points(4 + wheel, &[]);
                }
            }
        }
    }

    if syncs(ChartId::Suspension) {
        if let Some(update) = live_update(ChartId::Suspension) {
            for wheel in 0..4 {
                let appended = session.live_points_since(update.cursor, |sample| {
                    sample.suspension_travel_m[wheel] * 1_000.0
                });
                state.suspension_chart.update_live_series_points(
                    wheel,
                    update.minimum_x,
                    &appended,
                );
            }
            let ranges: [Option<(f64, f64)>; 4] = std::array::from_fn(|wheel| {
                session.live_value_range(|sample| sample.suspension_travel_m[wheel] * 1_000.0)
            });
            let limits = padded_y_limits(
                ranges
                    .into_iter()
                    .flatten()
                    .flat_map(|(minimum, maximum)| [minimum, maximum]),
                (-20.0, 120.0),
            );
            state.suspension_chart.set_y_limits(limits.0, limits.1);
        } else {
            let points: [Vec<[f64; 2]>; 4] = std::array::from_fn(|wheel| {
                session.points_in(lap_index, |sample| {
                    sample.suspension_travel_m[wheel] * 1_000.0
                })
            });
            let reference: Option<[Vec<[f64; 2]>; 4]> = comparison.map(|_| {
                std::array::from_fn(|wheel| {
                    comparison_points_for(session, comparison, |sample| {
                        sample.suspension_travel_m[wheel] * 1_000.0
                    })
                })
            });
            let limits = padded_y_limits(
                points
                    .iter()
                    .chain(reference.iter().flatten())
                    .flat_map(|points| points.iter())
                    .map(|point| point[1]),
                (-20.0, 120.0),
            );
            state.suspension_chart.set_y_limits(limits.0, limits.1);
            for (wheel, points) in points.iter().enumerate() {
                state.suspension_chart.set_series_points(wheel, points);
            }
            if let Some(reference) = reference {
                for (wheel, points) in reference.iter().enumerate() {
                    state.suspension_chart.set_series_points(4 + wheel, points);
                }
            } else if clears_comparison_for(ChartId::Suspension) {
                for wheel in 0..4 {
                    state.suspension_chart.set_series_points(4 + wheel, &[]);
                }
            }
        }
    }

    if syncs(ChartId::Fuel) {
        let points = session.fuel_used_points(lap_index);
        let reference = comparison.map(|(lap, reference, reference_lap)| {
            session.comparison_fuel_used_points(lap, reference, reference_lap)
        });
        let maximum = maximum_y(
            &points,
            reference
                .as_deref()
                .map_or(1.0, |points| maximum_y(points, 1.0)),
        ) * 1.2;
        state.fuel_chart.set_y_limits(0.0, maximum);
        state.fuel_chart.set_series_points(0, &points);
        if let Some(reference) = reference {
            state.fuel_chart.set_series_points(1, &reference);
        } else if clears_comparison_for(ChartId::Fuel) {
            state.fuel_chart.set_series_points(1, &[]);
        }
    }

    if syncs(ChartId::Delta) {
        if !is_live || scope == TelemetrySyncScope::All {
            let points = comparison.map_or_else(
                || {
                    lap_index
                        .zip(session.fastest_complete_lap_index())
                        .map_or_else(Vec::new, |(lap, reference)| {
                            session.lap_delta_points(lap, reference)
                        })
                },
                |(lap, reference, reference_lap)| {
                    session.lap_delta_points_against(lap, reference, reference_lap)
                },
            );
            let limits = symmetric_y_limits(points.iter().map(|point| point[1]), 6.0);
            state.delta_chart.set_y_limits(limits.0, limits.1);
            state.delta_chart.set_series_points(0, &points);
        } else if initializes_live_chart(ChartId::Delta) {
            state.delta_chart.set_series_points(0, &[]);
        }
    }

    let live_follow = state.live_follow;
    for chart_id in ChartId::ALL {
        if !syncs(chart_id) {
            continue;
        }
        let chart = state.chart_mut(chart_id);
        chart.set_x_axis(x_axis);
        if is_live {
            chart.set_live_x_limits(
                chart_min,
                chart_max,
                HISTORY_WINDOW.as_secs_f64(),
                live_follow,
            );
        } else {
            chart.set_x_limits(chart_min, chart_max);
        }
    }

    if !is_live {
        state.chart_packet_cursors.fill(None);
    } else if incremental_live {
        for chart in ChartId::ALL {
            if syncs(chart) {
                state.chart_packet_cursors[chart.index()] = Some(packets_received);
            }
        }
    } else {
        state.chart_packet_cursors.fill(Some(packets_received));
    }
    state.rendered_packets = packets_received;
    let focus_target = if is_live {
        state.focus_from_cursor = false;
        let latest = live_bounds.map_or(0.0, |(_, latest)| latest);
        if state.live_follow {
            latest
        } else {
            state.speed_chart.x_view_max().clamp(chart_min, latest)
        }
    } else if state.focus_from_cursor {
        state.focus_x.unwrap_or(chart_max)
    } else if uses_lap_distance {
        chart_max
    } else {
        chart_duration
    };
    focus_at(state, session, focus_target);
}

fn chart_uses_lap_distance(state: &DashboardState, session: &Session) -> bool {
    session.ibt_info().is_some() && state.selected_lap_index.is_some()
}

pub fn reset_telemetry(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    state.cached_session_info_revision = None;
    state.session_metadata = SessionMetadata::default();
    state.car_setup = setup_view::SetupViewData::default();
    state.lap_choices = session
        .laps()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lap)| {
            LapChoice::new(
                index,
                lap.number(),
                lap.duration_ms(),
                session.lap_start_fuel_litres(index),
                lap.is_complete(),
            )
        })
        .collect();
    state.selected_lap_index = session.preferred_lap_index();
    state.focus_x = None;
    state.focused = None;
    state.focus_from_cursor = false;
    state.live_follow = true;
    if state.active_tab == DashboardTab::Telemetry {
        sync_telemetry(state, session, reference_session);
    } else {
        sync_session_metadata(state, session);
    }
}

fn sync_session_metadata(state: &mut DashboardState, session: &Session) {
    let revision = session.session_info_revision();
    if state.cached_session_info_revision == Some(revision) {
        return;
    }

    state.cached_session_info_revision = Some(revision);
    match session.session_info().map(chiaro_irsdk::SessionInfo::parse) {
        Some(Ok(document)) => {
            state.session_metadata = session_metadata(&document);
            state.car_setup = setup_view::SetupViewData::from_document(&document);
        },
        Some(Err(error)) => {
            state.session_metadata = SessionMetadata::default();
            state.car_setup = setup_view::SetupViewData::parse_error(error.to_string());
        },
        None => {
            state.session_metadata = SessionMetadata::default();
            state.car_setup = setup_view::SetupViewData::default();
        },
    }
    state.reconcile_car_setup_cards();
}

fn session_metadata(document: &SessionInfoDocument) -> SessionMetadata {
    let weekend = document.weekend_info.as_ref();
    let options = weekend.and_then(|weekend| weekend.weekend_options.as_ref());
    let current_session = document.session_info.as_ref().and_then(|session_info| {
        session_info
            .current_session_num
            .and_then(|current| {
                session_info
                    .sessions
                    .iter()
                    .find(|session| session.session_num == Some(current))
            })
            .or_else(|| session_info.sessions.first())
    });
    let driver_info = document.driver_info.as_ref();
    let driver = driver_info.and_then(|driver_info| {
        driver_info
            .driver_car_idx
            .and_then(|player_car| {
                driver_info
                    .drivers
                    .iter()
                    .find(|driver| driver.car_idx == Some(player_car))
            })
            .or_else(|| driver_info.drivers.first())
    });

    SessionMetadata {
        track_name: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_display_name.as_deref()),
            weekend.and_then(|weekend| weekend.track_name.as_deref()),
        ]),
        track_config: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_config_name.as_deref())
        ]),
        track_length: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_length.as_deref())
        ]),
        track_turns: weekend.and_then(|weekend| weekend.track_num_turns),
        track_type: join_metadata_values(
            [
                weekend.and_then(|weekend| weekend.track_type.as_deref()),
                weekend.and_then(|weekend| weekend.track_direction.as_deref()),
            ],
            " · ",
        ),
        car_name: first_metadata_value([
            driver.and_then(|driver| driver.car_screen_name.as_deref()),
            driver.and_then(|driver| driver.car_screen_name_short.as_deref()),
            driver.and_then(|driver| driver.car_path.as_deref()),
        ]),
        car_class: first_metadata_value([
            driver.and_then(|driver| driver.car_class_short_name.as_deref())
        ]),
        session_type: first_metadata_value([
            current_session.and_then(|session| session.session_type.as_deref()),
            current_session.and_then(|session| session.session_name.as_deref()),
            weekend.and_then(|weekend| weekend.event_type.as_deref()),
        ]),
        session_time: current_session
            .and_then(|session| session.session_time.as_ref())
            .map(format_session_time),
        date_time: join_metadata_values(
            [
                options.and_then(|options| options.date.as_deref()),
                options.and_then(|options| options.time_of_day.as_deref()),
            ],
            " · ",
        ),
        weather: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_skies.as_deref()),
            options.and_then(|options| options.skies.as_deref()),
            weekend.and_then(|weekend| weekend.track_weather_type.as_deref()),
            options.and_then(|options| options.weather_type.as_deref()),
        ]),
        air_temperature: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_air_temp.as_deref()),
            options.and_then(|options| options.weather_temp.as_deref()),
        ]),
        surface_temperature: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_surface_temp.as_deref())
        ]),
        humidity: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_relative_humidity.as_deref()),
            options.and_then(|options| options.relative_humidity.as_deref()),
        ]),
        wind: join_metadata_values(
            [
                options
                    .and_then(|options| options.wind_speed.as_deref())
                    .or_else(|| weekend.and_then(|weekend| weekend.track_wind_vel.as_deref())),
                options
                    .and_then(|options| options.wind_direction.as_deref())
                    .or_else(|| weekend.and_then(|weekend| weekend.track_wind_dir.as_deref())),
            ],
            " · ",
        ),
    }
}

fn first_metadata_value<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_owned)
}

fn join_metadata_values<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
    separator: &str,
) -> Option<String> {
    let values = values
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    (!values.is_empty()).then(|| values.join(separator))
}

fn format_session_time(value: &SessionScalar) -> String {
    let seconds = match value {
        SessionScalar::Integer(seconds) => Some(*seconds as f64),
        SessionScalar::Float(seconds) => Some(*seconds),
        SessionScalar::Boolean(value) => return value.to_string(),
        SessionScalar::String(value) => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("unlimited") {
                return "Unlimited".to_owned();
            }

            value
                .strip_suffix(" sec")
                .and_then(|seconds| seconds.parse::<f64>().ok())
                .or_else(|| value.parse::<f64>().ok())
        },
    };

    seconds
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map_or_else(
            || match value {
                SessionScalar::Integer(value) => value.to_string(),
                SessionScalar::Float(value) => value.to_string(),
                SessionScalar::Boolean(value) => value.to_string(),
                SessionScalar::String(value) => value.trim().to_owned(),
            },
            format_recording_duration,
        )
}

pub fn reset_reference_telemetry(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    state.reference_lap_choices = reference_session.map_or_else(Vec::new, |reference| {
        reference
            .laps()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, lap)| {
                LapChoice::new(
                    index,
                    lap.number(),
                    lap.duration_ms(),
                    None,
                    lap.is_complete(),
                )
            })
            .collect()
    });
    state.selected_reference_lap_index = reference_session.and_then(Session::preferred_lap_index);
    state.reference_ibt_error = None;
    if state.active_tab == DashboardTab::Telemetry {
        sync_telemetry(state, session, reference_session);
    }
}

fn selected_comparison<'a>(
    state: &DashboardState,
    session: &Session,
    reference_session: Option<&'a Session>,
) -> Option<(usize, &'a Session, usize)> {
    let lap = state.selected_lap_index?;
    let reference = reference_session?;
    let reference_lap = state.selected_reference_lap_index?;
    tracks_are_compatible(session, reference).then_some((lap, reference, reference_lap))
}

fn comparison_points_for(
    session: &Session,
    comparison: Option<(usize, &Session, usize)>,
    value: impl Fn(&TelemetrySample) -> f32,
) -> Vec<[f64; 2]> {
    comparison.map_or_else(Vec::new, |(lap, reference, reference_lap)| {
        session.comparison_points(lap, reference, reference_lap, value)
    })
}

fn comparison_points_for_optional(
    session: &Session,
    comparison: Option<(usize, &Session, usize)>,
    value: impl Fn(&TelemetrySample) -> Option<f32>,
) -> Vec<[f64; 2]> {
    comparison.map_or_else(Vec::new, |(lap, reference, reference_lap)| {
        session.comparison_points_optional(lap, reference, reference_lap, value)
    })
}

fn tracks_are_compatible(session: &Session, reference: &Session) -> bool {
    session
        .ibt_info()
        .zip(reference.ibt_info())
        .is_some_and(|(main, comparison)| {
            let track_matches = match (main.track_id, comparison.track_id) {
                (Some(main), Some(comparison)) => main == comparison,
                _ => main
                    .track_name
                    .trim()
                    .eq_ignore_ascii_case(comparison.track_name.trim()),
            };
            let config_matches = match (&main.track_config_name, &comparison.track_config_name) {
                (Some(main), Some(comparison)) => {
                    main.trim().eq_ignore_ascii_case(comparison.trim())
                },
                _ => true,
            };
            track_matches && config_matches
        })
}

fn cars_are_different(session: &Session, reference: &Session) -> bool {
    session
        .ibt_info()
        .zip(reference.ibt_info())
        .is_some_and(
            |(main, comparison)| match (main.car_id, comparison.car_id) {
                (Some(main), Some(comparison)) => main != comparison,
                _ => main
                    .car_name
                    .as_deref()
                    .zip(comparison.car_name.as_deref())
                    .is_some_and(|(main, comparison)| !main.eq_ignore_ascii_case(comparison)),
            },
        )
}

fn comparison_issue(session: &Session, reference: &Session) -> Option<&'static str> {
    if session.ibt_info().is_none() {
        Some("Main IBT required")
    } else if reference.ibt_info().is_none() {
        Some("Reference IBT unavailable")
    } else if !tracks_are_compatible(session, reference) {
        Some("Track mismatch")
    } else {
        None
    }
}

fn focus_at(state: &mut DashboardState, session: &Session, x: f64) {
    let uses_lap_distance = chart_uses_lap_distance(state, session);
    let focused = if uses_lap_distance {
        state.selected_lap_index.and_then(|lap| {
            session.focused_telemetry_at_position(lap, (x / LAP_DISTANCE_AXIS_MAX) as f32)
        })
    } else {
        session.focused_telemetry(state.selected_lap_index, x)
    };
    state.focus_x = focused.map(|point| {
        if uses_lap_distance {
            f64::from(point.sample.normalized_car_position) * LAP_DISTANCE_AXIS_MAX
        } else {
            point.elapsed_seconds
        }
    });
    state.focused = focused;
    let focused_index = focused.map(|point| point.point_index);
    state.speed_chart.set_focus_index(focused_index);
    state.pedal_chart.set_focus_index(focused_index);
    state.brake_pressure_chart.set_focus_index(focused_index);
    state.abs_chart.set_focus_index(focused_index);
    state.steering_chart.set_focus_index(focused_index);
    state.steering_torque_chart.set_focus_index(focused_index);
    state.rpm_chart.set_focus_index(focused_index);
    state.gear_chart.set_focus_index(focused_index);
    state.dynamics_chart.set_focus_index(focused_index);
    state.yaw_chart.set_focus_index(focused_index);
    state.wheel_slip_chart.set_focus_index(focused_index);
    state.tyre_chart.set_focus_index(focused_index);
    state.suspension_chart.set_focus_index(focused_index);
    state.fuel_chart.set_focus_index(focused_index);
    state.delta_chart.set_focus_index(focused_index);
}

pub fn view<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, DashboardMessage> {
    let content = match state.active_tab {
        DashboardTab::Telemetry => telemetry_view(state, session, reference_session, live_source),
        DashboardTab::CarSetup => car_setup_view(state, session, live_source),
    };

    container(content)
        .padding(DASHBOARD_CONTENT_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn car_setup_view<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, DashboardMessage> {
    let order = state.maximized_car_setup_card.as_ref().map_or_else(
        || current_car_setup_card_order(state),
        |card| vec![card.clone()],
    );
    let mut content = column![]
        .spacing(CAR_SETUP_CARD_SPACING)
        .width(Length::Fill);
    let mut section_run = Vec::new();

    for card in order {
        if setup_view::SetupViewData::card_is_full_width(&card) {
            if !section_run.is_empty() {
                content = content.push(car_setup_card_columns(
                    state,
                    session,
                    live_source,
                    std::mem::take(&mut section_run),
                ));
            }
            content = content.push(draggable_car_setup_card(state, session, live_source, card));
        } else {
            section_run.push(card);
        }
    }
    if !section_run.is_empty() {
        content = content.push(car_setup_card_columns(
            state,
            session,
            live_source,
            section_run,
        ));
    }

    scrollable(content)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn car_setup_card_columns<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
    cards: Vec<String>,
) -> Element<'a, DashboardMessage> {
    if cards.len() == 1 {
        return draggable_car_setup_card(state, session, live_source, cards[0].clone());
    }

    let mut columns: [Vec<String>; 2] = std::array::from_fn(|_| Vec::new());
    let mut weights = [0_usize; 2];
    for card in cards {
        let column = usize::from(weights[1] < weights[0]);
        weights[column] = weights[column].saturating_add(state.car_setup.card_weight(&card));
        columns[column].push(card);
    }
    let [left_cards, right_cards] = columns;
    let left = left_cards.into_iter().fold(
        column![]
            .spacing(CAR_SETUP_CARD_SPACING)
            .width(Length::Fill),
        |column, card| column.push(draggable_car_setup_card(state, session, live_source, card)),
    );
    let right = right_cards.into_iter().fold(
        column![]
            .spacing(CAR_SETUP_CARD_SPACING)
            .width(Length::Fill),
        |column, card| column.push(draggable_car_setup_card(state, session, live_source, card)),
    );

    row![left, right]
        .spacing(CAR_SETUP_CARD_SPACING)
        .align_y(Vertical::Top)
        .width(Length::Fill)
        .into()
}

fn draggable_car_setup_card<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
    card: String,
) -> Element<'a, DashboardMessage> {
    let maximized = state.maximized_car_setup_card.as_ref() == Some(&card);
    let collapsed = state
        .car_setup_card_collapsed
        .get(&card)
        .copied()
        .unwrap_or(false);
    let interaction = if state.dragging_car_setup_card.as_ref() == Some(&card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle: Element<'_, DashboardMessage> = if maximized {
        Space::new()
            .width(Length::Fixed(CARD_HEADER_HEIGHT))
            .height(Length::Fixed(CARD_HEADER_HEIGHT))
            .into()
    } else {
        card_drag_handle(
            DashboardMessage::BeginCarSetupCardDrag(card.clone()),
            interaction,
        )
    };
    let highlighted = state.dragging_car_setup_card.is_some()
        && (state.dragging_car_setup_card.as_ref() == Some(&card)
            || state.car_setup_drop_target.as_ref() == Some(&card));
    let card_content = pane_card_with_maximize(
        CardTitle::new(
            state.car_setup.card_title(session, &card),
            setup_view::SetupViewData::card_icon(&card).size(CARD_TITLE_ICON_SIZE),
        ),
        state.car_setup.card_content(session, live_source, &card),
        0.0,
        handle,
        maximized,
        collapsed,
        DashboardMessage::ToggleCarSetupCardMaximized(card.clone()),
        DashboardMessage::ToggleCarSetupCardCollapsed(card.clone()),
        highlighted,
    );
    let card_content: Element<'_, DashboardMessage> = if state.dragging_car_setup_card.as_ref()
        == Some(&card)
        && let (Some(origin), Some(cursor)) =
            (state.car_setup_drag_origin, state.car_setup_drag_cursor)
    {
        float(card_content)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        card_content
    };

    bounds_reporter(card, card_content, |card, bounds, visible_bounds| {
        DashboardMessage::CarSetupCardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        }
    })
}

/// Builds the Dashboard destinations for placement in the application title bar.
pub fn tab_bar(state: &DashboardState) -> Element<'_, DashboardMessage> {
    let active_tab = state.active_tab;

    container(
        tabs([
            tab(
                "Telemetry",
                active_tab == DashboardTab::Telemetry,
                DashboardMessage::SelectTab(DashboardTab::Telemetry),
            )
            .icon(lucide::activity().size(DASHBOARD_TAB_ICON_SIZE))
            .width(Length::Fixed(DASHBOARD_TAB_WIDTH)),
            tab(
                "Car setup",
                active_tab == DashboardTab::CarSetup,
                DashboardMessage::SelectTab(DashboardTab::CarSetup),
            )
            .icon(lucide::wrench().size(DASHBOARD_TAB_ICON_SIZE))
            .width(Length::Fixed(DASHBOARD_TAB_WIDTH)),
        ])
        .width(Length::Shrink),
    )
    .height(Length::Fill)
    .padding(
        Padding::ZERO
            .top(DASHBOARD_TAB_TOP_PADDING)
            .left(DASHBOARD_TAB_LEFT_PADDING),
    )
    .into()
}

fn telemetry_view<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, DashboardMessage> {
    let charts: Element<'_, DashboardMessage> = if let Some(chart) = state.maximized_chart {
        chart_view(state, chart)
    } else {
        let mut charts = Vec::new();
        for chart in state.chart_order.iter().copied() {
            if state.chart_visibility[chart.index()] {
                charts.push(chart_view(state, chart));
            }
        }
        let chart_grid = grid(charts)
            .columns(state.chart_columns.count())
            .spacing(16)
            .height(Length::Shrink);
        scrollable(chart_grid)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let (analysis_setup, lap_analysis) =
        analysis_panels(state, session, reference_session, live_source);

    row![analysis_setup, charts, lap_analysis]
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn chart_view<'a>(state: &'a DashboardState, chart: ChartId) -> Element<'a, DashboardMessage> {
    let focused_x = state.focus_x;
    let scroll_modifiers = state.modifiers;
    let content: Element<'a, DashboardMessage> = if should_build_chart_plot(state, chart) {
        match chart {
            ChartId::Speed => state
                .speed_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::SpeedPlot),
            ChartId::Pedal => state
                .pedal_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::PedalPlot),
            ChartId::BrakePressure => state
                .brake_pressure_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::BrakePressurePlot),
            ChartId::Abs => state
                .abs_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::AbsPlot),
            ChartId::Steering => state
                .steering_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::SteeringPlot),
            ChartId::SteeringTorque => state
                .steering_torque_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::SteeringTorquePlot),
            ChartId::Rpm => state
                .rpm_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::RpmPlot),
            ChartId::Gear => state
                .gear_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::GearPlot),
            ChartId::Dynamics => state
                .dynamics_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::DynamicsPlot),
            ChartId::Yaw => state
                .yaw_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::YawPlot),
            ChartId::WheelSlip => state
                .wheel_slip_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::WheelSlipPlot),
            ChartId::Tyre => state
                .tyre_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::TyrePlot),
            ChartId::Suspension => state
                .suspension_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::SuspensionPlot),
            ChartId::Fuel => state
                .fuel_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::FuelPlot),
            ChartId::Delta => state
                .delta_chart
                .view(focused_x, scroll_modifiers)
                .map(DashboardMessage::DeltaPlot),
        }
    } else {
        Space::new().width(Length::Fill).height(Length::Fill).into()
    };

    draggable_chart(state, chart, content)
}

fn should_build_chart_plot(state: &DashboardState, chart: ChartId) -> bool {
    !state.chart_collapsed[chart.index()] && state.dragging_chart != Some(chart)
}

fn draggable_chart<'a>(
    state: &DashboardState,
    chart: ChartId,
    content: impl Into<Element<'a, DashboardMessage>>,
) -> Element<'a, DashboardMessage> {
    let maximized = state.maximized_chart == Some(chart);
    let interaction = if state.dragging_chart == Some(chart) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle: Element<'_, DashboardMessage> = if maximized {
        Space::new()
            .width(Length::Fixed(CARD_HEADER_HEIGHT))
            .height(Length::Fixed(CARD_HEADER_HEIGHT))
            .into()
    } else {
        card_drag_handle(DashboardMessage::BeginChartDrag(chart), interaction)
    };
    let highlighted = state.dragging_chart.is_some()
        && state.dragging_chart != Some(chart)
        && state.drop_target == Some(chart);
    let card = chart_card(
        CardTitle::new(chart.title(), chart.icon()),
        content,
        handle,
        maximized,
        state.chart_collapsed[chart.index()],
        DashboardMessage::ToggleChartMaximized(chart),
        DashboardMessage::ToggleChartCollapsed(chart),
        highlighted || state.dragging_chart == Some(chart),
        0.0,
    );
    let card = if maximized {
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else if state.chart_collapsed[chart.index()] {
        container(card)
            .width(Length::Fill)
            .height(Length::Shrink)
            .into()
    } else {
        fixed_height(card)
    };
    let card: Element<'_, DashboardMessage> = if state.dragging_chart == Some(chart)
        && let (Some(origin), Some(cursor)) = (state.drag_origin, state.drag_cursor)
    {
        float(card)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        card
    };

    bounds_reporter(
        (chart, state.chart_layout_generation),
        card,
        |(chart, _), bounds, visible_bounds| DashboardMessage::ChartLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        },
    )
}

fn card_drag_handle(
    on_press: DashboardMessage,
    interaction: mouse::Interaction,
) -> Element<'static, DashboardMessage> {
    tooltip(
        mouse_area(container(lucide::grip_vertical().size(16)).padding(6))
            .on_press(on_press)
            .interaction(interaction),
        container(text("Drag to reorder").size(12)).padding([4, 8]),
        tooltip::Position::Top,
    )
    .gap(4)
    .padding(0)
    .style(icon_tooltip_style)
    .into()
}

fn analysis_panels<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> (Element<'a, DashboardMessage>, Element<'a, DashboardMessage>) {
    let sample = state
        .focused
        .map(|focused| focused.sample)
        .or_else(|| session.latest().copied());
    let steering_angle_max = session.latest_frame().and_then(|frame| {
        frame
            .get_optional(variables::chassis::STEERING_WHEEL_ANGLE_MAX)
            .ok()
            .flatten()
    });
    let reference_focused = state.focused.and_then(|focused| {
        selected_comparison(state, session, reference_session).and_then(
            |(_, reference, reference_lap)| {
                reference.focused_telemetry_at_position(
                    reference_lap,
                    focused.sample.normalized_car_position,
                )
            },
        )
    });
    let ibt_info = session.ibt_info();
    let metadata = &state.session_metadata;
    let (connection_label, connection_color) = if session.ibt_info().is_some() {
        ("IBT", STATUS_INFO)
    } else if !live_source.is_available() {
        ("Offline", STATUS_MUTED)
    } else {
        match session.connection() {
            ConnectionStatus::Disconnected => ("Disconnected", STATUS_MUTED),
            ConnectionStatus::Connecting => ("Waiting", STATUS_WARNING),
            ConnectionStatus::Connected => ("Live", STATUS_SUCCESS),
        }
    };
    let track_name = ibt_info
        .map(|info| info.track_name.as_str())
        .or(metadata.track_name.as_deref())
        .unwrap_or("Waiting for track data")
        .to_owned();
    let car_name = ibt_info
        .and_then(|info| info.car_name.as_deref())
        .or(metadata.car_name.as_deref())
        .unwrap_or("--")
        .to_owned();
    let session_type = metadata.session_type.as_deref().unwrap_or("--").to_owned();
    let time_label = ibt_info.map_or_else(
        || {
            metadata.session_time.clone().unwrap_or_else(|| {
                let samples = session.packets_received();
                format!("{samples} samples")
            })
        },
        |info| format_recording_duration(info.duration_seconds),
    );
    let track_config = ibt_info
        .and_then(|info| info.track_config_name.clone())
        .or_else(|| metadata.track_config.clone());
    let track_length = match (metadata.track_length.as_deref(), metadata.track_turns) {
        (Some(length), Some(turns)) => Some(format!("{length} · {turns} turns")),
        (Some(length), None) => Some(length.to_owned()),
        (None, Some(turns)) => Some(format!("{turns} turns")),
        (None, None) => None,
    };
    let source_detail = if session.ibt_info().is_some() {
        session.last_error().unwrap_or(connection_label).to_owned()
    } else if let Some(error) = session.last_error() {
        error.to_owned()
    } else if live_source.is_available() {
        live_source.display_name().to_owned()
    } else {
        format!("{} · unavailable", live_source.display_name())
    };

    let connection_action = if !live_source.is_available() {
        "Live unavailable"
    } else if session.wants_connection() {
        "Disconnect"
    } else {
        "Connect"
    };
    let connection_button = action_button(connection_action)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Medium)
        .width(Length::Fill)
        .height(Length::Fixed(38.0))
        .padding(5)
        .on_press_maybe(
            (live_source.is_available() && state.ibt_load_state == IbtLoadState::Idle)
                .then_some(DashboardMessage::ToggleConnection),
        );
    let open_label = match state.ibt_load_state {
        IbtLoadState::Idle => "Open IBT",
        IbtLoadState::Selecting => "Selecting...",
        IbtLoadState::Loading => "Loading...",
    };
    let open_button = icon_button(lucide::folder_open().size(17), open_label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Icon)
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(38.0))
        .padding(10)
        .on_press_maybe(
            (state.ibt_load_state == IbtLoadState::Idle).then_some(DashboardMessage::OpenIbt),
        );
    let reference_open_label = match state.reference_ibt_load_state {
        IbtLoadState::Idle => "Open IBT",
        IbtLoadState::Selecting => "Selecting...",
        IbtLoadState::Loading => "Loading...",
    };
    let reference_open_button = action_button(
        stack([
            container(lucide::folder_open().size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Left)
                .align_y(Vertical::Center)
                .into(),
            container(text(reference_open_label).size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .variant(ButtonVariant::Outline)
    .size(ButtonSize::Medium)
    .width(Length::Fill)
    .height(Length::Fixed(38.0))
    .padding(5)
    .on_press_maybe(
        (state.reference_ibt_load_state == IbtLoadState::Idle)
            .then_some(DashboardMessage::OpenReferenceIbt),
    );
    let can_clear_reference = state.reference_ibt_load_state == IbtLoadState::Idle
        && (reference_session.is_some() || state.reference_ibt_error.is_some());
    let clear_reference_button = icon_button(lucide::x().size(16), "Clear reference")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Icon)
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(38.0))
        .padding(8)
        .on_press_maybe(can_clear_reference.then_some(DashboardMessage::ClearReferenceIbt));
    let reference_description = state.reference_ibt_error.as_deref().map_or_else(
        || {
            reference_session.map_or_else(
                || "No reference loaded".to_owned(),
                |reference| {
                    reference.ibt_info().map_or_else(
                        || "Reference IBT unavailable".to_owned(),
                        |info| {
                            let mut details = vec![info.track_name.as_str()];
                            if let Some(config) = info.track_config_name.as_deref() {
                                details.push(config);
                            }
                            if let Some(car) = info.car_name.as_deref() {
                                details.push(car);
                            }
                            details.push(info.file_name.as_str());
                            let details = details.join(" · ");

                            if let Some(issue) = comparison_issue(session, reference) {
                                format!("{issue} · {details}")
                            } else if cars_are_different(session, reference) {
                                format!("Car mismatch · {details}")
                            } else {
                                details
                            }
                        },
                    )
                },
            )
        },
        str::to_owned,
    );
    let reference_color = if state.reference_ibt_error.is_some()
        || reference_session.is_some_and(|reference| comparison_issue(session, reference).is_some())
    {
        STATUS_ERROR
    } else if reference_session.is_some_and(|reference| cars_are_different(session, reference)) {
        STATUS_WARNING
    } else {
        STATUS_MUTED
    };

    let mut session_details = column![setup_separated_block(
        text(track_name)
            .size(20)
            .font(typography::SANS_SEMIBOLD)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )]
    .width(Length::Fill);
    for (label, value) in [
        ("Car", car_name),
        ("Class", metadata_value(metadata.car_class.clone())),
        ("Session", session_type),
        ("Time", time_label),
        ("Date", metadata_value(metadata.date_time.clone())),
        ("Layout", metadata_value(track_config)),
        ("Length", metadata_value(track_length)),
        ("Type", metadata_value(metadata.track_type.clone())),
    ] {
        session_details = session_details.push(metadata_row(label, value, None));
    }
    session_details = session_details
        .push(metadata_row(
            "Source",
            source_detail,
            Some(connection_color),
        ))
        .push(setup_section_heading("CONDITIONS"));
    for (label, value) in [
        ("Weather", metadata_value(metadata.weather.clone())),
        ("Air temp", metadata_value(metadata.air_temperature.clone())),
        (
            "Track temp",
            metadata_value(metadata.surface_temperature.clone()),
        ),
        ("Humidity", metadata_value(metadata.humidity.clone())),
        ("Wind", metadata_value(metadata.wind.clone())),
    ] {
        session_details = session_details.push(metadata_row(label, value, None));
    }

    let reference_laps = lap_choice_list(
        &state.reference_lap_choices,
        state.selected_reference_lap_index,
        DashboardMessage::SelectReferenceLap,
        false,
    );
    let analysis_laps = lap_choice_list(
        &state.lap_choices,
        state.selected_lap_index,
        DashboardMessage::SelectLap,
        true,
    );

    let session_content = column![
        session_details,
        setup_content_block(
            row![connection_button, open_button]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        ),
    ]
    .width(Length::Fill);
    let reference_controls = column![
        callout(
            text(reference_description)
                .size(14)
                .color(reference_color)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        )
        .padding(9)
        .width(Length::Fill),
        row![reference_open_button, clear_reference_button]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    ]
    .spacing(8)
    .width(Length::Fill);
    let reference_content =
        column![setup_separated_block(reference_controls), reference_laps].width(Length::Fill);
    let laps_content = column![
        setup_separated_block(
            callout(
                row![
                    text("Session laps")
                        .size(15)
                        .font(typography::SANS_SEMIBOLD)
                        .width(Length::Fill),
                    text(format_lap_count(state.lap_choices.len())).size(13),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([8, 9])
            .width(Length::Fill)
        ),
        analysis_laps,
    ]
    .width(Length::Fill);
    let mut charts_content = column![setup_separated_block(
        row![
            text("Layout")
                .size(14)
                .font(typography::SANS_SEMIBOLD)
                .width(Length::Fill),
            chart_columns_button(ChartColumns::One, state.chart_columns == ChartColumns::One,),
            chart_columns_button(ChartColumns::Two, state.chart_columns == ChartColumns::Two,),
            icon_button(lucide::rotate_ccw().size(16), "Reset layout")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(6)
                .on_press(DashboardMessage::ResetDashboardLayout),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )]
    .width(Length::Fill);
    for chart in state.chart_order.iter().copied() {
        charts_content = charts_content.push(chart_list_item(state, chart));
    }

    let mut setup_content: [Option<Element<'_, DashboardMessage>>; SetupCardId::COUNT] = [
        Some(session_content.into()),
        Some(reference_content.into()),
        Some(laps_content.into()),
        Some(charts_content.into()),
    ];
    let mut setup_cards = column![].spacing(12).width(Length::Fill);
    for card in state.setup_card_order.iter().copied() {
        let content = setup_content[card.index()]
            .take()
            .expect("every setup card has content");
        setup_cards = setup_cards.push(draggable_setup_card(state, card, content));
    }

    let mut analysis_cards = column![].spacing(12).width(Length::Fill);
    for card in state.lap_analysis_order.iter().copied() {
        analysis_cards = analysis_cards.push(lap_analysis_card_view(
            state,
            card,
            sample,
            steering_angle_max,
            state.focused,
            reference_focused,
        ));
    }

    let setup = container(
        scrollable(setup_cards)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(SETUP_PANEL_WIDTH))
    .height(Length::Fill);

    let analysis = container(
        scrollable(analysis_cards)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(ANALYSIS_PANEL_WIDTH))
    .height(Length::Fill);

    (setup.into(), analysis.into())
}

fn lap_analysis_card_view<'a>(
    state: &DashboardState,
    card: LapAnalysisCardId,
    sample: Option<TelemetrySample>,
    steering_angle_max: Option<f32>,
    focused: Option<FocusedTelemetry>,
    reference_focused: Option<FocusedTelemetry>,
) -> Element<'a, DashboardMessage> {
    draggable_lap_analysis_card(
        state,
        card,
        lap_analysis_card_content(card, sample, steering_angle_max, focused, reference_focused),
    )
}

fn lap_analysis_card_content(
    card: LapAnalysisCardId,
    current_sample: Option<TelemetrySample>,
    steering_angle_max: Option<f32>,
    focused: Option<FocusedTelemetry>,
    reference_focused: Option<FocusedTelemetry>,
) -> Element<'static, DashboardMessage> {
    let content = column![].width(Length::Fill);
    let sample = current_sample.unwrap_or_default();

    match card {
        LapAnalysisCardId::Cursor => {
            let focused_sample = focused.map(|point| point.sample);
            content
                .push(cursor_value(
                    "Time",
                    focused_sample.map_or_else(
                        || "--:--.---".to_owned(),
                        |sample| format_lap_time(sample.current_lap_ms),
                    ),
                    None,
                ))
                .push(cursor_value(
                    "Lap position",
                    focused_sample.map_or_else(
                        || "--".to_owned(),
                        |sample| format_track_position(sample.normalized_car_position),
                    ),
                    None,
                ))
                .into()
        },
        LapAnalysisCardId::ReferenceCursor => {
            let reference = reference_focused.map(|point| point.sample);
            content
                .push(cursor_value(
                    "Time",
                    reference.map_or_else(
                        || "--:--.---".to_owned(),
                        |sample| format_lap_time(sample.current_lap_ms),
                    ),
                    None,
                ))
                .push(cursor_value(
                    "Speed",
                    reference.map_or_else(
                        || "--".to_owned(),
                        |sample| format!("{:.1} km/h", sample.speed_kmh),
                    ),
                    Some(Color::from_rgb(0.18, 0.65, 0.95)),
                ))
                .push(input_badge_value(
                    "Throttle",
                    reference.and_then(|sample| input_pedal_value(sample.throttle)),
                    BadgeVariant::Success,
                    InputMeter::Linear,
                    THROTTLE_LINE_COLOR,
                ))
                .push(input_badge_value(
                    "Brake",
                    reference.and_then(|sample| input_pedal_value(sample.brake)),
                    BadgeVariant::Danger,
                    InputMeter::Linear,
                    BRAKE_LINE_COLOR,
                ))
                .into()
        },
        LapAnalysisCardId::Vehicle => content
            .push(cursor_value(
                "Speed",
                format!("{:.1} km/h", sample.speed_kmh),
                Some(Color::from_rgb(0.18, 0.65, 0.95)),
            ))
            .push(cursor_value("RPM", sample.rpm.max(0).to_string(), None))
            .push(cursor_value("Gear", format_gear(sample.gear), None))
            .push(cursor_value(
                "Fuel",
                format!("{:.1} L", sample.fuel_litres),
                None,
            ))
            .push(cursor_value(
                "Current lap",
                format_lap_time(sample.current_lap_ms),
                None,
            ))
            .push(cursor_value(
                "Last lap",
                format_lap_time(sample.last_lap_ms),
                None,
            ))
            .push(cursor_value(
                "Position",
                format_position(sample.position),
                None,
            ))
            .into(),
        LapAnalysisCardId::Inputs => {
            let readout = input_readout(current_sample, steering_angle_max);

            content
                .push(input_badge_value(
                    "Throttle",
                    readout.throttle,
                    BadgeVariant::Success,
                    InputMeter::Linear,
                    THROTTLE_LINE_COLOR,
                ))
                .push(input_badge_value(
                    "Brake",
                    readout.brake,
                    BadgeVariant::Danger,
                    InputMeter::Linear,
                    BRAKE_LINE_COLOR,
                ))
                .push(input_badge_value(
                    "Steering",
                    readout.steering,
                    BadgeVariant::Primary,
                    InputMeter::Centered,
                    STEERING_LINE_COLOR,
                ))
                .into()
        },
        LapAnalysisCardId::Dynamics => content
            .push(cursor_value(
                "Lat G",
                format!("{:.2} G", sample.acceleration_g[0]),
                Some(Color::from_rgb(0.23, 0.55, 0.95)),
            ))
            .push(cursor_value(
                "Long G",
                format!("{:.2} G", sample.acceleration_g[1]),
                Some(Color::from_rgb(0.95, 0.55, 0.18)),
            ))
            .push(cursor_value(
                "Yaw rate",
                format!("{:+.1}°/s", sample.yaw_rate_rad_s.to_degrees()),
                Some(Color::from_rgb(0.72, 0.34, 0.95)),
            ))
            .into(),
        LapAnalysisCardId::Tyres => tyre_status::view(current_sample),
        LapAnalysisCardId::Wheels => ["FL", "FR", "RL", "RR"]
            .into_iter()
            .enumerate()
            .fold(content, |content, (wheel, label)| {
                content
                    .push(cursor_value(
                        label,
                        format!("{:+.1}% slip", sample.wheel_slip[wheel] * 100.0),
                        None,
                    ))
                    .push(cursor_value(
                        "Travel",
                        format!("{:.1} mm", sample.suspension_travel_m[wheel] * 1_000.0),
                        None,
                    ))
            })
            .into(),
    }
}

fn draggable_lap_analysis_card<'a>(
    state: &DashboardState,
    card: LapAnalysisCardId,
    content: impl Into<Element<'a, DashboardMessage>>,
) -> Element<'a, DashboardMessage> {
    let interaction = if state.dragging_lap_analysis_card == Some(card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle = card_drag_handle(DashboardMessage::BeginLapAnalysisDrag(card), interaction);
    let highlighted = state.dragging_lap_analysis_card.is_some()
        && (state.dragging_lap_analysis_card == Some(card)
            || state.lap_analysis_drop_target == Some(card));
    let card_content = pane_card(
        CardTitle::new(card.title(), card.icon()),
        content,
        0.0,
        handle,
        state.lap_analysis_collapsed[card.index()],
        DashboardMessage::ToggleLapAnalysisCardCollapsed(card),
        highlighted,
    );
    let card_content: Element<'_, DashboardMessage> = if state.dragging_lap_analysis_card
        == Some(card)
        && let (Some(origin), Some(cursor)) = (
            state.lap_analysis_drag_origin,
            state.lap_analysis_drag_cursor,
        ) {
        float(card_content)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        card_content
    };

    bounds_reporter(card, card_content, |card, bounds, visible_bounds| {
        DashboardMessage::LapAnalysisLayoutChanged {
            card,
            bounds,
            visible_bounds,
        }
    })
}

fn draggable_setup_card<'a>(
    state: &DashboardState,
    card: SetupCardId,
    content: impl Into<Element<'a, DashboardMessage>>,
) -> Element<'a, DashboardMessage> {
    let interaction = if state.dragging_setup_card == Some(card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle = card_drag_handle(DashboardMessage::BeginSetupCardDrag(card), interaction);
    let highlighted = state.dragging_setup_card.is_some()
        && (state.dragging_setup_card == Some(card) || state.setup_card_drop_target == Some(card));
    let card_content = pane_card(
        CardTitle::new(card.title(), card.icon()),
        content,
        0.0,
        handle,
        state.setup_card_collapsed[card.index()],
        DashboardMessage::ToggleSetupCardCollapsed(card),
        highlighted,
    );
    let card_content: Element<'_, DashboardMessage> = if state.dragging_setup_card == Some(card)
        && let (Some(origin), Some(cursor)) =
            (state.setup_card_drag_origin, state.setup_card_drag_cursor)
    {
        float(card_content)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        card_content
    };

    bounds_reporter(card, card_content, |card, bounds, visible_bounds| {
        DashboardMessage::SetupCardLayoutChanged {
            card,
            bounds,
            visible_bounds,
        }
    })
}

fn metadata_value(value: Option<String>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "--".to_owned())
}

fn metadata_row(
    label: &'static str,
    value: String,
    accent: Option<Color>,
) -> Element<'static, DashboardMessage> {
    let value = text(value)
        .size(14)
        .font(typography::SANS_SEMIBOLD)
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph);
    let value = match accent {
        Some(color) => value.color(color),
        None => value,
    };

    let label = container(text(label).size(12).color(TEXT_SECONDARY))
        .padding([0.0, DATA_TEXT_INSET])
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .align_y(Vertical::Center);
    let value = container(value)
        .padding([0.0, DATA_TEXT_INSET])
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Center);
    let cells = row![
        label,
        rule::vertical(DATA_SEPARATOR_WIDTH).style(data_separator_style),
        value,
    ]
    .width(Length::Fill)
    .height(Length::Fixed(
        SESSION_METADATA_ROW_HEIGHT - DATA_SEPARATOR_WIDTH,
    ));

    column![
        cells,
        rule::horizontal(DATA_SEPARATOR_WIDTH).style(data_separator_style),
    ]
    .width(Length::Fill)
    .height(Length::Fixed(SESSION_METADATA_ROW_HEIGHT))
    .into()
}

fn setup_section_heading(label: &'static str) -> Element<'static, DashboardMessage> {
    setup_separated_block(
        text(label)
            .size(12)
            .font(typography::SANS_SEMIBOLD)
            .color(TEXT_SECONDARY)
            .width(Length::Fill),
    )
}

fn setup_content_block<'a>(
    content: impl Into<Element<'a, DashboardMessage>>,
) -> Element<'a, DashboardMessage> {
    container(content)
        .padding(DATA_TEXT_INSET)
        .width(Length::Fill)
        .into()
}

fn setup_separated_block<'a>(
    content: impl Into<Element<'a, DashboardMessage>>,
) -> Element<'a, DashboardMessage> {
    column![
        setup_content_block(content),
        rule::horizontal(DATA_SEPARATOR_WIDTH).style(data_separator_style),
    ]
    .width(Length::Fill)
    .into()
}

fn setup_control_row<'a>(
    content: impl Into<Element<'a, DashboardMessage>>,
    highlighted: bool,
) -> Element<'a, DashboardMessage> {
    let content = container(content)
        .padding([0.0, DATA_TEXT_INSET])
        .width(Length::Fill)
        .height(Length::Fixed(DATA_ROW_HEIGHT - DATA_SEPARATOR_WIDTH))
        .align_y(Vertical::Center)
        .style(move |theme| {
            if highlighted {
                iced::widget::container::Style::default()
                    .background(theme.extended_palette().background.weak.color)
            } else {
                iced::widget::container::Style::default()
            }
        });

    column![
        content,
        rule::horizontal(DATA_SEPARATOR_WIDTH).style(data_separator_style),
    ]
    .width(Length::Fill)
    .height(Length::Fixed(DATA_ROW_HEIGHT))
    .into()
}

fn chart_list_item<'a>(state: &'a DashboardState, chart: ChartId) -> Element<'a, DashboardMessage> {
    let dragging = state.dragging_chart_list_item == Some(chart);
    let interaction = if dragging {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let highlighted = dragging || state.chart_list_drop_target == Some(chart);
    let content = row![
        checkbox(state.chart_visibility[chart.index()])
            .label(chart.title())
            .size(16)
            .spacing(8)
            .text_size(14)
            .width(Length::Fill)
            .style(checkbox_style)
            .on_toggle(move |visible| DashboardMessage::ToggleChart(chart, visible)),
        card_drag_handle(DashboardMessage::BeginChartListDrag(chart), interaction),
    ]
    .align_y(iced::Alignment::Center);
    let item = setup_control_row(content, highlighted);
    let item: Element<'_, DashboardMessage> = if dragging
        && let (Some(origin), Some(cursor)) =
            (state.chart_list_drag_origin, state.chart_list_drag_cursor)
    {
        float(item)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .into()
    } else {
        item
    };

    bounds_reporter(
        (chart, state.chart_list_layout_generation),
        item,
        |(chart, _), bounds, visible_bounds| DashboardMessage::ChartListLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        },
    )
}

fn chart_columns_button(columns: ChartColumns, active: bool) -> Element<'static, DashboardMessage> {
    let icon = match columns {
        ChartColumns::One => lucide::layout_list(),
        ChartColumns::Two => lucide::layout_grid(),
    }
    .size(16);
    let label = match columns {
        ChartColumns::One => "Single column",
        ChartColumns::Two => "Two columns",
    };

    icon_toggle_button(icon, label, active)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(6)
        .on_press(DashboardMessage::SetChartColumns(columns))
        .into()
}

fn cursor_value<'a>(
    label: &'static str,
    value: String,
    accent: Option<Color>,
) -> Element<'a, DashboardMessage> {
    let value = match accent {
        Some(color) => text(value)
            .size(18)
            .font(typography::MONO_SEMIBOLD)
            .color(color),
        None => text(value).size(18).font(typography::MONO_SEMIBOLD),
    };

    analysis_data_row(label, value.into())
}

fn input_badge_value(
    label: &'static str,
    value: Option<InputMeterValue>,
    variant: BadgeVariant,
    meter: InputMeter,
    meter_color: Color,
) -> Element<'static, DashboardMessage> {
    let badge = match value {
        Some(value) => {
            let badge = badge(value.text).variant(variant).meter_color(meter_color);
            match meter {
                InputMeter::Linear => badge.progress(value.progress.unwrap_or(0.0)),
                InputMeter::Centered => badge.centered_progress(value.progress.unwrap_or(0.0)),
            }
        },
        None => badge("N/A").variant(BadgeVariant::Neutral),
    };

    analysis_data_row(
        label,
        badge
            .font(typography::MONO_SEMIBOLD)
            .width(Length::Fixed(INPUT_BADGE_WIDTH))
            .into(),
    )
}

fn analysis_data_row<'a>(
    label: &'static str,
    value: Element<'a, DashboardMessage>,
) -> Element<'a, DashboardMessage> {
    let label = container(text(label).size(12).color(TEXT_SECONDARY))
        .padding([0.0, DATA_TEXT_INSET])
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .align_y(Vertical::Center);
    let value = container(value)
        .padding([0.0, DATA_TEXT_INSET])
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Center);
    let cells = row![
        label,
        rule::vertical(DATA_SEPARATOR_WIDTH).style(data_separator_style),
        value,
    ]
    .width(Length::Fill)
    .height(Length::Fixed(DATA_ROW_HEIGHT - DATA_SEPARATOR_WIDTH));

    column![
        cells,
        rule::horizontal(DATA_SEPARATOR_WIDTH).style(data_separator_style),
    ]
    .width(Length::Fill)
    .height(Length::Fixed(DATA_ROW_HEIGHT))
    .into()
}

fn data_separator_style(theme: &iced::Theme) -> rule::Style {
    rule::Style {
        color: theme.extended_palette().background.weaker.color,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn fixed_height<'a>(content: Element<'a, DashboardMessage>) -> Element<'a, DashboardMessage> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(CHART_HEIGHT))
        .into()
}

fn build_speed_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.18, 0.65, 0.95);
    let speed = LineSeries::new(
        placeholder(),
        "Speed",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );
    let reference = reference_line("Reference speed", REFERENCE_AMBER);

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -40.0, 420.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.1} km/h")),
        ),
        [speed, reference],
        x_axis_link,
    )
}

fn build_pedal_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let throttle = LineSeries::new(
        placeholder(),
        "Throttle",
        THROTTLE_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );
    let brake = LineSeries::new(
        placeholder(),
        "Brake",
        BRAKE_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -15.0, 110.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.1}%")),
        ),
        [
            throttle,
            brake,
            reference_line("Reference throttle", REFERENCE_CYAN),
            reference_line("Reference brake", REFERENCE_PINK),
        ],
        x_axis_link,
    )
}

fn build_brake_pressure_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(BRAKE_PRESSURE_CHART_LINE_WIDTH));

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", 0.0, 120.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.1} bar")),
        ),
        series,
        x_axis_link,
    )
}

fn build_abs_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let abs = LineSeries::new(
        placeholder(),
        "ABS active",
        STATUS_WARNING,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", 0.0, 100.0, format_abs_activity)
                .with_value_formatter(format_abs_activity)
                .with_tick_count(2),
        ),
        [abs],
        x_axis_link,
    )
}

fn build_steering_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let steering = LineSeries::new(
        placeholder(),
        "Steering angle",
        STEERING_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -180.0, 180.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:+.1}°")),
        ),
        [
            steering,
            reference_line("Reference steering", REFERENCE_LAVENDER),
        ],
        x_axis_link,
    )
}

fn build_steering_torque_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let torque = LineSeries::new(
        placeholder(),
        "Steering torque",
        STEERING_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -30.0, 30.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:+.1} N·m")),
        ),
        [
            torque,
            reference_line("Reference steering torque", REFERENCE_LAVENDER),
        ],
        x_axis_link,
    )
}

fn build_rpm_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.92, 0.46, 0.18);
    let rpm = LineSeries::new(
        placeholder(),
        "RPM",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", 0.0, 8_000.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.0} rpm")),
        ),
        [rpm, reference_line("Reference RPM", REFERENCE_CYAN)],
        x_axis_link,
    )
}

fn build_gear_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.55, 0.58, 0.65);
    let gear = LineSeries::new(
        placeholder(),
        "Gear",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -1.0, 8.0, format_gear_axis)
                .with_value_formatter(format_gear_axis)
                .with_tick_count(11),
        ),
        [gear, reference_line("Reference gear", REFERENCE_AMBER)],
        x_axis_link,
    )
}

fn build_dynamics_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let lateral_color = Color::from_rgb(0.23, 0.55, 0.95);
    let longitudinal_color = Color::from_rgb(0.95, 0.55, 0.18);
    let lateral = LineSeries::new(
        placeholder(),
        "Lateral G",
        lateral_color,
        LineStyle::solid().with_pixel_width(DYNAMICS_CHART_LINE_WIDTH),
    );
    let longitudinal = LineSeries::new(
        placeholder(),
        "Longitudinal G",
        longitudinal_color,
        LineStyle::solid().with_pixel_width(DYNAMICS_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -3.6, 3.6, |value| format!("{value:.1}"))
                .with_value_formatter(|value| format!("{value:.2} G")),
        ),
        [
            lateral,
            longitudinal,
            reference_line("Reference lateral G", REFERENCE_LAVENDER),
            reference_line("Reference longitudinal G", REFERENCE_CYAN),
        ],
        x_axis_link,
    )
}

fn build_yaw_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.72, 0.34, 0.95);
    let yaw = LineSeries::new(
        placeholder(),
        "Yaw rate",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -60.0, 60.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:+.1}°/s")),
        ),
        [yaw, reference_line("Reference yaw rate", REFERENCE_AMBER)],
        x_axis_link,
    )
}

fn build_wheel_slip_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_CHART_LINE_WIDTH));
    let reference = reference_wheel_series();

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -20.0, 20.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:+.1}%")),
        ),
        series.into_iter().chain(reference),
        x_axis_link,
    )
}

fn build_tyre_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_CHART_LINE_WIDTH));
    let reference = reference_wheel_series();

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -15.0, 165.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.1}°C")),
        ),
        series.into_iter().chain(reference),
        x_axis_link,
    )
}

fn build_suspension_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_CHART_LINE_WIDTH));
    let reference = reference_wheel_series();

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -20.0, 120.0, |value| format!("{value:.0}"))
                .with_value_formatter(|value| format!("{value:.1} mm")),
        ),
        series.into_iter().chain(reference),
        x_axis_link,
    )
}

fn build_fuel_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.84, 0.65, 0.16);
    let fuel = LineSeries::new(
        placeholder(),
        "Fuel used",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", 0.0, 1.0, |value| format!("{value:.1}"))
                .with_value_formatter(|value| format!("{value:.3} L")),
        ),
        [fuel, reference_line("Reference fuel used", REFERENCE_CYAN)],
        x_axis_link,
    )
}

fn build_delta_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let delta = LineSeries::new(
        placeholder(),
        "Delta",
        Color::from_rgb(0.72, 0.34, 0.95),
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("", -6.0, 6.0, |value| format!("{value:+.1}"))
                .with_value_formatter(|value| format!("{value:+.3}s")),
        ),
        [delta],
        x_axis_link,
    )
}

fn time_axis() -> AxisSpec {
    AxisSpec::new("Time", 0.0, HISTORY_WINDOW.as_secs_f64(), format_chart_time)
}

fn lap_distance_axis() -> AxisSpec {
    AxisSpec::new(
        "Lap distance",
        0.0,
        LAP_DISTANCE_AXIS_MAX,
        format_lap_distance,
    )
}

fn placeholder() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0]]
}

fn wheel_series(style: LineStyle) -> [LineSeries; 4] {
    [
        ("Front left", Color::from_rgb(0.18, 0.65, 0.95)),
        ("Front right", Color::from_rgb(0.35, 0.78, 0.65)),
        ("Rear left", Color::from_rgb(0.95, 0.62, 0.20)),
        ("Rear right", Color::from_rgb(0.90, 0.32, 0.38)),
    ]
    .map(|(label, color)| LineSeries::new(placeholder(), label, color, style))
}

fn reference_wheel_series() -> [LineSeries; 4] {
    [
        ("Reference front left", REFERENCE_LAVENDER),
        ("Reference front right", REFERENCE_CYAN),
        ("Reference rear left", REFERENCE_AMBER),
        ("Reference rear right", REFERENCE_PINK),
    ]
    .map(|(label, color)| reference_line(label, color))
}

fn reference_line(label: &'static str, color: Color) -> LineSeries {
    LineSeries::new(
        placeholder(),
        label,
        color,
        LineStyle::solid().with_pixel_width(REFERENCE_CHART_LINE_WIDTH),
    )
}

fn format_gear_axis(value: f64) -> String {
    format_gear(value.round() as i32)
}

fn format_abs_activity(value: f64) -> String {
    if value >= 50.0 {
        "Active".to_owned()
    } else {
        "Off".to_owned()
    }
}

fn format_gear(gear: i32) -> String {
    match gear {
        -1 => "R".to_owned(),
        0 => "N".to_owned(),
        gear if gear > 0 => gear.to_string(),
        _ => "—".to_owned(),
    }
}

fn format_position(position: i32) -> String {
    if position > 0 {
        format!("P{position}")
    } else {
        "—".to_owned()
    }
}

fn format_track_position(position: f32) -> String {
    if position < 0.0 {
        "—".to_owned()
    } else {
        format!("{:.1}%", position.clamp(0.0, 1.0) * 100.0)
    }
}

fn pedal_percent(value: f32) -> f32 {
    value.clamp(0.0, 1.0) * 100.0
}

fn abs_activity_percent(active: bool) -> f32 {
    if active { 100.0 } else { 0.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMeter {
    Linear,
    Centered,
}

#[derive(Debug, Clone, PartialEq)]
struct InputMeterValue {
    text: String,
    progress: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct InputReadout {
    throttle: Option<InputMeterValue>,
    brake: Option<InputMeterValue>,
    steering: Option<InputMeterValue>,
}

fn input_readout(sample: Option<TelemetrySample>, steering_angle_max: Option<f32>) -> InputReadout {
    let Some(sample) = sample else {
        return InputReadout::default();
    };

    InputReadout {
        throttle: input_pedal_value(sample.throttle),
        brake: input_pedal_value(sample.brake),
        steering: sample.steering_angle.is_finite().then(|| InputMeterValue {
            text: format!("{:.1}°", sample.steering_angle.to_degrees()),
            progress: steering_angle_max
                .and_then(|maximum| steering_meter_progress(sample.steering_angle, maximum)),
        }),
    }
}

fn input_pedal_value(value: f32) -> Option<InputMeterValue> {
    if !value.is_finite() {
        return None;
    }

    let progress = value.clamp(0.0, 1.0);
    Some(InputMeterValue {
        text: format!("{:.1}%", progress * 100.0),
        progress: Some(progress),
    })
}

fn steering_meter_progress(angle: f32, maximum: f32) -> Option<f32> {
    let maximum = maximum.abs();
    if !angle.is_finite() || !maximum.is_finite() || maximum <= f32::EPSILON {
        return None;
    }
    let maximum = maximum.min(STEERING_METER_HALF_RANGE_RADIANS);

    // The centered Badge uses negative values for left and positive values for
    // right. Invert the SDK angle only for the visual meter so the fill follows
    // the physical steering direction while the displayed number stays raw.
    Some((-angle / maximum).clamp(-1.0, 1.0))
}

fn symmetric_y_limits(values: impl Iterator<Item = f64>, minimum_half_range: f64) -> (f64, f64) {
    let maximum = values
        .filter(|value| value.is_finite())
        .map(f64::abs)
        .fold(0.0, f64::max);
    let padded = maximum * 1.2;
    let half_range = if padded.is_finite() {
        padded.max(minimum_half_range)
    } else {
        minimum_half_range
    };

    (-half_range, half_range)
}

fn maximum_y(points: &[[f64; 2]], minimum: f64) -> f64 {
    points
        .iter()
        .map(|point| point[1])
        .filter(|value| value.is_finite())
        .fold(minimum, f64::max)
}

fn padded_y_limits(values: impl Iterator<Item = f64>, minimum_limits: (f64, f64)) -> (f64, f64) {
    let (minimum, maximum) = values.filter(|value| value.is_finite()).fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(minimum, maximum), value| (minimum.min(value), maximum.max(value)),
    );
    if !minimum.is_finite() || !maximum.is_finite() {
        return minimum_limits;
    }

    let span = (maximum - minimum).max(1.0);
    let padding = span * 0.1;
    (
        (minimum - padding).min(minimum_limits.0),
        (maximum + padding).max(minimum_limits.1),
    )
}

fn format_lap_count(lap_count: usize) -> String {
    if lap_count == 1 {
        "1 lap".to_owned()
    } else {
        format!("{lap_count} laps")
    }
}

fn format_recording_duration(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).round() as u64;
    let hours = total_seconds / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn format_chart_time(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).round() as u64;
    let hours = total_seconds / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:02}")
    } else {
        format!("{seconds}s")
    }
}

fn format_lap_distance(chart_position: f64) -> String {
    let percentage = chart_position / LAP_DISTANCE_AXIS_MAX * 100.0;
    if percentage == 0.0 {
        "0%".to_owned()
    } else {
        format!("{percentage:.0}%")
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use chiaro_actions::Action;
    use chiaro_irsdk::{
        OptionalTelemetryValues, SessionInfo, SessionScalar, TelemetryFrame, TelemetrySample,
        TelemetryValue,
    };
    use chiaro_telemetry::{
        IbtInfo, LAP_DISTANCE_AXIS_MAX, LoadedIbt, RecordingSource, Session, TimedSample,
    };
    use chiaro_time_series_chart::TimeSeriesMessage;
    use iced::{Point, Rectangle, Theme, keyboard, mouse, widget::rule};

    use super::{
        CardLayout, ChartColumns, ChartId, DASHBOARD_CONTENT_PADDING, DASHBOARD_TAB_LEFT_PADDING,
        DASHBOARD_TAB_TOP_PADDING, DATA_ROW_HEIGHT, DashboardLayout, DashboardLayoutFlag,
        DashboardMessage, DashboardState, DashboardTab, LapAnalysisCardId, LiveChartNavigation,
        SetupCardId, TelemetrySyncScope, chart_sync_targets, current_car_setup_card_order,
        data_separator_style, focus_at, format_chart_time, format_gear, format_lap_distance,
        format_lap_time, format_position, format_recording_duration, format_session_time,
        format_track_position, input_readout, metadata_value, move_item_to, pedal_percent,
        reset_reference_telemetry, reset_telemetry as reset_dashboard_telemetry,
        select_drop_target, selected_comparison, session_metadata, should_build_chart_plot,
        steering_meter_progress, symmetric_y_limits, sync_telemetry, update as update_dashboard,
        update_chart_focus,
    };

    #[test]
    fn data_rows_use_only_a_thin_surface_separator() {
        let theme = Theme::Dark;
        let palette = theme.extended_palette();
        let separator = data_separator_style(&theme);

        assert_eq!(separator.color, palette.background.weaker.color);
        assert_eq!(separator.fill_mode, rule::FillMode::Full);
        assert_eq!(separator.radius, 0.0.into());
    }

    #[test]
    fn title_bar_tabs_align_with_the_dashboard_content_edge() {
        assert_eq!(DASHBOARD_TAB_TOP_PADDING, 4.0);
        assert_eq!(DASHBOARD_TAB_LEFT_PADDING, DASHBOARD_CONTENT_PADDING);
        assert_eq!(DASHBOARD_TAB_LEFT_PADDING, 24.0);
    }

    #[test]
    fn dashboard_starts_on_the_telemetry_tab() {
        let state = DashboardState::default();
        assert_eq!(state.active_tab(), DashboardTab::Telemetry);
        assert_eq!(state.chart_order, ChartId::DEFAULT_ORDER);
        assert!(state.chart_visibility[ChartId::Speed.index()]);
        assert!(state.chart_visibility[ChartId::Pedal.index()]);
        assert!(state.chart_visibility[ChartId::Steering.index()]);
        for chart in ChartId::ALL {
            assert_eq!(
                state.chart_visibility[chart.index()],
                ChartId::DEFAULT_VISIBLE.contains(&chart)
            );
        }
        assert_eq!(state.layout_snapshot(), DashboardLayout::default());
    }

    #[test]
    fn persisted_layout_keys_are_stable_and_round_trip() {
        for chart in ChartId::ALL {
            assert_eq!(ChartId::from_key(chart.key()), Some(chart));
        }
        for card in LapAnalysisCardId::ALL {
            assert_eq!(LapAnalysisCardId::from_key(card.key()), Some(card));
        }
        for card in SetupCardId::ALL {
            assert_eq!(SetupCardId::from_key(card.key()), Some(card));
        }
        assert_eq!(ChartId::from_key("future_chart"), None);
        assert_eq!(LapAnalysisCardId::from_key("future_card"), None);
        assert_eq!(SetupCardId::from_key("future_card"), None);
    }

    #[test]
    fn imported_layout_normalizes_unknown_duplicate_and_missing_values() {
        let mut state = DashboardState {
            layout_revision: 41,
            maximized_chart: Some(ChartId::Speed),
            dragging_chart: Some(ChartId::Speed),
            dragging_lap_analysis_card: Some(LapAnalysisCardId::Cursor),
            dragging_setup_card: Some(SetupCardId::Session),
            ..DashboardState::default()
        };
        state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
            bounds: Rectangle::default(),
            visible_bounds: Some(Rectangle::default()),
        });
        state.chart_list_layouts[ChartId::Speed.index()] =
            state.chart_layouts[ChartId::Speed.index()];
        state.lap_analysis_layouts[LapAnalysisCardId::Cursor.index()] =
            state.chart_layouts[ChartId::Speed.index()];
        state.setup_card_layouts[SetupCardId::Session.index()] =
            state.chart_layouts[ChartId::Speed.index()];

        let layout = DashboardLayout {
            chart_order: ["fuel", "future_chart", "fuel", "speed"]
                .map(str::to_owned)
                .to_vec(),
            chart_visibility: vec![
                DashboardLayoutFlag {
                    key: "future_chart".to_owned(),
                    value: true,
                },
                DashboardLayoutFlag {
                    key: "speed".to_owned(),
                    value: false,
                },
                DashboardLayoutFlag {
                    key: "speed".to_owned(),
                    value: true,
                },
                DashboardLayoutFlag {
                    key: "abs".to_owned(),
                    value: true,
                },
            ],
            chart_collapsed: vec![
                DashboardLayoutFlag {
                    key: "fuel".to_owned(),
                    value: true,
                },
                DashboardLayoutFlag {
                    key: "fuel".to_owned(),
                    value: false,
                },
            ],
            chart_columns: 99,
            setup_card_order: ["charts", "future_card", "charts"]
                .map(str::to_owned)
                .to_vec(),
            setup_card_collapsed: vec![DashboardLayoutFlag {
                key: "reference".to_owned(),
                value: true,
            }],
            lap_analysis_order: ["wheels", "cursor", "wheels"].map(str::to_owned).to_vec(),
            lap_analysis_collapsed: vec![DashboardLayoutFlag {
                key: "inputs".to_owned(),
                value: true,
            }],
            car_setup_card_order: ["setup:section:Tires", "", "setup:section:Tires"]
                .map(str::to_owned)
                .to_vec(),
            car_setup_card_collapsed: vec![
                DashboardLayoutFlag {
                    key: "setup:section:Tires".to_owned(),
                    value: true,
                },
                DashboardLayoutFlag {
                    key: "setup:section:Tires".to_owned(),
                    value: false,
                },
            ],
        };

        state.apply_layout(&layout);

        assert_eq!(state.layout_revision(), 41);
        assert_eq!(
            &state.chart_order[..4],
            &[
                ChartId::Fuel,
                ChartId::Speed,
                ChartId::Delta,
                ChartId::Pedal
            ]
        );
        assert_eq!(state.chart_order.len(), ChartId::COUNT);
        assert!(!state.chart_visibility[ChartId::Speed.index()]);
        assert!(state.chart_visibility[ChartId::Abs.index()]);
        assert!(state.chart_visibility[ChartId::Pedal.index()]);
        assert!(state.chart_visibility[ChartId::Steering.index()]);
        assert!(!state.chart_visibility[ChartId::Fuel.index()]);
        assert!(state.chart_collapsed[ChartId::Fuel.index()]);
        assert_eq!(state.chart_columns, ChartColumns::One);
        assert_eq!(
            state.setup_card_order,
            [
                SetupCardId::Charts,
                SetupCardId::Session,
                SetupCardId::Reference,
                SetupCardId::Laps,
            ]
        );
        assert!(state.setup_card_collapsed[SetupCardId::Reference.index()]);
        assert_eq!(
            state.lap_analysis_order,
            [
                LapAnalysisCardId::Wheels,
                LapAnalysisCardId::Cursor,
                LapAnalysisCardId::ReferenceCursor,
                LapAnalysisCardId::Vehicle,
                LapAnalysisCardId::Inputs,
                LapAnalysisCardId::Dynamics,
                LapAnalysisCardId::Tyres,
            ]
        );
        assert!(state.lap_analysis_collapsed[LapAnalysisCardId::Inputs.index()]);
        assert_eq!(
            state.car_setup_card_order,
            ["setup:section:Tires".to_owned()]
        );
        assert_eq!(
            state.car_setup_card_collapsed.get("setup:section:Tires"),
            Some(&true)
        );
        assert_eq!(state.maximized_chart, None);
        assert!(!state.is_dragging_card());
        assert!(state.chart_layouts.iter().all(Option::is_none));
        assert!(state.chart_list_layouts.iter().all(Option::is_none));
        assert!(state.lap_analysis_layouts.iter().all(Option::is_none));
        assert!(state.setup_card_layouts.iter().all(Option::is_none));
    }

    #[test]
    fn layout_revision_changes_only_for_persisted_edits_and_explicit_reset() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let default_layout = state.layout_snapshot();

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Speed, true),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::SetChartColumns(ChartColumns::One),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        update(&mut state, &session, DashboardMessage::FinishCardDrag);
        state.apply_layout(&default_layout);
        assert_eq!(state.layout_revision(), 0);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Speed, false),
        );
        assert_eq!(state.layout_revision(), 1);
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Speed, false),
        );
        assert_eq!(state.layout_revision(), 1);
        update(
            &mut state,
            &session,
            DashboardMessage::SetChartColumns(ChartColumns::Two),
        );
        assert_eq!(state.layout_revision(), 2);
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleLapAnalysisCardCollapsed(LapAnalysisCardId::Cursor),
        );
        assert_eq!(state.layout_revision(), 3);
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleSetupCardCollapsed(SetupCardId::Session),
        );
        assert_eq!(state.layout_revision(), 4);

        assert!(DashboardMessage::ResetDashboardLayout.resets_layout());
        assert!(!DashboardMessage::Refresh.resets_layout());
        update(&mut state, &session, DashboardMessage::ResetDashboardLayout);
        assert_eq!(state.layout_revision(), 5);
        assert_eq!(state.layout_snapshot(), DashboardLayout::default());
        update(&mut state, &session, DashboardMessage::ResetDashboardLayout);
        assert_eq!(state.layout_revision(), 6);
    }

    #[test]
    fn optional_vehicle_channels_feed_their_charts_without_zero_fallbacks() {
        let started_at = Instant::now();
        let mut session = Session::default();
        for (offset, pressure, abs, torque) in [
            (0, [80.0, 82.0, 48.0, 50.0], false, -4.0),
            (1, [90.0, 91.0, 54.0, 55.0], true, 6.0),
        ] {
            session.record_sample_at(
                started_at + Duration::from_millis(offset * 16),
                TelemetrySample {
                    brake_line_pressure_bar: OptionalTelemetryValues::from_options(
                        pressure.map(Some),
                    ),
                    abs_active: Some(abs),
                    steering_wheel_torque_nm: OptionalTelemetryValues::from_options([Some(torque)]),
                    ..TelemetrySample::default()
                },
            );
        }

        let mut state = DashboardState::default();
        sync_telemetry(&mut state, &session, None);

        for series in 0..4 {
            assert_eq!(state.brake_pressure_chart.series_length(series), Some(2));
        }
        assert_eq!(state.abs_chart.series_length(0), Some(2));
        assert_eq!(state.steering_torque_chart.series_length(0), Some(2));

        let mut unavailable = Session::default();
        unavailable.record_sample(TelemetrySample::default());
        sync_telemetry(&mut state, &unavailable, None);
        assert_eq!(state.brake_pressure_chart.series_length(0), Some(0));
        assert_eq!(state.abs_chart.series_length(0), Some(0));
        assert_eq!(state.steering_torque_chart.series_length(0), Some(0));
    }

    #[test]
    fn switching_tabs_cancels_dragging_and_defers_hidden_chart_updates() {
        let started_at = Instant::now();
        let mut session = Session::default();
        session.record_sample_at(
            started_at,
            TelemetrySample {
                speed_kmh: 100.0,
                ..TelemetrySample::default()
            },
        );
        let mut state = DashboardState::default();
        sync_telemetry(&mut state, &session, None);
        assert_eq!(state.speed_chart.series_length(0), Some(1));
        state.dragging_chart = Some(ChartId::Speed);

        update(
            &mut state,
            &session,
            DashboardMessage::SelectTab(DashboardTab::CarSetup),
        );

        assert_eq!(state.active_tab(), DashboardTab::CarSetup);
        assert_eq!(state.dragging_chart, None);
        session.record_sample_at(
            started_at + Duration::from_millis(16),
            TelemetrySample {
                speed_kmh: 110.0,
                ..TelemetrySample::default()
            },
        );
        session.record_sample_at(
            started_at + Duration::from_millis(32),
            TelemetrySample {
                speed_kmh: 120.0,
                ..TelemetrySample::default()
            },
        );
        update(&mut state, &session, DashboardMessage::Refresh);
        assert_eq!(state.rendered_packets, 1);
        assert_eq!(state.speed_chart.series_length(0), Some(1));

        update(
            &mut state,
            &session,
            DashboardMessage::SelectTab(DashboardTab::Telemetry),
        );
        assert_eq!(state.active_tab(), DashboardTab::Telemetry);
        assert_eq!(state.rendered_packets, 3);
        assert_eq!(state.speed_chart.series_length(0), Some(3));

        update(&mut state, &session, DashboardMessage::CycleTab);
        assert_eq!(state.active_tab(), DashboardTab::CarSetup);
        update(&mut state, &session, DashboardMessage::CycleTab);
        assert_eq!(state.speed_chart.series_length(0), Some(3));
    }

    #[test]
    fn resetting_a_session_on_the_setup_tab_defers_the_full_chart_rebuild() {
        let mut session = Session::default();
        session.record_sample(TelemetrySample {
            speed_kmh: 144.0,
            ..TelemetrySample::default()
        });
        let mut state = DashboardState::default();
        update(
            &mut state,
            &session,
            DashboardMessage::SelectTab(DashboardTab::CarSetup),
        );
        state.rendered_packets = 99;
        let hidden_series_length = state.speed_chart.series_length(0);

        reset_dashboard_telemetry(&mut state, &session, None);

        assert_eq!(state.rendered_packets, 99);
        assert_eq!(state.speed_chart.series_length(0), hidden_series_length);

        update(
            &mut state,
            &session,
            DashboardMessage::SelectTab(DashboardTab::Telemetry),
        );
        assert_eq!(state.rendered_packets, 1);
        assert_eq!(state.speed_chart.series_length(0), Some(1));
    }

    #[test]
    fn missing_or_blank_session_metadata_keeps_a_placeholder_value() {
        assert_eq!(metadata_value(None), "--");
        assert_eq!(metadata_value(Some(String::new())), "--");
        assert_eq!(metadata_value(Some("  ".to_owned())), "--");
        assert_eq!(metadata_value(Some("Dynamic".to_owned())), "Dynamic");
    }

    fn update(
        state: &mut DashboardState,
        session: &Session,
        message: DashboardMessage,
    ) -> Option<Action> {
        update_dashboard(state, session, None, message)
    }

    fn chart_bounds(x: f32, y: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: 100.0,
            height: 100.0,
        }
    }

    fn report_chart_layout(
        state: &mut DashboardState,
        session: &Session,
        chart: ChartId,
        bounds: Rectangle,
    ) {
        update(
            state,
            session,
            DashboardMessage::ChartLayoutChanged {
                chart,
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_chart_drag(
        state: &mut DashboardState,
        session: &Session,
        chart: ChartId,
        origin: Point,
    ) {
        update(state, session, DashboardMessage::BeginChartDrag(chart));
        update(state, session, DashboardMessage::DragCursor(origin));
    }

    fn report_chart_list_layout(
        state: &mut DashboardState,
        session: &Session,
        chart: ChartId,
        bounds: Rectangle,
    ) {
        update(
            state,
            session,
            DashboardMessage::ChartListLayoutChanged {
                chart,
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_chart_list_drag(
        state: &mut DashboardState,
        session: &Session,
        chart: ChartId,
        origin: Point,
    ) {
        update(state, session, DashboardMessage::BeginChartListDrag(chart));
        update(state, session, DashboardMessage::DragCursor(origin));
    }

    fn report_lap_analysis_layout(
        state: &mut DashboardState,
        session: &Session,
        card: LapAnalysisCardId,
        bounds: Rectangle,
    ) {
        update(
            state,
            session,
            DashboardMessage::LapAnalysisLayoutChanged {
                card,
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_lap_analysis_drag(
        state: &mut DashboardState,
        session: &Session,
        card: LapAnalysisCardId,
        origin: Point,
    ) {
        update(state, session, DashboardMessage::BeginLapAnalysisDrag(card));
        update(state, session, DashboardMessage::DragCursor(origin));
    }

    fn report_setup_card_layout(
        state: &mut DashboardState,
        session: &Session,
        card: SetupCardId,
        bounds: Rectangle,
    ) {
        update(
            state,
            session,
            DashboardMessage::SetupCardLayoutChanged {
                card,
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_setup_card_drag(
        state: &mut DashboardState,
        session: &Session,
        card: SetupCardId,
        origin: Point,
    ) {
        update(state, session, DashboardMessage::BeginSetupCardDrag(card));
        update(state, session, DashboardMessage::DragCursor(origin));
    }

    fn report_car_setup_card_layout(
        state: &mut DashboardState,
        session: &Session,
        card: &str,
        bounds: Rectangle,
    ) {
        update(
            state,
            session,
            DashboardMessage::CarSetupCardLayoutChanged {
                card: card.to_owned(),
                bounds,
                visible_bounds: Some(bounds),
            },
        );
    }

    fn begin_car_setup_card_drag(
        state: &mut DashboardState,
        session: &Session,
        card: &str,
        origin: Point,
    ) {
        update(
            state,
            session,
            DashboardMessage::BeginCarSetupCardDrag(card.to_owned()),
        );
        update(state, session, DashboardMessage::DragCursor(origin));
    }

    fn reset_telemetry(state: &mut DashboardState, session: &Session) {
        reset_dashboard_telemetry(state, session, None);
    }

    fn loaded_test_session(track_name: &str, file_name: &str, speed_kmh: f32) -> Session {
        let sample = |elapsed_seconds,
                      completed_laps,
                      current_lap_ms,
                      last_lap_ms,
                      normalized_car_position| TimedSample {
            elapsed_seconds,
            sample: TelemetrySample {
                completed_laps,
                current_lap_ms,
                last_lap_ms,
                normalized_car_position,
                speed_kmh,
                ..TelemetrySample::default()
            },
        };
        let frame = TelemetryFrame::try_new(
            4,
            Vec::<chiaro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();
        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                source: RecordingSource::local_file(file_name),
                file_name: file_name.to_owned(),
                track_name: track_name.to_owned(),
                track_id: None,
                track_config_name: None,
                car_id: None,
                car_name: None,
                duration_seconds: 21.0,
                lap_count: 1,
                record_count: 4,
                tick_rate: 60,
            },
            samples: vec![
                sample(0.0, 0, 0, 0, 0.0),
                sample(10.0, 0, 10_000, 0, 0.5),
                sample(20.0, 0, 20_000, 0, 0.95),
                sample(21.0, 1, 0, 20_000, 0.0),
            ],
            latest_frame: frame,
            session_info: SessionInfo {
                update_count: 1,
                yaml: String::new(),
                raw: Vec::new(),
            },
        });
        session
    }

    #[test]
    fn extracts_current_session_vehicle_course_and_conditions() {
        let yaml = r#"
WeekendInfo:
 TrackDisplayName: Test Circuit
 TrackConfigName: Grand Prix
 TrackLength: 4.20 km
 TrackNumTurns: 12
 TrackType: road course
 TrackDirection: clockwise
 TrackSkies: Clear
 TrackAirTemp: 24.0 C
 TrackSurfaceTemp: 38.0 C
 TrackRelativeHumidity: 55 %
 WeekendOptions:
  Date: 2026-07-16
  TimeOfDay: 3:30 pm
  WindSpeed: 12 km/h
  WindDirection: NW
SessionInfo:
 CurrentSessionNum: 1
 Sessions:
 - SessionNum: 0
   SessionType: Practice
 - SessionNum: 1
   SessionType: Race
   SessionTime: 3600.0 sec
   SessionTrackRubberState: high usage
DriverInfo:
 DriverCarIdx: 2
 Drivers:
 - CarIdx: 1
   CarScreenName: Other Car
 - CarIdx: 2
   CarScreenName: Test GT3
   CarClassShortName: GT3
"#;
        let document = SessionInfo {
            update_count: 1,
            yaml: yaml.to_owned(),
            raw: yaml.as_bytes().to_vec(),
        }
        .parse()
        .expect("valid session metadata");

        let metadata = session_metadata(&document);

        assert_eq!(metadata.track_name.as_deref(), Some("Test Circuit"));
        assert_eq!(metadata.track_config.as_deref(), Some("Grand Prix"));
        assert_eq!(metadata.track_length.as_deref(), Some("4.20 km"));
        assert_eq!(metadata.track_turns, Some(12));
        assert_eq!(
            metadata.track_type.as_deref(),
            Some("road course · clockwise")
        );
        assert_eq!(metadata.car_name.as_deref(), Some("Test GT3"));
        assert_eq!(metadata.car_class.as_deref(), Some("GT3"));
        assert_eq!(metadata.session_type.as_deref(), Some("Race"));
        assert_eq!(metadata.session_time.as_deref(), Some("1:00:00"));
        assert_eq!(metadata.date_time.as_deref(), Some("2026-07-16 · 3:30 pm"));
        assert_eq!(metadata.weather.as_deref(), Some("Clear"));
        assert_eq!(metadata.air_temperature.as_deref(), Some("24.0 C"));
        assert_eq!(metadata.surface_temperature.as_deref(), Some("38.0 C"));
        assert_eq!(metadata.humidity.as_deref(), Some("55 %"));
        assert_eq!(metadata.wind.as_deref(), Some("12 km/h · NW"));
    }

    #[test]
    fn formats_configured_session_time_for_display() {
        assert_eq!(format_session_time(&SessionScalar::Float(90.0)), "1:30");
        assert_eq!(
            format_session_time(&SessionScalar::String("unlimited".to_owned())),
            "Unlimited"
        );
    }

    #[test]
    fn formats_irsdk_gears() {
        assert_eq!(format_gear(-1), "R");
        assert_eq!(format_gear(0), "N");
        assert_eq!(format_gear(5), "5");
    }

    #[test]
    fn formats_lap_times() {
        assert_eq!(format_lap_time(91_234), "1:31.234");
        assert_eq!(format_lap_time(0), "--:--.---");
    }

    #[test]
    fn formats_recording_times_without_sixty_second_rollover() {
        assert_eq!(format_chart_time(5.2), "5s");
        assert_eq!(format_chart_time(65.0), "1:05");
        assert_eq!(format_chart_time(3_599.9), "1:00:00");
        assert_eq!(format_recording_duration(3_661.0), "1:01:01");
    }

    #[test]
    fn formats_lap_distance_axis_units_as_a_percentage() {
        assert_eq!(format_lap_distance(-0.0), "0%");
        assert_eq!(format_lap_distance(0.0), "0%");
        assert_eq!(format_lap_distance(6_200.0), "62%");
        assert_eq!(format_lap_distance(10_000.0), "100%");
    }

    #[test]
    fn formats_race_position() {
        assert_eq!(format_position(3), "P3");
        assert_eq!(format_position(0), "—");
    }

    #[test]
    fn formats_normalized_track_position() {
        assert_eq!(format_track_position(0.425), "42.5%");
        assert_eq!(format_track_position(-1.0), "—");
    }

    #[test]
    fn clamps_pedal_values_to_a_percentage() {
        assert_eq!(pedal_percent(-0.2), 0.0);
        assert_eq!(pedal_percent(0.425), 42.5);
        assert_eq!(pedal_percent(1.2), 100.0);
    }

    #[test]
    fn input_readout_is_unavailable_without_a_sample() {
        let readout = input_readout(None, None);

        assert_eq!(readout.throttle, None);
        assert_eq!(readout.brake, None);
        assert_eq!(readout.steering, None);
    }

    #[test]
    fn input_readout_formats_the_current_sample() {
        let readout = input_readout(
            Some(TelemetrySample {
                throttle: 0.425,
                brake: 0.059,
                steering_angle: (-90.0_f32).to_radians(),
                ..TelemetrySample::default()
            }),
            Some(180.0_f32.to_radians()),
        );
        let throttle = readout.throttle.expect("finite throttle");
        let brake = readout.brake.expect("finite brake");
        let steering = readout.steering.expect("finite steering");

        assert_eq!(throttle.text, "42.5%");
        assert_eq!(throttle.progress, Some(0.425));
        assert_eq!(brake.text, "5.9%");
        assert_eq!(brake.progress, Some(0.059));
        assert_eq!(steering.text, "-90.0°");
        assert_eq!(steering.progress, Some(0.5));
    }

    #[test]
    fn input_readout_clamps_pedal_text_and_progress_together() {
        let readout = input_readout(
            Some(TelemetrySample {
                throttle: -0.2,
                brake: 1.2,
                ..TelemetrySample::default()
            }),
            None,
        );
        let throttle = readout.throttle.expect("throttle");
        let brake = readout.brake.expect("brake");

        assert_eq!(throttle.text, "0.0%");
        assert_eq!(throttle.progress, Some(0.0));
        assert_eq!(brake.text, "100.0%");
        assert_eq!(brake.progress, Some(1.0));
    }

    #[test]
    fn input_readout_rejects_non_finite_values() {
        let readout = input_readout(
            Some(TelemetrySample {
                throttle: f32::NAN,
                brake: f32::INFINITY,
                steering_angle: f32::NEG_INFINITY,
                ..TelemetrySample::default()
            }),
            Some(1.0),
        );

        assert_eq!(readout, super::InputReadout::default());
    }

    #[test]
    fn steering_meter_progress_uses_the_sdk_limit_and_matches_screen_direction() {
        let maximum = 180.0_f32.to_radians();

        assert_eq!(steering_meter_progress(-maximum, maximum), Some(1.0));
        assert_eq!(steering_meter_progress(-maximum / 2.0, maximum), Some(0.5));
        assert_eq!(steering_meter_progress(0.0, maximum), Some(-0.0));
        assert_eq!(steering_meter_progress(maximum / 2.0, maximum), Some(-0.5));
        assert_eq!(steering_meter_progress(maximum * 2.0, maximum), Some(-1.0));
        assert_eq!(steering_meter_progress(0.5, 0.0), None);
        assert_eq!(steering_meter_progress(0.5, f32::NAN), None);
    }

    #[test]
    fn steering_meter_caps_large_sdk_ranges_for_a_more_responsive_display() {
        let sdk_maximum = 450.0_f32.to_radians();
        let right_angle = -90.0_f32.to_radians();
        let left_angle = 90.0_f32.to_radians();

        assert_eq!(steering_meter_progress(right_angle, sdk_maximum), Some(0.5));
        assert_eq!(steering_meter_progress(left_angle, sdk_maximum), Some(-0.5));
    }

    #[test]
    fn steering_text_remains_available_without_an_sdk_limit() {
        let readout = input_readout(
            Some(TelemetrySample {
                steering_angle: 12.5_f32.to_radians(),
                ..TelemetrySample::default()
            }),
            None,
        );
        let steering = readout.steering.expect("finite steering angle");

        assert_eq!(steering.text, "12.5°");
        assert_eq!(steering.progress, None);
    }

    #[test]
    fn expands_symmetric_y_limits_to_fit_outlying_values() {
        assert_eq!(
            symmetric_y_limits([1.0, -2.0].into_iter(), 3.0),
            (-3.0, 3.0)
        );

        let expanded = symmetric_y_limits([2.0, -4.0].into_iter(), 3.0);
        assert!((expanded.0 + 4.8).abs() < 1e-9);
        assert!((expanded.1 - 4.8).abs() < 1e-9);
    }

    #[test]
    fn refreshes_charts_only_after_new_packets() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        let _action = update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.rendered_packets, 1);
        assert_eq!(state.speed_chart.x_axis_label(), "Time");
    }

    #[test]
    fn refresh_is_deferred_while_any_dashboard_card_is_dragged() {
        let begin_messages = [
            DashboardMessage::BeginChartDrag(ChartId::Speed),
            DashboardMessage::BeginChartListDrag(ChartId::Speed),
            DashboardMessage::BeginLapAnalysisDrag(LapAnalysisCardId::Cursor),
            DashboardMessage::BeginSetupCardDrag(SetupCardId::Session),
        ];

        for begin in begin_messages {
            let mut state = DashboardState::default();
            let mut session = Session::default();
            session.record_sample(TelemetrySample::default());

            update(&mut state, &session, begin);
            update(&mut state, &session, DashboardMessage::Refresh);

            assert_eq!(state.rendered_packets, 0);

            update(&mut state, &session, DashboardMessage::FinishCardDrag);
            assert_eq!(state.rendered_packets, 0);

            update(&mut state, &session, DashboardMessage::Refresh);
            assert_eq!(state.rendered_packets, 1);
        }
    }

    #[test]
    fn collapsed_and_dragged_chart_cards_skip_plot_construction() {
        let mut state = DashboardState::default();
        let session = Session::default();

        assert!(should_build_chart_plot(&state, ChartId::Speed));

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartCollapsed(ChartId::Speed),
        );
        assert!(!should_build_chart_plot(&state, ChartId::Speed));

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartCollapsed(ChartId::Speed),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        assert!(!should_build_chart_plot(&state, ChartId::Speed));
        assert!(should_build_chart_plot(&state, ChartId::Pedal));
    }

    #[test]
    fn live_sync_targets_only_visible_expanded_charts() {
        let mut state = DashboardState::default();
        let bounds = chart_bounds(0.0, 0.0);
        let initial_targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
        assert!(initial_targets.into_iter().all(|target| !target));

        for chart in ChartId::ALL {
            state.chart_layouts[chart.index()] = Some(CardLayout {
                bounds,
                visible_bounds: None,
            });
        }
        state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
            bounds,
            visible_bounds: Some(bounds),
        });

        let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
        assert!(targets[ChartId::Speed.index()]);
        assert!(!targets[ChartId::Pedal.index()]);
        let ibt_targets = chart_sync_targets(&state, false, TelemetrySyncScope::LiveVisible);
        assert!(ibt_targets.into_iter().all(|target| target));

        state.chart_collapsed[ChartId::Speed.index()] = true;
        let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
        assert!(!targets[ChartId::Speed.index()]);

        state.chart_collapsed[ChartId::Speed.index()] = false;
        state.chart_layouts[ChartId::Pedal.index()] = Some(CardLayout {
            bounds,
            visible_bounds: Some(bounds),
        });
        state.maximized_chart = Some(ChartId::Pedal);
        let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
        assert!(targets[ChartId::Pedal.index()]);
        assert!(!targets[ChartId::Speed.index()]);

        state.chart_visibility[ChartId::Pedal.index()] = false;
        let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
        assert!(!targets[ChartId::Pedal.index()]);
    }

    #[test]
    fn live_refresh_skips_offscreen_charts_but_full_sync_keeps_reset_correct() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        let bounds = chart_bounds(0.0, 0.0);
        for chart in ChartId::ALL {
            state.chart_layouts[chart.index()] = Some(CardLayout {
                bounds,
                visible_bounds: None,
            });
        }
        state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
            bounds,
            visible_bounds: Some(bounds),
        });
        session.record_sample(TelemetrySample::default());
        session.record_sample(TelemetrySample::default());

        update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.speed_chart.series_length(0), Some(2));
        assert_eq!(state.speed_chart.series_length(1), Some(0));
        assert_eq!(state.pedal_chart.series_length(0), Some(1));
        assert_eq!(state.chart_packet_cursors[ChartId::Speed.index()], Some(2));
        assert_eq!(state.chart_packet_cursors[ChartId::Pedal.index()], None);
        assert_eq!(state.rendered_packets, 2);

        state.chart_layouts[ChartId::Pedal.index()] = Some(CardLayout {
            bounds,
            visible_bounds: Some(bounds),
        });
        update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.pedal_chart.series_length(0), Some(2));
        assert_eq!(state.chart_packet_cursors[ChartId::Pedal.index()], Some(2));

        session.record_sample(TelemetrySample::default());
        update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.speed_chart.series_length(0), Some(3));
        assert_eq!(state.pedal_chart.series_length(0), Some(3));
        assert_eq!(state.rpm_chart.series_length(0), Some(1));

        sync_telemetry(&mut state, &session, None);

        assert_eq!(state.rpm_chart.series_length(0), Some(3));
        assert_eq!(state.pedal_chart.series_length(2), Some(0));
        assert!(
            state
                .chart_packet_cursors
                .into_iter()
                .all(|cursor| cursor == Some(3))
        );
    }

    #[test]
    fn incremental_live_refresh_trims_points_before_the_retained_history() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        let bounds = chart_bounds(0.0, 0.0);
        state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
            bounds,
            visible_bounds: Some(bounds),
        });
        let started_at = Instant::now();
        session.record_sample_at(started_at, TelemetrySample::default());
        session.record_sample_at(
            started_at + Duration::from_secs(1),
            TelemetrySample::default(),
        );

        update(&mut state, &session, DashboardMessage::Refresh);
        assert_eq!(state.speed_chart.series_length(0), Some(2));

        session.record_sample_at(
            started_at + Duration::from_secs(25),
            TelemetrySample::default(),
        );
        update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.speed_chart.series_length(0), Some(1));
        assert_eq!(state.chart_packet_cursors[ChartId::Speed.index()], Some(3));
    }

    #[test]
    fn full_ibt_sync_resets_every_live_chart_cursor() {
        let mut state = DashboardState::default();
        let session = loaded_test_session("Test Circuit", "test.ibt", 120.0);

        sync_telemetry(&mut state, &session, None);

        assert!(
            state
                .chart_packet_cursors
                .into_iter()
                .all(|cursor| cursor.is_none())
        );
    }

    #[test]
    fn live_chart_browsing_pauses_and_reset_restores_latest_following() {
        let mut state = DashboardState::default();
        let session = Session::default();

        let _action = update(
            &mut state,
            &session,
            DashboardMessage::SpeedPlot(TimeSeriesMessage::PanX(mouse::ScrollDelta::Lines {
                x: 0.0,
                y: -1.0,
            })),
        );
        assert!(!state.live_follow);

        let _action = update(
            &mut state,
            &session,
            DashboardMessage::SpeedPlot(TimeSeriesMessage::ResetX),
        );
        assert!(state.live_follow);
    }

    #[test]
    fn tooltip_visibility_is_independent_for_each_chart() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::SpeedPlot(TimeSeriesMessage::ToggleTooltips),
        );

        assert!(!state.speed_chart.tooltips_visible());
        assert!(state.pedal_chart.tooltips_visible());
    }

    #[test]
    fn live_chart_ignores_hover_time_and_keeps_focus_on_the_latest_sample() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        update_chart_focus(&mut state, &session, Some(999.0), LiveChartNavigation::None);

        assert_eq!(state.focus_x, Some(0.0));
        assert!(!state.focus_from_cursor);
    }

    #[test]
    fn requests_the_ibt_file_picker() {
        let mut state = DashboardState::default();
        let session = Session::default();

        let action = update(&mut state, &session, DashboardMessage::OpenIbt);

        assert_eq!(action, Some(Action::OpenIbt));
    }

    #[test]
    fn loads_and_selects_a_reference_lap_without_replacing_the_main_session() {
        let session = loaded_test_session("Test Circuit", "main.ibt", 100.0);
        let reference = loaded_test_session("Test Circuit", "reference.ibt", 110.0);
        let mut state = DashboardState::default();

        let action = update_dashboard(
            &mut state,
            &session,
            Some(&reference),
            DashboardMessage::OpenReferenceIbt,
        );
        assert_eq!(action, Some(Action::OpenReferenceIbt));

        reset_dashboard_telemetry(&mut state, &session, Some(&reference));
        reset_reference_telemetry(&mut state, &session, Some(&reference));

        assert_eq!(state.reference_lap_choices.len(), 2);
        assert_eq!(state.selected_reference_lap_index, Some(0));
        assert!(selected_comparison(&state, &session, Some(&reference)).is_some());

        let different_track = loaded_test_session("Other Circuit", "other.ibt", 110.0);
        reset_reference_telemetry(&mut state, &session, Some(&different_track));
        assert!(selected_comparison(&state, &session, Some(&different_track)).is_none());
    }

    #[test]
    fn overlapping_card_reorders_even_when_cursor_is_outside_target() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let source = chart_bounds(0.0, 0.0);
        let target = chart_bounds(0.0, 116.0);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Fuel, false),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Gear, true),
        );
        assert!(!state.chart_visibility[ChartId::Fuel.index()]);
        report_chart_layout(&mut state, &session, ChartId::Speed, source);
        report_chart_layout(&mut state, &session, ChartId::Gear, target);

        begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));
        let cursor = Point::new(50.0, 40.0);
        update(&mut state, &session, DashboardMessage::DragCursor(cursor));

        assert!(!target.contains(cursor));
        assert_eq!(state.drop_target, Some(ChartId::Gear));

        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(
            &state.chart_order[..8],
            &[
                ChartId::Delta,
                ChartId::Pedal,
                ChartId::BrakePressure,
                ChartId::Abs,
                ChartId::Steering,
                ChartId::SteeringTorque,
                ChartId::Gear,
                ChartId::Speed,
            ]
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.drop_target, None);
        assert_eq!(state.drag_source_bounds, None);
    }

    #[test]
    fn chart_list_reorders_hidden_items_without_dragging_the_chart_card() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let visibility = state.chart_visibility;
        let source = Rectangle {
            width: 200.0,
            height: DATA_ROW_HEIGHT,
            ..Rectangle::default()
        };
        let target = Rectangle {
            y: DATA_ROW_HEIGHT,
            ..source
        };

        assert!(!state.chart_visibility[ChartId::Abs.index()]);
        report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
        report_chart_list_layout(&mut state, &session, ChartId::Speed, source);
        report_chart_list_layout(&mut state, &session, ChartId::Abs, target);
        begin_chart_list_drag(
            &mut state,
            &session,
            ChartId::Speed,
            Point::new(100.0, 10.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(100.0, 40.0)),
        );

        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.chart_list_drop_target, Some(ChartId::Abs));
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(
            &state.chart_order[..5],
            &[
                ChartId::Delta,
                ChartId::Pedal,
                ChartId::BrakePressure,
                ChartId::Abs,
                ChartId::Speed,
            ]
        );
        assert_eq!(state.chart_visibility, visibility);
        assert!(state.chart_layouts.iter().all(Option::is_none));
        assert!(state.chart_list_layouts.iter().all(Option::is_none));
        assert_eq!(state.chart_layout_generation, 1);
        assert_eq!(state.chart_list_layout_generation, 1);
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(state.chart_list_drop_target, None);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Abs, true),
        );
        assert_eq!(state.chart_order[3], ChartId::Abs);
    }

    #[test]
    fn moving_a_chart_list_item_away_cancels_reordering() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.chart_order.clone();
        let source = Rectangle {
            width: 200.0,
            height: DATA_ROW_HEIGHT,
            ..Rectangle::default()
        };
        let target = Rectangle {
            y: DATA_ROW_HEIGHT,
            ..source
        };
        let origin = Point::new(100.0, 10.0);

        report_chart_list_layout(&mut state, &session, ChartId::Speed, source);
        report_chart_list_layout(&mut state, &session, ChartId::Pedal, target);
        begin_chart_list_drag(&mut state, &session, ChartId::Speed, origin);
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(100.0, 40.0)),
        );
        assert_eq!(state.chart_list_drop_target, Some(ChartId::Pedal));

        update(&mut state, &session, DashboardMessage::DragCursor(origin));
        assert_eq!(state.chart_list_drop_target, None);
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.chart_order, original_order);
    }

    #[test]
    fn moving_a_chart_up_inserts_it_at_the_target_position() {
        let mut order = ChartId::ALL.to_vec();

        assert!(move_item_to(&mut order, ChartId::Fuel, ChartId::Pedal));

        assert_eq!(order[1], ChartId::Fuel);
        assert_eq!(order[2], ChartId::Pedal);
    }

    #[test]
    fn lap_analysis_cards_reorder_by_card_overlap() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let chart_order = state.chart_order.clone();
        let target_bounds = chart_bounds(0.0, 116.0);

        assert_eq!(state.lap_analysis_order, LapAnalysisCardId::ALL);
        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Cursor,
            chart_bounds(0.0, 0.0),
        );
        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Vehicle,
            target_bounds,
        );
        begin_lap_analysis_drag(
            &mut state,
            &session,
            LapAnalysisCardId::Cursor,
            Point::new(50.0, 20.0),
        );
        let cursor = Point::new(50.0, 40.0);
        update(&mut state, &session, DashboardMessage::DragCursor(cursor));

        assert!(!target_bounds.contains(cursor));
        assert_eq!(
            state.lap_analysis_drop_target,
            Some(LapAnalysisCardId::Vehicle)
        );

        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(
            &state.lap_analysis_order[..3],
            &[
                LapAnalysisCardId::ReferenceCursor,
                LapAnalysisCardId::Vehicle,
                LapAnalysisCardId::Cursor,
            ]
        );
        assert_eq!(state.chart_order, chart_order);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.lap_analysis_drop_target, None);
    }

    #[test]
    fn moving_a_lap_analysis_card_away_cancels_reordering() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.lap_analysis_order.clone();

        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Inputs,
            chart_bounds(0.0, 0.0),
        );
        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Dynamics,
            chart_bounds(0.0, 116.0),
        );
        begin_lap_analysis_drag(
            &mut state,
            &session,
            LapAnalysisCardId::Inputs,
            Point::new(50.0, 20.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(
            state.lap_analysis_drop_target,
            Some(LapAnalysisCardId::Dynamics)
        );

        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 20.0)),
        );
        assert_eq!(state.lap_analysis_drop_target, None);
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.lap_analysis_order, original_order);
    }

    #[test]
    fn setup_cards_reorder_by_card_overlap() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let chart_order = state.chart_order.clone();
        let analysis_order = state.lap_analysis_order.clone();
        let target_bounds = chart_bounds(0.0, 116.0);

        assert_eq!(state.setup_card_order, SetupCardId::ALL);
        report_setup_card_layout(
            &mut state,
            &session,
            SetupCardId::Session,
            chart_bounds(0.0, 0.0),
        );
        report_setup_card_layout(&mut state, &session, SetupCardId::Laps, target_bounds);
        begin_setup_card_drag(
            &mut state,
            &session,
            SetupCardId::Session,
            Point::new(50.0, 20.0),
        );
        let cursor = Point::new(50.0, 40.0);
        update(&mut state, &session, DashboardMessage::DragCursor(cursor));

        assert!(!target_bounds.contains(cursor));
        assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Laps));
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(
            &state.setup_card_order[..3],
            &[
                SetupCardId::Reference,
                SetupCardId::Laps,
                SetupCardId::Session,
            ]
        );
        assert_eq!(state.chart_order, chart_order);
        assert_eq!(state.lap_analysis_order, analysis_order);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.setup_card_drop_target, None);
        assert_eq!(state.setup_card_drag_source_bounds, None);
    }

    #[test]
    fn moving_a_setup_card_away_cancels_reordering() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.setup_card_order.clone();

        report_setup_card_layout(
            &mut state,
            &session,
            SetupCardId::Reference,
            chart_bounds(0.0, 0.0),
        );
        report_setup_card_layout(
            &mut state,
            &session,
            SetupCardId::Laps,
            chart_bounds(0.0, 116.0),
        );
        begin_setup_card_drag(
            &mut state,
            &session,
            SetupCardId::Reference,
            Point::new(50.0, 20.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Laps));

        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 20.0)),
        );
        assert_eq!(state.setup_card_drop_target, None);
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.setup_card_order, original_order);
    }

    #[test]
    fn car_setup_cards_reorder_only_while_the_cards_overlap() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let source = chart_bounds(0.0, 0.0);
        let target = chart_bounds(0.0, 116.0);

        report_car_setup_card_layout(&mut state, &session, "summary", source);
        report_car_setup_card_layout(&mut state, &session, "status", target);
        begin_car_setup_card_drag(&mut state, &session, "summary", Point::new(50.0, 20.0));
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(state.car_setup_drop_target.as_deref(), Some("status"));

        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 20.0)),
        );
        assert_eq!(state.car_setup_drop_target, None);
        update(&mut state, &session, DashboardMessage::FinishCardDrag);
        assert_eq!(current_car_setup_card_order(&state), ["summary", "status"]);

        report_car_setup_card_layout(&mut state, &session, "summary", source);
        report_car_setup_card_layout(&mut state, &session, "status", target);
        begin_car_setup_card_drag(&mut state, &session, "summary", Point::new(50.0, 20.0));
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(current_car_setup_card_order(&state), ["status", "summary"]);
        assert_eq!(state.layout_revision(), 1);
        assert_eq!(state.dragging_car_setup_card, None);
        assert_eq!(state.car_setup_drop_target, None);
    }

    #[test]
    fn car_setup_collapse_and_maximize_keep_a_single_visible_mode() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleCarSetupCardMaximized("summary".to_owned()),
        );
        assert_eq!(state.maximized_car_setup_card.as_deref(), Some("summary"));
        assert_eq!(state.layout_revision(), 0);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleCarSetupCardCollapsed("summary".to_owned()),
        );
        assert_eq!(state.maximized_car_setup_card, None);
        assert_eq!(state.car_setup_card_collapsed.get("summary"), Some(&true));
        assert_eq!(state.layout_revision(), 1);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleCarSetupCardMaximized("summary".to_owned()),
        );
        assert_eq!(state.maximized_car_setup_card.as_deref(), Some("summary"));
        assert_eq!(state.car_setup_card_collapsed.get("summary"), Some(&false));
        assert_eq!(state.layout_revision(), 2);
    }

    #[test]
    fn all_card_drags_are_mutually_exclusive() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        assert_eq!(state.dragging_chart, Some(ChartId::Speed));
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.dragging_car_setup_card, None);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartListDrag(ChartId::Abs),
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.dragging_chart_list_item, Some(ChartId::Abs));
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.dragging_car_setup_card, None);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginLapAnalysisDrag(LapAnalysisCardId::Vehicle),
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(
            state.dragging_lap_analysis_card,
            Some(LapAnalysisCardId::Vehicle)
        );
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.dragging_car_setup_card, None);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginSetupCardDrag(SetupCardId::Reference),
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.dragging_setup_card, Some(SetupCardId::Reference));
        assert_eq!(state.dragging_car_setup_card, None);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginCarSetupCardDrag("summary".to_owned()),
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.dragging_car_setup_card.as_deref(), Some("summary"));

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Pedal),
        );
        assert_eq!(state.dragging_chart, Some(ChartId::Pedal));
        assert_eq!(state.dragging_chart_list_item, None);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.dragging_car_setup_card, None);
    }

    #[test]
    fn cancelling_pointer_interactions_aborts_lap_analysis_dragging() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.lap_analysis_order.clone();

        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Tyres,
            chart_bounds(0.0, 0.0),
        );
        report_lap_analysis_layout(
            &mut state,
            &session,
            LapAnalysisCardId::Wheels,
            chart_bounds(0.0, 116.0),
        );
        begin_lap_analysis_drag(
            &mut state,
            &session,
            LapAnalysisCardId::Tyres,
            Point::new(50.0, 20.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(
            state.lap_analysis_drop_target,
            Some(LapAnalysisCardId::Wheels)
        );

        update(
            &mut state,
            &session,
            DashboardMessage::CancelPointerInteractions {
                reset_modifiers: false,
            },
        );

        assert_eq!(state.lap_analysis_order, original_order);
        assert_eq!(state.dragging_lap_analysis_card, None);
        assert_eq!(state.lap_analysis_drop_target, None);
        assert_eq!(state.lap_analysis_drag_source_bounds, None);
    }

    #[test]
    fn cancelling_pointer_interactions_aborts_setup_card_dragging() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.setup_card_order.clone();

        report_setup_card_layout(
            &mut state,
            &session,
            SetupCardId::Laps,
            chart_bounds(0.0, 0.0),
        );
        report_setup_card_layout(
            &mut state,
            &session,
            SetupCardId::Charts,
            chart_bounds(0.0, 116.0),
        );
        begin_setup_card_drag(
            &mut state,
            &session,
            SetupCardId::Laps,
            Point::new(50.0, 20.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Charts));

        update(
            &mut state,
            &session,
            DashboardMessage::CancelPointerInteractions {
                reset_modifiers: false,
            },
        );

        assert_eq!(state.setup_card_order, original_order);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.setup_card_drop_target, None);
        assert_eq!(state.setup_card_drag_source_bounds, None);
    }

    #[test]
    fn moving_the_card_away_cancels_the_drop_target() {
        let mut state = DashboardState::default();
        let session = Session::default();
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Gear, true),
        );
        let original_order = state.chart_order.clone();

        report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
        report_chart_layout(
            &mut state,
            &session,
            ChartId::Gear,
            chart_bounds(0.0, 116.0),
        );
        begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));

        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        assert_eq!(state.drop_target, Some(ChartId::Gear));

        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 20.0)),
        );
        assert_eq!(state.drop_target, None);

        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.chart_order, original_order);
    }

    #[test]
    fn dropping_without_a_target_does_not_reorder_charts() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.chart_order.clone();

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.chart_order, original_order);
    }

    #[test]
    fn cancelling_pointer_interactions_aborts_chart_and_cursor_dragging() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.chart_order.clone();

        report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
        report_chart_layout(
            &mut state,
            &session,
            ChartId::Gear,
            chart_bounds(0.0, 116.0),
        );
        begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(50.0, 40.0)),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::SpeedPlot(TimeSeriesMessage::BeginCursorDrag),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::KeyboardModifiersChanged(keyboard::Modifiers::CTRL),
        );

        update(
            &mut state,
            &session,
            DashboardMessage::CancelPointerInteractions {
                reset_modifiers: false,
            },
        );

        assert_eq!(state.chart_order, original_order);
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.drop_target, None);
        assert_eq!(state.drag_source_bounds, None);
        assert!(!state.speed_chart.is_cursor_dragging());
        assert_eq!(state.modifiers, keyboard::Modifiers::CTRL);

        update(
            &mut state,
            &session,
            DashboardMessage::CancelPointerInteractions {
                reset_modifiers: true,
            },
        );
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert_eq!(state.modifiers, keyboard::Modifiers::NONE);
    }

    #[test]
    fn the_target_with_the_largest_visible_overlap_is_selected() {
        let mut layouts = [None; ChartId::COUNT];
        layouts[ChartId::Pedal.index()] = Some(CardLayout {
            bounds: chart_bounds(80.0, 100.0),
            visible_bounds: Some(chart_bounds(80.0, 100.0)),
        });
        layouts[ChartId::Gear.index()] = Some(CardLayout {
            bounds: chart_bounds(160.0, 100.0),
            visible_bounds: Some(chart_bounds(160.0, 100.0)),
        });
        let mut visibility = [true; ChartId::COUNT];
        let dragged = chart_bounds(100.0, 100.0);

        assert_eq!(
            select_drop_target(ChartId::Speed, dragged, &ChartId::ALL, |chart| {
                visibility[chart.index()]
                    .then(|| layouts[chart.index()].and_then(|layout| layout.visible_bounds))
                    .flatten()
            }),
            Some(ChartId::Pedal)
        );

        visibility[ChartId::Pedal.index()] = false;
        assert_eq!(
            select_drop_target(ChartId::Speed, dragged, &ChartId::ALL, |chart| {
                visibility[chart.index()]
                    .then(|| layouts[chart.index()].and_then(|layout| layout.visible_bounds))
                    .flatten()
            }),
            Some(ChartId::Gear)
        );
    }

    #[test]
    fn clipped_or_edge_only_overlap_is_not_a_drop_target() {
        let mut layouts = [None; ChartId::COUNT];
        let target = chart_bounds(0.0, 100.0);
        layouts[ChartId::Pedal.index()] = Some(CardLayout {
            bounds: target,
            visible_bounds: Some(Rectangle {
                y: 150.0,
                height: 50.0,
                ..target
            }),
        });

        assert_eq!(
            select_drop_target(
                ChartId::Speed,
                chart_bounds(0.0, 50.0),
                &ChartId::ALL,
                |chart| layouts[chart.index()].and_then(|layout| layout.visible_bounds),
            ),
            None
        );
        assert_eq!(
            select_drop_target(
                ChartId::Speed,
                chart_bounds(0.0, 51.0),
                &ChartId::ALL,
                |chart| layouts[chart.index()].and_then(|layout| layout.visible_bounds),
            ),
            Some(ChartId::Pedal)
        );
    }

    #[test]
    fn chart_layout_and_drag_state_are_updated() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::SetChartColumns(ChartColumns::Two),
        );
        assert_eq!(state.chart_columns, ChartColumns::Two);
        report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
        report_chart_layout(
            &mut state,
            &session,
            ChartId::Pedal,
            chart_bounds(0.0, 116.0),
        );

        begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(10.0, 20.0));
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(10.0, 40.0)),
        );
        assert_eq!(state.drag_origin, Some(Point::new(10.0, 20.0)));
        assert_eq!(state.drag_cursor, Some(Point::new(10.0, 40.0)));
        assert_eq!(state.drop_target, Some(ChartId::Pedal));

        update(&mut state, &session, DashboardMessage::FinishCardDrag);
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.drag_origin, None);
        assert_eq!(
            &state.chart_order[..3],
            &[ChartId::Delta, ChartId::Pedal, ChartId::Speed]
        );
    }

    #[test]
    fn chart_maximization_toggles_and_clears_when_hidden() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartMaximized(ChartId::Speed),
        );
        assert_eq!(state.maximized_chart, Some(ChartId::Speed));

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartMaximized(ChartId::Speed),
        );
        assert_eq!(state.maximized_chart, None);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartMaximized(ChartId::Pedal),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Pedal, false),
        );
        assert_eq!(state.maximized_chart, None);
    }

    #[test]
    fn every_dashboard_card_starts_expanded_and_toggles_independently() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let chart_order = state.chart_order.clone();
        let analysis_order = state.lap_analysis_order.clone();
        let setup_order = state.setup_card_order.clone();

        assert!(state.chart_collapsed.iter().all(|collapsed| !collapsed));
        assert!(
            state
                .lap_analysis_collapsed
                .iter()
                .all(|collapsed| !collapsed)
        );
        assert!(
            state
                .setup_card_collapsed
                .iter()
                .all(|collapsed| !collapsed)
        );

        for chart in ChartId::ALL {
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleChartCollapsed(chart),
            );
            assert!(state.chart_collapsed[chart.index()]);
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleChartCollapsed(chart),
            );
            assert!(!state.chart_collapsed[chart.index()]);
        }
        for card in LapAnalysisCardId::ALL {
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleLapAnalysisCardCollapsed(card),
            );
            assert!(state.lap_analysis_collapsed[card.index()]);
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleLapAnalysisCardCollapsed(card),
            );
            assert!(!state.lap_analysis_collapsed[card.index()]);
        }
        for card in SetupCardId::ALL {
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleSetupCardCollapsed(card),
            );
            assert!(state.setup_card_collapsed[card.index()]);
            update(
                &mut state,
                &session,
                DashboardMessage::ToggleSetupCardCollapsed(card),
            );
            assert!(!state.setup_card_collapsed[card.index()]);
        }

        assert_eq!(state.chart_order, chart_order);
        assert_eq!(state.lap_analysis_order, analysis_order);
        assert_eq!(state.setup_card_order, setup_order);
    }

    #[test]
    fn chart_collapse_and_maximize_keep_a_single_visible_mode() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartMaximized(ChartId::Speed),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartCollapsed(ChartId::Speed),
        );
        assert!(state.chart_collapsed[ChartId::Speed.index()]);
        assert_eq!(state.maximized_chart, None);

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChartMaximized(ChartId::Speed),
        );
        assert!(!state.chart_collapsed[ChartId::Speed.index()]);
        assert_eq!(state.maximized_chart, Some(ChartId::Speed));
    }

    #[test]
    fn collapsing_during_a_drag_cancels_the_pending_reorder() {
        let mut state = DashboardState::default();
        let session = Session::default();
        let original_order = state.setup_card_order.clone();
        let source_bounds = Rectangle {
            width: 100.0,
            height: 42.0,
            ..Rectangle::default()
        };
        let target_bounds = Rectangle {
            y: 50.0,
            width: 100.0,
            height: 100.0,
            ..Rectangle::default()
        };

        report_setup_card_layout(&mut state, &session, SetupCardId::Laps, source_bounds);
        report_setup_card_layout(&mut state, &session, SetupCardId::Charts, target_bounds);
        begin_setup_card_drag(
            &mut state,
            &session,
            SetupCardId::Laps,
            Point::new(10.0, 10.0),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(Point::new(10.0, 70.0)),
        );
        assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Charts));

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleSetupCardCollapsed(SetupCardId::Laps),
        );
        update(&mut state, &session, DashboardMessage::FinishCardDrag);

        assert!(state.setup_card_collapsed[SetupCardId::Laps.index()]);
        assert_eq!(state.setup_card_order, original_order);
        assert_eq!(state.dragging_setup_card, None);
        assert_eq!(state.setup_card_drop_target, None);
        assert_eq!(state.setup_card_drag_origin, None);
        assert_eq!(state.setup_card_drag_cursor, None);
        assert_eq!(state.setup_card_drag_source_bounds, None);
    }

    #[test]
    fn collapsed_cards_remain_draggable_with_header_sized_bounds() {
        let session = Session::default();
        let source_bounds = Rectangle {
            width: 100.0,
            height: 42.0,
            ..Rectangle::default()
        };
        let target_bounds = Rectangle {
            y: 50.0,
            width: 100.0,
            height: 42.0,
            ..Rectangle::default()
        };
        let origin = Point::new(10.0, 10.0);
        let target = Point::new(10.0, 60.0);

        let mut charts = DashboardState::default();
        update(
            &mut charts,
            &session,
            DashboardMessage::ToggleChartCollapsed(ChartId::Speed),
        );
        report_chart_layout(&mut charts, &session, ChartId::Speed, source_bounds);
        report_chart_layout(&mut charts, &session, ChartId::Pedal, target_bounds);
        begin_chart_drag(&mut charts, &session, ChartId::Speed, origin);
        update(&mut charts, &session, DashboardMessage::DragCursor(target));
        update(&mut charts, &session, DashboardMessage::FinishCardDrag);
        assert_eq!(
            &charts.chart_order[..3],
            &[ChartId::Delta, ChartId::Pedal, ChartId::Speed]
        );
        assert!(charts.chart_collapsed[ChartId::Speed.index()]);

        let mut analysis = DashboardState::default();
        update(
            &mut analysis,
            &session,
            DashboardMessage::ToggleLapAnalysisCardCollapsed(LapAnalysisCardId::Cursor),
        );
        report_lap_analysis_layout(
            &mut analysis,
            &session,
            LapAnalysisCardId::Cursor,
            source_bounds,
        );
        report_lap_analysis_layout(
            &mut analysis,
            &session,
            LapAnalysisCardId::ReferenceCursor,
            target_bounds,
        );
        begin_lap_analysis_drag(&mut analysis, &session, LapAnalysisCardId::Cursor, origin);
        update(
            &mut analysis,
            &session,
            DashboardMessage::DragCursor(target),
        );
        update(&mut analysis, &session, DashboardMessage::FinishCardDrag);
        assert_eq!(
            &analysis.lap_analysis_order[..2],
            &[
                LapAnalysisCardId::ReferenceCursor,
                LapAnalysisCardId::Cursor,
            ]
        );
        assert!(analysis.lap_analysis_collapsed[LapAnalysisCardId::Cursor.index()]);

        let mut setup = DashboardState::default();
        update(
            &mut setup,
            &session,
            DashboardMessage::ToggleSetupCardCollapsed(SetupCardId::Session),
        );
        report_setup_card_layout(&mut setup, &session, SetupCardId::Session, source_bounds);
        report_setup_card_layout(&mut setup, &session, SetupCardId::Reference, target_bounds);
        begin_setup_card_drag(&mut setup, &session, SetupCardId::Session, origin);
        update(&mut setup, &session, DashboardMessage::DragCursor(target));
        update(&mut setup, &session, DashboardMessage::FinishCardDrag);
        assert_eq!(
            &setup.setup_card_order[..2],
            &[SetupCardId::Reference, SetupCardId::Session]
        );
        assert!(setup.setup_card_collapsed[SetupCardId::Session.index()]);
    }

    #[test]
    fn resets_to_the_latest_complete_lap_and_switches_laps() {
        let timed_sample = |elapsed_seconds,
                            completed_laps,
                            current_lap_ms,
                            last_lap_ms,
                            normalized_car_position| TimedSample {
            elapsed_seconds,
            sample: TelemetrySample {
                completed_laps,
                current_lap_ms,
                last_lap_ms,
                normalized_car_position,
                ..TelemetrySample::default()
            },
        };
        let frame = TelemetryFrame::try_new(
            3,
            Vec::<chiaro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();
        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                source: RecordingSource::local_file("laps.ibt"),
                file_name: "laps.ibt".to_owned(),
                track_name: "Test Circuit".to_owned(),
                track_id: None,
                track_config_name: None,
                car_id: None,
                car_name: None,
                duration_seconds: 60.0,
                lap_count: 1,
                record_count: 4,
                tick_rate: 60,
            },
            samples: vec![
                timed_sample(0.0, 0, 0, 0, 0.0),
                timed_sample(50.0, 0, 50_000, 0, 0.9),
                timed_sample(51.0, 1, 0, 51_000, 0.0),
                timed_sample(60.0, 1, 9_000, 51_000, 0.2),
            ],
            latest_frame: frame,
            session_info: SessionInfo {
                update_count: 1,
                yaml: String::new(),
                raw: Vec::new(),
            },
        });
        let mut state = DashboardState::default();

        reset_telemetry(&mut state, &session);

        assert_eq!(state.lap_choices.len(), 2);
        assert_eq!(state.selected_lap_index, Some(0));
        assert_eq!(
            state.focus_x,
            Some(f64::from(0.9_f32) * LAP_DISTANCE_AXIS_MAX)
        );
        assert_eq!(state.speed_chart.x_axis_label(), "Lap distance");
        assert_eq!(state.speed_chart.x_limits(), (0.0, LAP_DISTANCE_AXIS_MAX));

        let action = update(&mut state, &session, DashboardMessage::SelectLap(1));

        assert_eq!(action, None);
        assert_eq!(state.selected_lap_index, Some(1));
        assert_eq!(
            state.focus_x,
            Some(f64::from(0.2_f32) * LAP_DISTANCE_AXIS_MAX)
        );

        state.focus_from_cursor = true;
        focus_at(&mut state, &session, 1_800.0);

        assert_eq!(
            state.focus_x,
            Some(f64::from(0.2_f32) * LAP_DISTANCE_AXIS_MAX)
        );
        assert_eq!(state.focused.map(|point| point.elapsed_seconds), Some(9.0));
        assert_eq!(state.steering_chart.focus_index(), Some(1));
        assert_eq!(state.rpm_chart.focus_index(), Some(1));
        assert_eq!(state.gear_chart.focus_index(), Some(1));
        assert_eq!(state.yaw_chart.focus_index(), Some(1));
        assert_eq!(state.wheel_slip_chart.focus_index(), Some(1));
        assert_eq!(state.suspension_chart.focus_index(), Some(1));
        assert_eq!(state.fuel_chart.focus_index(), Some(1));
    }
}
