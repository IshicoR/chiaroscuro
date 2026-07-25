//! Telemetry screen state, update logic, and view.

mod charts;
mod formatting;
mod interaction;
mod lap_choice;
mod layout;
mod metadata;
mod readout;
mod scaling;
mod tyre_status;

pub use interaction::subscription;
pub use layout::{
    ChartColumns, ChartId, LapAnalysisCardId, SetupCardId, TelemetryLayout, TelemetryLayoutFlag,
};

use charts::{
    build_abs_chart, build_brake_pressure_chart, build_delta_chart, build_dynamics_chart,
    build_fuel_chart, build_gear_chart, build_pedal_chart, build_rpm_chart, build_speed_chart,
    build_steering_chart, build_steering_torque_chart, build_suspension_chart, build_tyre_chart,
    build_wheel_slip_chart, build_yaw_chart, lap_distance_axis, time_axis,
};
use formatting::{
    format_gear, format_lap_count, format_position, format_recording_duration,
    format_track_position,
};
use interaction::{
    move_item_to, update_chart_drop_target, update_chart_list_drop_target,
    update_lap_analysis_drop_target, update_setup_card_drop_target,
};
use layout::{normalize_layout_flags, normalize_layout_order};
use metadata::sync_session_metadata;
use readout::{
    InputMeter, InputMeterValue, abs_activity_percent, input_pedal_value, input_readout,
    pedal_percent,
};
use scaling::{maximum_y, padded_y_limits, symmetric_y_limits};

use chiaro_actions::Action;
use chiaro_irsdk::{TelemetrySample, variables};
use chiaro_telemetry::{
    ConnectionStatus, FocusedTelemetry, HISTORY_WINDOW, LAP_DISTANCE_AXIS_MAX,
    LiveTelemetrySourceInfo, Session,
};
use chiaro_time_series_chart::{TimeSeriesChart, TimeSeriesMessage};
use chiaro_widgets::{
    BadgeVariant, ButtonSize, ButtonVariant, CARD_HEADER_HEIGHT, CardTitle, badge, bounds_reporter,
    button as action_button, callout, card_drag_handle, chart_card, checkbox_style, icon_button,
    icon_toggle_button, pane_card, typography,
};
use iced::{
    Color, Element, Length, Point, Rectangle, Vector,
    alignment::{Horizontal, Vertical},
    keyboard, mouse,
    widget::{Space, checkbox, column, container, float, grid, row, rule, scrollable, stack, text},
};
use iced_fonts::lucide;
use iced_plot::AxisLink;
use lap_choice::{LapChoice, format_lap_time, lap_choice_list};

const CHART_HEIGHT: f32 = 360.0;
const CONTENT_PADDING: f32 = 24.0;
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
const THROTTLE_LINE_COLOR: Color = Color::from_rgb(0.12, 0.72, 0.38);
const BRAKE_LINE_COLOR: Color = Color::from_rgb(0.90, 0.24, 0.24);
const STEERING_LINE_COLOR: Color = Color::from_rgb(0.20, 0.72, 0.68);

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
pub struct TelemetryState {
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
    modifiers: keyboard::Modifiers,
}

impl Default for TelemetryState {
    fn default() -> Self {
        let x_axis_link = AxisLink::new();
        Self {
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
            modifiers: keyboard::Modifiers::NONE,
        }
    }
}

impl TelemetryState {
    pub const fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    pub fn layout_snapshot(&self) -> TelemetryLayout {
        TelemetryLayout {
            chart_order: self
                .chart_order
                .iter()
                .map(|chart| chart.key().to_owned())
                .collect(),
            chart_visibility: ChartId::ALL
                .map(|chart| TelemetryLayoutFlag {
                    key: chart.key().to_owned(),
                    value: self.chart_visibility[chart.index()],
                })
                .to_vec(),
            chart_collapsed: ChartId::ALL
                .map(|chart| TelemetryLayoutFlag {
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
                .map(|card| TelemetryLayoutFlag {
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
                .map(|card| TelemetryLayoutFlag {
                    key: card.key().to_owned(),
                    value: self.lap_analysis_collapsed[card.index()],
                })
                .to_vec(),
        }
    }

    /// Restores a persisted layout without marking it as a user edit.
    pub fn apply_layout(&mut self, layout: &TelemetryLayout) {
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
        self.maximized_chart = None;
        self.clear_chart_drag();
        self.clear_chart_list_drag();
        self.clear_lap_analysis_drag();
        self.clear_setup_card_drag();
        self.cancel_chart_interactions();
        self.invalidate_chart_layouts();
        self.invalidate_chart_list_layouts();
        self.lap_analysis_layouts = [None; LapAnalysisCardId::COUNT];
        self.setup_card_layouts = [None; SetupCardId::COUNT];
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

    fn is_dragging_card(&self) -> bool {
        self.dragging_chart.is_some()
            || self.dragging_chart_list_item.is_some()
            || self.dragging_lap_analysis_card.is_some()
            || self.dragging_setup_card.is_some()
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

#[derive(Debug, Clone)]
pub enum TelemetryMessage {
    ToggleConnection,
    OpenIbt,
    OpenReferenceIbt,
    ClearReferenceIbt,
    SelectLap(usize),
    SelectReferenceLap(usize),
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
    FinishCardDrag,
    SetChartColumns(ChartColumns),
    ResetTelemetryLayout,
    ToggleChartMaximized(ChartId),
    DragCursor(Point),
    KeyboardModifiersChanged(keyboard::Modifiers),
    CancelPointerInteractions {
        reset_modifiers: bool,
    },
}

impl TelemetryMessage {
    pub const fn resets_layout(&self) -> bool {
        matches!(self, Self::ResetTelemetryLayout)
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
    state: &mut TelemetryState,
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

/// Synchronizes state when the Telemetry screen becomes active.
pub fn activate(
    state: &mut TelemetryState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    deactivate(state);
    state.invalidate_chart_layouts();
    sync_telemetry(state, session, reference_session);
}

/// Cancels interactions owned by the Telemetry screen before navigation.
pub fn deactivate(state: &mut TelemetryState) {
    state.clear_chart_drag();
    state.clear_chart_list_drag();
    state.clear_lap_analysis_drag();
    state.clear_setup_card_drag();
    state.cancel_chart_interactions();
    state.modifiers = keyboard::Modifiers::NONE;
}

/// Synchronizes the visible Telemetry screen with the latest session data.
pub fn refresh(state: &mut TelemetryState, session: &Session, reference_session: Option<&Session>) {
    sync_session_metadata(state, session);
    if !state.is_dragging_card() && telemetry_sync_is_pending(state, session) {
        let scope = if session.ibt_info().is_none() {
            TelemetrySyncScope::LiveVisible
        } else {
            TelemetrySyncScope::All
        };
        sync_telemetry_with_scope(state, session, reference_session, scope);
    }
}

pub fn update(
    state: &mut TelemetryState,
    session: &Session,
    reference_session: Option<&Session>,
    message: TelemetryMessage,
) -> Option<Action> {
    match message {
        TelemetryMessage::ToggleConnection => {
            Some(Action::SetConnected(!session.wants_connection()))
        },
        TelemetryMessage::OpenIbt => Some(Action::OpenIbt),
        TelemetryMessage::OpenReferenceIbt => Some(Action::OpenReferenceIbt),
        TelemetryMessage::ClearReferenceIbt => Some(Action::ClearReferenceIbt),
        TelemetryMessage::SelectLap(index) => {
            if index < session.laps().len() {
                state.selected_lap_index = Some(index);
                state.focus_x = None;
                state.focus_from_cursor = false;
                sync_telemetry(state, session, reference_session);
            }
            None
        },
        TelemetryMessage::SelectReferenceLap(index) => {
            if reference_session.is_some_and(|reference| index < reference.laps().len()) {
                state.selected_reference_lap_index = Some(index);
                sync_telemetry(state, session, reference_session);
            }
            None
        },
        TelemetryMessage::SpeedPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.speed_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::PedalPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.pedal_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::BrakePressurePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.brake_pressure_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::AbsPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.abs_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::SteeringPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.steering_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::SteeringTorquePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.steering_torque_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::RpmPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.rpm_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::GearPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.gear_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::DynamicsPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.dynamics_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::YawPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.yaw_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::WheelSlipPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.wheel_slip_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::TyrePlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.tyre_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::SuspensionPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.suspension_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::FuelPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.fuel_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::DeltaPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.delta_chart.update(message);
            update_chart_focus(state, session, cursor_x, navigation);
            None
        },
        TelemetryMessage::ToggleChart(chart, visible) => {
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
        TelemetryMessage::ToggleChartCollapsed(chart) => {
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
            state.mark_layout_changed();
            None
        },
        TelemetryMessage::BeginChartDrag(chart) => {
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.dragging_chart = Some(chart);
            state.drop_target = None;
            state.drag_origin = None;
            state.drag_cursor = None;
            state.drag_source_bounds =
                state.chart_layouts[chart.index()].map(|layout| layout.bounds);
            None
        },
        TelemetryMessage::ChartLayoutChanged {
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
        TelemetryMessage::BeginChartListDrag(chart) => {
            state.clear_chart_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.dragging_chart_list_item = Some(chart);
            state.chart_list_drop_target = None;
            state.chart_list_drag_origin = None;
            state.chart_list_drag_cursor = None;
            state.chart_list_drag_source_bounds =
                state.chart_list_layouts[chart.index()].map(|layout| layout.bounds);
            None
        },
        TelemetryMessage::ChartListLayoutChanged {
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
        TelemetryMessage::ToggleLapAnalysisCardCollapsed(card) => {
            let collapsed = &mut state.lap_analysis_collapsed[card.index()];
            *collapsed = !*collapsed;
            state.lap_analysis_layouts[card.index()] = None;
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        TelemetryMessage::BeginLapAnalysisDrag(card) => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_setup_card_drag();
            state.dragging_lap_analysis_card = Some(card);
            state.lap_analysis_drop_target = None;
            state.lap_analysis_drag_origin = None;
            state.lap_analysis_drag_cursor = None;
            state.lap_analysis_drag_source_bounds =
                state.lap_analysis_layouts[card.index()].map(|layout| layout.bounds);
            None
        },
        TelemetryMessage::LapAnalysisLayoutChanged {
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
        TelemetryMessage::ToggleSetupCardCollapsed(card) => {
            let collapsed = &mut state.setup_card_collapsed[card.index()];
            *collapsed = !*collapsed;
            state.setup_card_layouts[card.index()] = None;
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.mark_layout_changed();
            None
        },
        TelemetryMessage::BeginSetupCardDrag(card) => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.dragging_setup_card = Some(card);
            state.setup_card_drop_target = None;
            state.setup_card_drag_origin = None;
            state.setup_card_drag_cursor = None;
            state.setup_card_drag_source_bounds =
                state.setup_card_layouts[card.index()].map(|layout| layout.bounds);
            None
        },
        TelemetryMessage::SetupCardLayoutChanged {
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
        TelemetryMessage::FinishCardDrag => {
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
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            if chart_order_changed {
                state.invalidate_chart_layouts();
                state.invalidate_chart_list_layouts();
            }
            if layout_changed {
                state.mark_layout_changed();
            }
            None
        },
        TelemetryMessage::SetChartColumns(columns) => {
            if state.chart_columns == columns {
                return None;
            }
            state.chart_columns = columns;
            state.invalidate_chart_layouts();
            state.mark_layout_changed();
            None
        },
        TelemetryMessage::ResetTelemetryLayout => {
            state.apply_layout(&TelemetryLayout::default());
            // Reset also removes an explicit persisted override, so it is a
            // meaningful save operation even when the values were defaults.
            state.mark_layout_changed();
            None
        },
        TelemetryMessage::ToggleChartMaximized(chart) => {
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
            if expanded_collapsed {
                state.mark_layout_changed();
            }
            None
        },
        TelemetryMessage::DragCursor(position) => {
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
            None
        },
        TelemetryMessage::KeyboardModifiersChanged(modifiers) => {
            state.modifiers = modifiers;
            None
        },
        TelemetryMessage::CancelPointerInteractions { reset_modifiers } => {
            state.clear_chart_drag();
            state.clear_chart_list_drag();
            state.clear_lap_analysis_drag();
            state.clear_setup_card_drag();
            state.cancel_chart_interactions();
            if reset_modifiers {
                state.modifiers = keyboard::Modifiers::NONE;
            }
            None
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelemetrySyncScope {
    All,
    LiveVisible,
}

fn live_chart_is_active(state: &TelemetryState, chart: ChartId) -> bool {
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
    state: &TelemetryState,
    is_live: bool,
    scope: TelemetrySyncScope,
) -> [bool; ChartId::COUNT] {
    if !is_live || scope == TelemetrySyncScope::All {
        return [true; ChartId::COUNT];
    }

    std::array::from_fn(|index| live_chart_is_active(state, ChartId::ALL[index]))
}

fn telemetry_sync_is_pending(state: &TelemetryState, session: &Session) -> bool {
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
    state: &mut TelemetryState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    sync_telemetry_with_scope(state, session, reference_session, TelemetrySyncScope::All);
}

fn sync_telemetry_with_scope(
    state: &mut TelemetryState,
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

fn chart_uses_lap_distance(state: &TelemetryState, session: &Session) -> bool {
    session.ibt_info().is_some() && state.selected_lap_index.is_some()
}

pub fn reset_session(
    state: &mut TelemetryState,
    session: &Session,
    reference_session: Option<&Session>,
    telemetry_active: bool,
) {
    state.cached_session_info_revision = None;
    state.session_metadata = SessionMetadata::default();
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
    if telemetry_active {
        sync_telemetry(state, session, reference_session);
    } else {
        sync_session_metadata(state, session);
    }
}

pub fn reset_reference(
    state: &mut TelemetryState,
    session: &Session,
    reference_session: Option<&Session>,
    telemetry_active: bool,
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
    if telemetry_active {
        sync_telemetry(state, session, reference_session);
    }
}

fn selected_comparison<'a>(
    state: &TelemetryState,
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

fn focus_at(state: &mut TelemetryState, session: &Session, x: f64) {
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
    state: &'a TelemetryState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, TelemetryMessage> {
    container(telemetry_content(
        state,
        session,
        reference_session,
        live_source,
    ))
    .padding(CONTENT_PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn telemetry_content<'a>(
    state: &'a TelemetryState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> Element<'a, TelemetryMessage> {
    let charts: Element<'_, TelemetryMessage> = if let Some(chart) = state.maximized_chart {
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

fn chart_view<'a>(state: &'a TelemetryState, chart: ChartId) -> Element<'a, TelemetryMessage> {
    let focused_x = state.focus_x;
    let scroll_modifiers = state.modifiers;
    let content: Element<'a, TelemetryMessage> = if should_build_chart_plot(state, chart) {
        match chart {
            ChartId::Speed => state
                .speed_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::SpeedPlot),
            ChartId::Pedal => state
                .pedal_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::PedalPlot),
            ChartId::BrakePressure => state
                .brake_pressure_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::BrakePressurePlot),
            ChartId::Abs => state
                .abs_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::AbsPlot),
            ChartId::Steering => state
                .steering_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::SteeringPlot),
            ChartId::SteeringTorque => state
                .steering_torque_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::SteeringTorquePlot),
            ChartId::Rpm => state
                .rpm_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::RpmPlot),
            ChartId::Gear => state
                .gear_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::GearPlot),
            ChartId::Dynamics => state
                .dynamics_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::DynamicsPlot),
            ChartId::Yaw => state
                .yaw_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::YawPlot),
            ChartId::WheelSlip => state
                .wheel_slip_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::WheelSlipPlot),
            ChartId::Tyre => state
                .tyre_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::TyrePlot),
            ChartId::Suspension => state
                .suspension_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::SuspensionPlot),
            ChartId::Fuel => state
                .fuel_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::FuelPlot),
            ChartId::Delta => state
                .delta_chart
                .view(focused_x, scroll_modifiers)
                .map(TelemetryMessage::DeltaPlot),
        }
    } else {
        Space::new().width(Length::Fill).height(Length::Fill).into()
    };

    draggable_chart(state, chart, content)
}

fn should_build_chart_plot(state: &TelemetryState, chart: ChartId) -> bool {
    !state.chart_collapsed[chart.index()] && state.dragging_chart != Some(chart)
}

fn draggable_chart<'a>(
    state: &TelemetryState,
    chart: ChartId,
    content: impl Into<Element<'a, TelemetryMessage>>,
) -> Element<'a, TelemetryMessage> {
    let maximized = state.maximized_chart == Some(chart);
    let interaction = if state.dragging_chart == Some(chart) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle: Element<'_, TelemetryMessage> = if maximized {
        Space::new()
            .width(Length::Fixed(CARD_HEADER_HEIGHT))
            .height(Length::Fixed(CARD_HEADER_HEIGHT))
            .into()
    } else {
        card_drag_handle(TelemetryMessage::BeginChartDrag(chart), interaction)
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
        TelemetryMessage::ToggleChartMaximized(chart),
        TelemetryMessage::ToggleChartCollapsed(chart),
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
    let card: Element<'_, TelemetryMessage> = if state.dragging_chart == Some(chart)
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
        |(chart, _), bounds, visible_bounds| TelemetryMessage::ChartLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        },
    )
}

fn analysis_panels<'a>(
    state: &'a TelemetryState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
    live_source: LiveTelemetrySourceInfo,
) -> (Element<'a, TelemetryMessage>, Element<'a, TelemetryMessage>) {
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
        "Unavailable"
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
                .then_some(TelemetryMessage::ToggleConnection),
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
            (state.ibt_load_state == IbtLoadState::Idle).then_some(TelemetryMessage::OpenIbt),
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
            .then_some(TelemetryMessage::OpenReferenceIbt),
    );
    let can_clear_reference = state.reference_ibt_load_state == IbtLoadState::Idle
        && (reference_session.is_some() || state.reference_ibt_error.is_some());
    let clear_reference_button = icon_button(lucide::x().size(16), "Clear reference")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Icon)
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(38.0))
        .padding(8)
        .on_press_maybe(can_clear_reference.then_some(TelemetryMessage::ClearReferenceIbt));
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
        TelemetryMessage::SelectReferenceLap,
        false,
    );
    let analysis_laps = lap_choice_list(
        &state.lap_choices,
        state.selected_lap_index,
        TelemetryMessage::SelectLap,
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
                .on_press(TelemetryMessage::ResetTelemetryLayout),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )]
    .width(Length::Fill);
    for chart in state.chart_order.iter().copied() {
        charts_content = charts_content.push(chart_list_item(state, chart));
    }

    let mut setup_content: [Option<Element<'_, TelemetryMessage>>; SetupCardId::COUNT] = [
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
    state: &TelemetryState,
    card: LapAnalysisCardId,
    sample: Option<TelemetrySample>,
    steering_angle_max: Option<f32>,
    focused: Option<FocusedTelemetry>,
    reference_focused: Option<FocusedTelemetry>,
) -> Element<'a, TelemetryMessage> {
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
) -> Element<'static, TelemetryMessage> {
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
    state: &TelemetryState,
    card: LapAnalysisCardId,
    content: impl Into<Element<'a, TelemetryMessage>>,
) -> Element<'a, TelemetryMessage> {
    let interaction = if state.dragging_lap_analysis_card == Some(card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle = card_drag_handle(TelemetryMessage::BeginLapAnalysisDrag(card), interaction);
    let highlighted = state.dragging_lap_analysis_card.is_some()
        && (state.dragging_lap_analysis_card == Some(card)
            || state.lap_analysis_drop_target == Some(card));
    let card_content = pane_card(
        CardTitle::new(card.title(), card.icon()),
        content,
        0.0,
        handle,
        state.lap_analysis_collapsed[card.index()],
        TelemetryMessage::ToggleLapAnalysisCardCollapsed(card),
        highlighted,
    );
    let card_content: Element<'_, TelemetryMessage> = if state.dragging_lap_analysis_card
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
        TelemetryMessage::LapAnalysisLayoutChanged {
            card,
            bounds,
            visible_bounds,
        }
    })
}

fn draggable_setup_card<'a>(
    state: &TelemetryState,
    card: SetupCardId,
    content: impl Into<Element<'a, TelemetryMessage>>,
) -> Element<'a, TelemetryMessage> {
    let interaction = if state.dragging_setup_card == Some(card) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    };
    let handle = card_drag_handle(TelemetryMessage::BeginSetupCardDrag(card), interaction);
    let highlighted = state.dragging_setup_card.is_some()
        && (state.dragging_setup_card == Some(card) || state.setup_card_drop_target == Some(card));
    let card_content = pane_card(
        CardTitle::new(card.title(), card.icon()),
        content,
        0.0,
        handle,
        state.setup_card_collapsed[card.index()],
        TelemetryMessage::ToggleSetupCardCollapsed(card),
        highlighted,
    );
    let card_content: Element<'_, TelemetryMessage> = if state.dragging_setup_card == Some(card)
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
        TelemetryMessage::SetupCardLayoutChanged {
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
) -> Element<'static, TelemetryMessage> {
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

fn setup_section_heading(label: &'static str) -> Element<'static, TelemetryMessage> {
    setup_separated_block(
        text(label)
            .size(12)
            .font(typography::SANS_SEMIBOLD)
            .color(TEXT_SECONDARY)
            .width(Length::Fill),
    )
}

fn setup_content_block<'a>(
    content: impl Into<Element<'a, TelemetryMessage>>,
) -> Element<'a, TelemetryMessage> {
    container(content)
        .padding(DATA_TEXT_INSET)
        .width(Length::Fill)
        .into()
}

fn setup_separated_block<'a>(
    content: impl Into<Element<'a, TelemetryMessage>>,
) -> Element<'a, TelemetryMessage> {
    column![
        setup_content_block(content),
        rule::horizontal(DATA_SEPARATOR_WIDTH).style(data_separator_style),
    ]
    .width(Length::Fill)
    .into()
}

fn setup_control_row<'a>(
    content: impl Into<Element<'a, TelemetryMessage>>,
    highlighted: bool,
) -> Element<'a, TelemetryMessage> {
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

fn chart_list_item<'a>(state: &'a TelemetryState, chart: ChartId) -> Element<'a, TelemetryMessage> {
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
            .on_toggle(move |visible| TelemetryMessage::ToggleChart(chart, visible)),
        card_drag_handle(TelemetryMessage::BeginChartListDrag(chart), interaction),
    ]
    .align_y(iced::Alignment::Center);
    let item = setup_control_row(content, highlighted);
    let item: Element<'_, TelemetryMessage> = if dragging
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
        |(chart, _), bounds, visible_bounds| TelemetryMessage::ChartListLayoutChanged {
            chart,
            bounds,
            visible_bounds,
        },
    )
}

fn chart_columns_button(columns: ChartColumns, active: bool) -> Element<'static, TelemetryMessage> {
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
        .on_press(TelemetryMessage::SetChartColumns(columns))
        .into()
}

fn cursor_value<'a>(
    label: &'static str,
    value: String,
    accent: Option<Color>,
) -> Element<'a, TelemetryMessage> {
    let value = match accent {
        Some(color) => text(value).size(18).font(typography::SANS).color(color),
        None => text(value).size(18).font(typography::SANS),
    };

    analysis_data_row(label, value.into())
}

fn input_badge_value(
    label: &'static str,
    value: Option<InputMeterValue>,
    variant: BadgeVariant,
    meter: InputMeter,
    meter_color: Color,
) -> Element<'static, TelemetryMessage> {
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
            .font(typography::SANS)
            .width(Length::Fixed(INPUT_BADGE_WIDTH))
            .into(),
    )
}

fn analysis_data_row<'a>(
    label: &'static str,
    value: Element<'a, TelemetryMessage>,
) -> Element<'a, TelemetryMessage> {
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

fn fixed_height<'a>(content: Element<'a, TelemetryMessage>) -> Element<'a, TelemetryMessage> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(CHART_HEIGHT))
        .into()
}

#[cfg(test)]
mod tests;
