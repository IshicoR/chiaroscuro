use std::time::{Duration, Instant};

mod chart_card;
mod style;

use chart_card::chart_card;
use chiaro_actions::Action;
use chiaro_theme::typography;
use chiaro_time_series_chart::{
    AxisSpec, LineSeries, TimeSeriesChart, TimeSeriesMessage, TimeSeriesSpec,
};
use chiaroscuro_irsdk::{SessionInfoDocument, SessionScalar, TelemetrySample};
use chiaroscuro_telemetry::{
    ConnectionStatus, FocusedTelemetry, HISTORY_WINDOW, Session, TelemetryLap,
};
use iced::{
    Background, Border, Color, Element, Length, Point, Shadow, Subscription, Theme, Vector, mouse,
    widget::{
        Space, button, checkbox, column, container, float, grid, mouse_area, row, rule, scrollable,
        stack, text, tooltip,
    },
};
use iced_fonts::lucide;
use iced_plot::{AxisLink, LineStyle};

const CHART_HEIGHT: f32 = 360.0;
const PRIMARY_CHART_LINE_WIDTH: f32 = 1.9;
const DYNAMICS_CHART_LINE_WIDTH: f32 = 1.7;
const WHEEL_CHART_LINE_WIDTH: f32 = 1.5;
const REFERENCE_CHART_LINE_WIDTH: f32 = 1.2;
const REFRESH_INTERVAL: Duration = Duration::from_millis(33);
const SETUP_PANEL_WIDTH: f32 = 248.0;
const ANALYSIS_PANEL_WIDTH: f32 = 280.0;
const LAP_LIST_VISIBLE_ROWS: usize = 6;
const LAP_CHOICE_HEIGHT: f32 = 32.0;
const LAP_CHOICE_SPACING: f32 = 3.0;
const DRAG_TRANSITION_DURATION: Duration = Duration::from_millis(140);
const DROP_TRANSITION_DURATION: Duration = Duration::from_millis(180);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);
const REFERENCE_AMBER: Color = Color::from_rgb(0.96, 0.76, 0.28);
const REFERENCE_CYAN: Color = Color::from_rgb(0.30, 0.78, 0.88);
const REFERENCE_PINK: Color = Color::from_rgb(0.94, 0.42, 0.66);
const REFERENCE_LAVENDER: Color = Color::from_rgb(0.69, 0.56, 0.94);

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartId {
    Speed,
    Pedal,
    Steering,
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
    const ALL: [Self; 12] = [
        Self::Speed,
        Self::Pedal,
        Self::Steering,
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

    const fn index(self) -> usize {
        self as usize
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Speed => "Speed",
            Self::Pedal => "Pedal",
            Self::Steering => "Steering",
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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum IbtLoadState {
    #[default]
    Idle,
    Selecting,
    Loading,
}

#[derive(Debug, Clone, PartialEq)]
struct LapChoice {
    index: usize,
    number: i32,
    duration_ms: i32,
    fuel_litres: Option<f32>,
    complete: bool,
}

impl LapChoice {
    fn new(index: usize, lap: TelemetryLap, fuel_litres: Option<f32>) -> Self {
        Self {
            index,
            number: lap.number(),
            duration_ms: lap.duration_ms(),
            fuel_litres,
            complete: lap.is_complete(),
        }
    }
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
    speed_chart: TimeSeriesChart,
    pedal_chart: TimeSeriesChart,
    steering_chart: TimeSeriesChart,
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
    ibt_load_state: IbtLoadState,
    reference_ibt_load_state: IbtLoadState,
    reference_ibt_error: Option<String>,
    lap_choices: Vec<LapChoice>,
    selected_lap_index: Option<usize>,
    reference_lap_choices: Vec<LapChoice>,
    selected_reference_lap_index: Option<usize>,
    session_metadata_update_count: Option<i32>,
    session_metadata: SessionMetadata,
    focus_x: Option<f64>,
    focused: Option<FocusedTelemetry>,
    focus_from_cursor: bool,
    live_follow: bool,
    chart_order: Vec<ChartId>,
    chart_visibility: [bool; ChartId::COUNT],
    chart_columns: ChartColumns,
    maximized_chart: Option<ChartId>,
    dragging_chart: Option<ChartId>,
    drop_target: Option<ChartId>,
    drag_origin: Option<Point>,
    drag_cursor: Option<Point>,
    drag_started_at: Option<Instant>,
    settling_chart: Option<ChartId>,
    settle_started_at: Option<Instant>,
    now: Instant,
}

impl Default for DashboardState {
    fn default() -> Self {
        let x_axis_link = AxisLink::new();
        let now = Instant::now();
        Self {
            speed_chart: build_speed_chart(x_axis_link.clone()),
            pedal_chart: build_pedal_chart(x_axis_link.clone()),
            steering_chart: build_steering_chart(x_axis_link.clone()),
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
            ibt_load_state: IbtLoadState::Idle,
            reference_ibt_load_state: IbtLoadState::Idle,
            reference_ibt_error: None,
            lap_choices: Vec::new(),
            selected_lap_index: None,
            reference_lap_choices: Vec::new(),
            selected_reference_lap_index: None,
            session_metadata_update_count: None,
            session_metadata: SessionMetadata::default(),
            focus_x: None,
            focused: None,
            focus_from_cursor: false,
            live_follow: true,
            chart_order: ChartId::ALL.to_vec(),
            chart_visibility: [true; ChartId::COUNT],
            chart_columns: ChartColumns::One,
            maximized_chart: None,
            dragging_chart: None,
            drop_target: None,
            drag_origin: None,
            drag_cursor: None,
            drag_started_at: None,
            settling_chart: None,
            settle_started_at: None,
            now,
        }
    }
}

impl DashboardState {
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

    fn drag_progress(&self) -> f32 {
        transition_progress(self.drag_started_at, self.now, DRAG_TRANSITION_DURATION)
    }

    fn settle_progress(&self) -> f32 {
        transition_progress(self.settle_started_at, self.now, DROP_TRANSITION_DURATION)
    }

    fn is_animating_chart_drag(&self) -> bool {
        self.dragging_chart.is_some()
            || (self.settling_chart.is_some() && self.settle_progress() < 1.0)
    }
}

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    ToggleConnection,
    OpenIbt,
    OpenReferenceIbt,
    ClearReferenceIbt,
    SelectLap(usize),
    SelectReferenceLap(usize),
    Refresh,
    SpeedPlot(TimeSeriesMessage),
    PedalPlot(TimeSeriesMessage),
    SteeringPlot(TimeSeriesMessage),
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
    BeginChartDrag(ChartId),
    HoverChart(ChartId),
    FinishChartDrag,
    SetChartColumns(ChartColumns),
    ToggleChartMaximized(ChartId),
    DragCursor(Point),
    DragAnimationFrame(Instant),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveChartNavigation {
    None,
    Browse,
    Follow,
}

fn live_chart_navigation(message: &TimeSeriesMessage) -> LiveChartNavigation {
    match message {
        TimeSeriesMessage::BeginPan | TimeSeriesMessage::Scroll(_) => LiveChartNavigation::Browse,
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
            if state.rendered_packets != session.packets_received() {
                sync_telemetry(state, session, reference_session);
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
        DashboardMessage::SteeringPlot(message) => {
            let navigation = live_chart_navigation(&message);
            let cursor_x = state.steering_chart.update(message);
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
            state.chart_visibility[chart.index()] = visible;
            if !visible && state.maximized_chart == Some(chart) {
                state.maximized_chart = None;
            }
            if !visible && state.dragging_chart == Some(chart) {
                state.dragging_chart = None;
                state.drop_target = None;
                state.drag_origin = None;
                state.drag_cursor = None;
                state.drag_started_at = None;
            }
            None
        },
        DashboardMessage::BeginChartDrag(chart) => {
            let now = Instant::now();
            state.now = now;
            state.dragging_chart = Some(chart);
            state.drop_target = Some(chart);
            state.drag_origin = None;
            state.drag_cursor = None;
            state.drag_started_at = Some(now);
            state.settling_chart = None;
            state.settle_started_at = None;
            None
        },
        DashboardMessage::HoverChart(chart) => {
            if state.dragging_chart.is_some() {
                state.drop_target = Some(chart);
            }
            None
        },
        DashboardMessage::FinishChartDrag => {
            if let (Some(dragging), Some(target)) = (state.dragging_chart, state.drop_target) {
                move_chart_to(&mut state.chart_order, dragging, target);
                let now = Instant::now();
                state.now = now;
                state.settling_chart = Some(dragging);
                state.settle_started_at = Some(now);
            }
            state.dragging_chart = None;
            state.drop_target = None;
            state.drag_origin = None;
            state.drag_cursor = None;
            state.drag_started_at = None;
            None
        },
        DashboardMessage::SetChartColumns(columns) => {
            state.chart_columns = columns;
            None
        },
        DashboardMessage::ToggleChartMaximized(chart) => {
            state.maximized_chart = (state.maximized_chart != Some(chart)).then_some(chart);
            state.dragging_chart = None;
            state.drop_target = None;
            state.drag_origin = None;
            state.drag_started_at = None;
            state.settling_chart = None;
            state.settle_started_at = None;
            None
        },
        DashboardMessage::DragCursor(position) => {
            if state.dragging_chart.is_some() {
                state.drag_origin.get_or_insert(position);
                state.drag_cursor = Some(position);
            }
            None
        },
        DashboardMessage::DragAnimationFrame(now) => {
            state.now = now;
            if state.settling_chart.is_some() && state.settle_progress() >= 1.0 {
                state.settling_chart = None;
                state.settle_started_at = None;
            }
            None
        },
    }
}

fn transition_progress(started_at: Option<Instant>, now: Instant, duration: Duration) -> f32 {
    let Some(started_at) = started_at else {
        return 0.0;
    };
    let linear = now.saturating_duration_since(started_at).as_secs_f32() / duration.as_secs_f32();
    let linear = linear.clamp(0.0, 1.0);
    1.0 - (1.0 - linear).powi(3)
}

fn move_chart_to(order: &mut Vec<ChartId>, chart: ChartId, target: ChartId) {
    let Some(from) = order.iter().position(|item| *item == chart) else {
        return;
    };
    let Some(to) = order.iter().position(|item| *item == target) else {
        return;
    };
    if from == to {
        return;
    }

    let chart = order.remove(from);
    order.insert(to.min(order.len()), chart);
}

pub fn subscription(state: &DashboardState, active: bool) -> Subscription<DashboardMessage> {
    let refresh = if active {
        iced::time::every(REFRESH_INTERVAL).map(|_| DashboardMessage::Refresh)
    } else {
        Subscription::none()
    };
    let finish_drag = iced::event::listen_with(|event, _, _| match event {
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | iced::Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
            Some(DashboardMessage::FinishChartDrag)
        },
        _ => None,
    });
    let drag_cursor = if state.dragging_chart.is_some() {
        iced::event::listen_with(|event, _, _| match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(DashboardMessage::DragCursor(position))
            },
            _ => None,
        })
    } else {
        Subscription::none()
    };
    let animation = if state.is_animating_chart_drag() {
        iced::time::every(ANIMATION_FRAME_INTERVAL).map(DashboardMessage::DragAnimationFrame)
    } else {
        Subscription::none()
    };

    Subscription::batch([refresh, finish_drag, drag_cursor, animation])
}

pub fn sync_telemetry(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    sync_session_metadata(state, session);

    let lap_index = state.selected_lap_index;
    let is_live = session.ibt_info().is_none();
    let chart_duration = session.chart_duration_seconds_for(lap_index);
    let live_bounds = session.live_chart_time_bounds();
    let (chart_min, chart_max) = if is_live {
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
    let live_follow = state.live_follow;
    for chart in [
        &mut state.speed_chart,
        &mut state.pedal_chart,
        &mut state.steering_chart,
        &mut state.rpm_chart,
        &mut state.gear_chart,
        &mut state.dynamics_chart,
        &mut state.yaw_chart,
        &mut state.wheel_slip_chart,
        &mut state.tyre_chart,
        &mut state.suspension_chart,
        &mut state.fuel_chart,
        &mut state.delta_chart,
    ] {
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

    state
        .speed_chart
        .set_series_points(0, &session.points_in(lap_index, |sample| sample.speed_kmh));

    state.pedal_chart.set_series_points(
        0,
        &session.points_in(lap_index, |sample| pedal_percent(sample.throttle)),
    );
    state.pedal_chart.set_series_points(
        1,
        &session.points_in(lap_index, |sample| pedal_percent(sample.brake)),
    );

    let steering_points = session.points_in(lap_index, |sample| sample.steering_angle.to_degrees());
    let steering_limits = symmetric_y_limits(steering_points.iter().map(|point| point[1]), 180.0);
    state
        .steering_chart
        .set_y_limits(steering_limits.0, steering_limits.1);
    state.steering_chart.set_series_points(0, &steering_points);

    let rpm_points = session.points_in(lap_index, |sample| sample.rpm.max(0) as f32);
    let rpm_max = maximum_y(&rpm_points, 8_000.0) * 1.1;
    state.rpm_chart.set_y_limits(0.0, rpm_max);
    state.rpm_chart.set_series_points(0, &rpm_points);
    let gear_points = session.gear_points(lap_index);
    state.gear_chart.set_series_points(0, &gear_points);

    let lateral_g = session.points_in(lap_index, |sample| sample.acceleration_g[0]);
    let longitudinal_g = session.points_in(lap_index, |sample| sample.acceleration_g[1]);
    let dynamics_limits = symmetric_y_limits(
        lateral_g
            .iter()
            .chain(&longitudinal_g)
            .map(|point| point[1]),
        3.6,
    );
    state
        .dynamics_chart
        .set_y_limits(dynamics_limits.0, dynamics_limits.1);
    state.dynamics_chart.set_series_points(0, &lateral_g);
    state.dynamics_chart.set_series_points(1, &longitudinal_g);

    let yaw_points = session.points_in(lap_index, |sample| sample.yaw_rate_rad_s.to_degrees());
    let yaw_limits = symmetric_y_limits(yaw_points.iter().map(|point| point[1]), 60.0);
    state.yaw_chart.set_y_limits(yaw_limits.0, yaw_limits.1);
    state.yaw_chart.set_series_points(0, &yaw_points);

    let mut wheel_slip_points = Vec::with_capacity(4);
    for wheel in 0..4 {
        wheel_slip_points
            .push(session.points_in(lap_index, |sample| sample.wheel_slip[wheel] * 100.0));
        state
            .wheel_slip_chart
            .set_series_points(wheel, &wheel_slip_points[wheel]);
    }
    let wheel_slip_limits = symmetric_y_limits(
        wheel_slip_points
            .iter()
            .flat_map(|points| points.iter())
            .map(|point| point[1]),
        20.0,
    );
    state
        .wheel_slip_chart
        .set_y_limits(wheel_slip_limits.0, wheel_slip_limits.1);

    for wheel in 0..4 {
        state.tyre_chart.set_series_points(
            wheel,
            &session.points_in(lap_index, |sample| sample.tyre_core_temperature_c[wheel]),
        );
    }

    let mut suspension_points = Vec::with_capacity(4);
    for wheel in 0..4 {
        suspension_points.push(session.points_in(lap_index, |sample| {
            sample.suspension_travel_m[wheel] * 1_000.0
        }));
        state
            .suspension_chart
            .set_series_points(wheel, &suspension_points[wheel]);
    }
    let suspension_limits = padded_y_limits(
        suspension_points
            .iter()
            .flat_map(|points| points.iter())
            .map(|point| point[1]),
        (-20.0, 120.0),
    );
    state
        .suspension_chart
        .set_y_limits(suspension_limits.0, suspension_limits.1);

    let fuel_points = session.fuel_used_points(lap_index);
    let fuel_max = maximum_y(&fuel_points, 1.0) * 1.2;
    state.fuel_chart.set_y_limits(0.0, fuel_max);
    state.fuel_chart.set_series_points(0, &fuel_points);

    let comparison = selected_comparison(state, session, reference_session);
    let reference_speed = comparison_points_for(session, comparison, |sample| sample.speed_kmh);
    state.speed_chart.set_series_points(1, &reference_speed);
    let reference_throttle =
        comparison_points_for(session, comparison, |sample| pedal_percent(sample.throttle));
    let reference_brake =
        comparison_points_for(session, comparison, |sample| pedal_percent(sample.brake));
    state.pedal_chart.set_series_points(2, &reference_throttle);
    state.pedal_chart.set_series_points(3, &reference_brake);

    let reference_steering = comparison_points_for(session, comparison, |sample| {
        sample.steering_angle.to_degrees()
    });
    state
        .steering_chart
        .set_series_points(1, &reference_steering);
    let steering_limits = symmetric_y_limits(
        steering_points
            .iter()
            .chain(&reference_steering)
            .map(|point| point[1]),
        180.0,
    );
    state
        .steering_chart
        .set_y_limits(steering_limits.0, steering_limits.1);

    let reference_rpm =
        comparison_points_for(session, comparison, |sample| sample.rpm.max(0) as f32);
    state.rpm_chart.set_series_points(1, &reference_rpm);
    let rpm_max = maximum_y(&rpm_points, maximum_y(&reference_rpm, 8_000.0)) * 1.1;
    state.rpm_chart.set_y_limits(0.0, rpm_max);
    let reference_gear = comparison.map_or_else(Vec::new, |(lap, reference, reference_lap)| {
        session.comparison_gear_points(lap, reference, reference_lap)
    });
    state.gear_chart.set_series_points(1, &reference_gear);

    let reference_lateral =
        comparison_points_for(session, comparison, |sample| sample.acceleration_g[0]);
    let reference_longitudinal =
        comparison_points_for(session, comparison, |sample| sample.acceleration_g[1]);
    state
        .dynamics_chart
        .set_series_points(2, &reference_lateral);
    state
        .dynamics_chart
        .set_series_points(3, &reference_longitudinal);
    let dynamics_limits = symmetric_y_limits(
        lateral_g
            .iter()
            .chain(&longitudinal_g)
            .chain(&reference_lateral)
            .chain(&reference_longitudinal)
            .map(|point| point[1]),
        3.6,
    );
    state
        .dynamics_chart
        .set_y_limits(dynamics_limits.0, dynamics_limits.1);

    let reference_yaw = comparison_points_for(session, comparison, |sample| {
        sample.yaw_rate_rad_s.to_degrees()
    });
    state.yaw_chart.set_series_points(1, &reference_yaw);
    let yaw_limits = symmetric_y_limits(
        yaw_points
            .iter()
            .chain(&reference_yaw)
            .map(|point| point[1]),
        60.0,
    );
    state.yaw_chart.set_y_limits(yaw_limits.0, yaw_limits.1);

    let reference_wheel_slip: [Vec<[f64; 2]>; 4] = std::array::from_fn(|wheel| {
        comparison_points_for(session, comparison, |sample| {
            sample.wheel_slip[wheel] * 100.0
        })
    });
    for (wheel, points) in reference_wheel_slip.iter().enumerate() {
        state.wheel_slip_chart.set_series_points(4 + wheel, points);
    }
    let wheel_slip_limits = symmetric_y_limits(
        wheel_slip_points
            .iter()
            .chain(&reference_wheel_slip)
            .flat_map(|points| points.iter())
            .map(|point| point[1]),
        20.0,
    );
    state
        .wheel_slip_chart
        .set_y_limits(wheel_slip_limits.0, wheel_slip_limits.1);

    let reference_tyres: [Vec<[f64; 2]>; 4] = std::array::from_fn(|wheel| {
        comparison_points_for(session, comparison, |sample| {
            sample.tyre_core_temperature_c[wheel]
        })
    });
    for (wheel, points) in reference_tyres.iter().enumerate() {
        state.tyre_chart.set_series_points(4 + wheel, points);
    }

    let reference_suspension: [Vec<[f64; 2]>; 4] = std::array::from_fn(|wheel| {
        comparison_points_for(session, comparison, |sample| {
            sample.suspension_travel_m[wheel] * 1_000.0
        })
    });
    for (wheel, points) in reference_suspension.iter().enumerate() {
        state.suspension_chart.set_series_points(4 + wheel, points);
    }
    let suspension_limits = padded_y_limits(
        suspension_points
            .iter()
            .chain(&reference_suspension)
            .flat_map(|points| points.iter())
            .map(|point| point[1]),
        (-20.0, 120.0),
    );
    state
        .suspension_chart
        .set_y_limits(suspension_limits.0, suspension_limits.1);

    let reference_fuel = comparison.map_or_else(Vec::new, |(lap, reference, reference_lap)| {
        session.comparison_fuel_used_points(lap, reference, reference_lap)
    });
    state.fuel_chart.set_series_points(1, &reference_fuel);
    let fuel_max = maximum_y(&fuel_points, maximum_y(&reference_fuel, 1.0)) * 1.2;
    state.fuel_chart.set_y_limits(0.0, fuel_max);

    let delta_points = comparison.map_or_else(
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
    let delta_limits = symmetric_y_limits(delta_points.iter().map(|point| point[1]), 6.0);
    state
        .delta_chart
        .set_y_limits(delta_limits.0, delta_limits.1);
    state.delta_chart.set_series_points(0, &delta_points);

    state.rendered_packets = session.packets_received();
    let focus_target = if is_live {
        state.focus_from_cursor = false;
        let latest = live_bounds.map_or(0.0, |(_, latest)| latest);
        if state.live_follow {
            latest
        } else {
            state.speed_chart.x_view_max().clamp(chart_min, latest)
        }
    } else if state.focus_from_cursor {
        state.focus_x.unwrap_or(chart_duration)
    } else {
        chart_duration
    };
    focus_at(state, session, focus_target);
}

pub fn reset_telemetry(
    state: &mut DashboardState,
    session: &Session,
    reference_session: Option<&Session>,
) {
    state.session_metadata_update_count = None;
    state.session_metadata = SessionMetadata::default();
    state.lap_choices = session
        .laps()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, lap)| LapChoice::new(index, lap, session.lap_start_fuel_litres(index)))
        .collect();
    state.selected_lap_index = session.preferred_lap_index();
    state.focus_x = None;
    state.focused = None;
    state.focus_from_cursor = false;
    state.live_follow = true;
    sync_telemetry(state, session, reference_session);
}

fn sync_session_metadata(state: &mut DashboardState, session: &Session) {
    let update_count = session.session_info().map(|info| info.update_count);
    if state.session_metadata_update_count == update_count {
        return;
    }

    state.session_metadata_update_count = update_count;
    state.session_metadata = session
        .session_info()
        .and_then(|info| info.parse().ok())
        .as_ref()
        .map_or_else(SessionMetadata::default, session_metadata);
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
            .map(|(index, lap)| LapChoice::new(index, lap, None))
            .collect()
    });
    state.selected_reference_lap_index = reference_session.and_then(Session::preferred_lap_index);
    state.reference_ibt_error = None;
    sync_telemetry(state, session, reference_session);
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
    let focused = session.focused_telemetry(state.selected_lap_index, x);
    state.focus_x = focused.map(|point| point.elapsed_seconds);
    state.focused = focused;
    let focused_index = focused.map(|point| point.point_index);
    state.speed_chart.set_focus_index(focused_index);
    state.pedal_chart.set_focus_index(focused_index);
    state.steering_chart.set_focus_index(focused_index);
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
            .spacing(12)
            .height(Length::Shrink);
        scrollable(chart_grid)
            .spacing(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let (analysis_setup, lap_analysis) = analysis_panels(state, session, reference_session);

    row![analysis_setup, charts, lap_analysis]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn chart_view<'a>(state: &'a DashboardState, chart: ChartId) -> Element<'a, DashboardMessage> {
    let focused_x = state.focus_x;
    match chart {
        ChartId::Speed => draggable_chart(
            state,
            chart,
            state
                .speed_chart
                .view(focused_x)
                .map(DashboardMessage::SpeedPlot),
        ),
        ChartId::Pedal => draggable_chart(
            state,
            chart,
            state
                .pedal_chart
                .view(focused_x)
                .map(DashboardMessage::PedalPlot),
        ),
        ChartId::Steering => draggable_chart(
            state,
            chart,
            state
                .steering_chart
                .view(focused_x)
                .map(DashboardMessage::SteeringPlot),
        ),
        ChartId::Rpm => draggable_chart(
            state,
            chart,
            state
                .rpm_chart
                .view(focused_x)
                .map(DashboardMessage::RpmPlot),
        ),
        ChartId::Gear => draggable_chart(
            state,
            chart,
            state
                .gear_chart
                .view(focused_x)
                .map(DashboardMessage::GearPlot),
        ),
        ChartId::Dynamics => draggable_chart(
            state,
            chart,
            state
                .dynamics_chart
                .view(focused_x)
                .map(DashboardMessage::DynamicsPlot),
        ),
        ChartId::Yaw => draggable_chart(
            state,
            chart,
            state
                .yaw_chart
                .view(focused_x)
                .map(DashboardMessage::YawPlot),
        ),
        ChartId::WheelSlip => draggable_chart(
            state,
            chart,
            state
                .wheel_slip_chart
                .view(focused_x)
                .map(DashboardMessage::WheelSlipPlot),
        ),
        ChartId::Tyre => draggable_chart(
            state,
            chart,
            state
                .tyre_chart
                .view(focused_x)
                .map(DashboardMessage::TyrePlot),
        ),
        ChartId::Suspension => draggable_chart(
            state,
            chart,
            state
                .suspension_chart
                .view(focused_x)
                .map(DashboardMessage::SuspensionPlot),
        ),
        ChartId::Fuel => draggable_chart(
            state,
            chart,
            state
                .fuel_chart
                .view(focused_x)
                .map(DashboardMessage::FuelPlot),
        ),
        ChartId::Delta => draggable_chart(
            state,
            chart,
            state
                .delta_chart
                .view(focused_x)
                .map(DashboardMessage::DeltaPlot),
        ),
    }
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
        Space::new().width(Length::Fixed(30.0)).into()
    } else {
        tooltip(
            mouse_area(container(lucide::grip_vertical().size(18)).padding(6))
                .on_press(DashboardMessage::BeginChartDrag(chart))
                .interaction(interaction),
            container(text("Drag to reorder").size(14)).padding(6),
            tooltip::Position::Left,
        )
        .into()
    };
    let highlighted = state.dragging_chart.is_some()
        && state.dragging_chart != Some(chart)
        && state.drop_target == Some(chart);
    let lift = if state.dragging_chart == Some(chart) {
        state.drag_progress()
    } else {
        0.0
    };
    let card = chart_card(
        chart.title(),
        content,
        handle,
        DashboardMessage::ToggleChartMaximized(chart),
        highlighted || state.dragging_chart == Some(chart),
        lift,
    );
    let card = if maximized {
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        fixed_height(card)
    };
    let card: Element<'_, DashboardMessage> = mouse_area(card)
        .on_enter(DashboardMessage::HoverChart(chart))
        .into();

    if state.dragging_chart == Some(chart)
        && let (Some(origin), Some(cursor)) = (state.drag_origin, state.drag_cursor)
    {
        let progress = state.drag_progress();
        float(card)
            .translate(move |_, _| Vector::new(cursor.x - origin.x, cursor.y - origin.y))
            .style(move |_| iced::widget::float::Style {
                shadow: Shadow {
                    color: Color {
                        a: 0.3 * progress,
                        ..Color::BLACK
                    },
                    offset: Vector::new(0.0, 7.0 * progress),
                    blur_radius: 18.0 * progress,
                },
                shadow_border_radius: 8.0.into(),
            })
            .into()
    } else if state.settling_chart == Some(chart) {
        let remaining = 1.0 - state.settle_progress();
        float(card)
            .scale(1.0 + 0.012 * remaining)
            .style(move |_| iced::widget::float::Style {
                shadow: Shadow {
                    color: Color {
                        a: 0.22 * remaining,
                        ..Color::BLACK
                    },
                    offset: Vector::new(0.0, 4.0 * remaining),
                    blur_radius: 12.0 * remaining,
                },
                shadow_border_radius: 8.0.into(),
            })
            .into()
    } else {
        card
    }
}

fn analysis_panels<'a>(
    state: &'a DashboardState,
    session: &'a Session,
    reference_session: Option<&'a Session>,
) -> (Element<'a, DashboardMessage>, Element<'a, DashboardMessage>) {
    let sample = state
        .focused
        .map(|focused| focused.sample)
        .or_else(|| session.latest().copied())
        .unwrap_or_default();
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
        ("IBT", Color::from_rgb(0.23, 0.55, 0.95))
    } else {
        match session.connection() {
            ConnectionStatus::Disconnected => ("Disconnected", Color::from_rgb(0.52, 0.55, 0.60)),
            ConnectionStatus::Connecting => ("Waiting", Color::from_rgb(0.95, 0.62, 0.12)),
            ConnectionStatus::Connected => ("Live", Color::from_rgb(0.12, 0.72, 0.38)),
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
    let source_detail = session.last_error().unwrap_or(connection_label);

    let connection_action = if session.wants_connection() {
        "Disconnect"
    } else {
        "Connect"
    };
    let connection_button = button(connection_action)
        .width(Length::Fill)
        .height(Length::Fixed(38.0))
        .style(setup_outline_button_style);
    let connection_button = if state.ibt_load_state == IbtLoadState::Idle {
        connection_button.on_press(DashboardMessage::ToggleConnection)
    } else {
        connection_button
    };
    let open_label = match state.ibt_load_state {
        IbtLoadState::Idle => "Open IBT",
        IbtLoadState::Selecting => "Selecting...",
        IbtLoadState::Loading => "Loading...",
    };
    let open_button = button(lucide::folder_open().size(17))
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(38.0))
        .padding(10)
        .style(setup_outline_button_style);
    let open_button = if state.ibt_load_state == IbtLoadState::Idle {
        open_button.on_press(DashboardMessage::OpenIbt)
    } else {
        open_button
    };
    let open_button = tooltip(
        open_button,
        container(text(open_label).size(14)).padding(6),
        tooltip::Position::Top,
    );

    let reference_open_label = match state.reference_ibt_load_state {
        IbtLoadState::Idle => "Open IBT",
        IbtLoadState::Selecting => "Selecting...",
        IbtLoadState::Loading => "Loading...",
    };
    let reference_open_button = button(
        stack([
            container(lucide::folder_open().size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
            container(text(reference_open_label).size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(38.0))
    .style(reference_action_button_style);
    let reference_open_button = if state.reference_ibt_load_state == IbtLoadState::Idle {
        reference_open_button.on_press(DashboardMessage::OpenReferenceIbt)
    } else {
        reference_open_button
    };
    let clear_reference_button = button(lucide::x().size(16))
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(38.0))
        .padding(8)
        .style(setup_outline_button_style);
    let clear_reference_button = if state.reference_ibt_load_state == IbtLoadState::Idle
        && (reference_session.is_some() || state.reference_ibt_error.is_some())
    {
        clear_reference_button.on_press(DashboardMessage::ClearReferenceIbt)
    } else {
        clear_reference_button
    };
    let clear_reference_button = tooltip(
        clear_reference_button,
        container(text("Clear reference").size(14)).padding(6),
        tooltip::Position::Top,
    );
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
        Color::from_rgb(0.90, 0.32, 0.38)
    } else if reference_session.is_some_and(|reference| cars_are_different(session, reference)) {
        Color::from_rgb(0.95, 0.62, 0.12)
    } else {
        Color::from_rgb(0.52, 0.55, 0.60)
    };

    let mut session_details = column![
        text(track_name)
            .size(19)
            .font(typography::SANS_SEMIBOLD)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        rule::horizontal(1),
    ]
    .spacing(8)
    .width(Length::Fill);
    for (label, value) in [
        ("Car", Some(car_name)),
        ("Class", metadata.car_class.clone()),
        ("Session", Some(session_type)),
        ("Time", Some(time_label)),
        ("Date", metadata.date_time.clone()),
        ("Layout", track_config),
        ("Length", track_length),
        ("Type", metadata.track_type.clone()),
    ] {
        if let Some(value) = value {
            session_details = session_details.push(metadata_row(label, value, None));
        }
    }
    session_details = session_details
        .push(metadata_row(
            "Source",
            source_detail.to_owned(),
            Some(connection_color),
        ))
        .push(setup_section_heading("CONDITIONS"));
    for (label, value) in [
        (
            "Weather",
            Some(metadata.weather.as_deref().unwrap_or("--").to_owned()),
        ),
        (
            "Air temp",
            Some(
                metadata
                    .air_temperature
                    .as_deref()
                    .unwrap_or("--")
                    .to_owned(),
            ),
        ),
        (
            "Track temp",
            Some(
                metadata
                    .surface_temperature
                    .as_deref()
                    .unwrap_or("--")
                    .to_owned(),
            ),
        ),
        ("Humidity", metadata.humidity.clone()),
        ("Wind", metadata.wind.clone()),
    ] {
        if let Some(value) = value {
            session_details = session_details.push(metadata_row(label, value, None));
        }
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

    let mut setup = column![
        session_details,
        row![connection_button, open_button]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        setup_section_heading("REFERENCE"),
        container(
            text(reference_description)
                .size(14)
                .color(reference_color)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        )
        .padding(9)
        .width(Length::Fill)
        .style(setup_callout_style),
        row![reference_open_button, clear_reference_button]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        reference_laps,
        setup_section_heading("MY LAPS"),
        container(
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
        .style(setup_callout_style),
        analysis_laps,
        setup_section_heading("CHARTS"),
        row![
            text("Layout")
                .size(14)
                .font(typography::SANS_SEMIBOLD)
                .width(Length::Fill),
            chart_columns_button(ChartColumns::One, state.chart_columns == ChartColumns::One,),
            chart_columns_button(ChartColumns::Two, state.chart_columns == ChartColumns::Two,),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(10);
    for chart in state.chart_order.iter().copied() {
        setup = setup.push(
            checkbox(state.chart_visibility[chart.index()])
                .label(chart.title())
                .size(16)
                .text_size(14)
                .width(Length::Fill)
                .on_toggle(move |visible| DashboardMessage::ToggleChart(chart, visible)),
        );
    }

    let mut readout = column![].spacing(8);
    if let Some(focused) = state.focused {
        readout = readout
            .push(text("Cursor").size(14).font(typography::SANS_SEMIBOLD))
            .push(cursor_value(
                "Time",
                format_lap_time(focused.sample.current_lap_ms),
                None,
            ))
            .push(cursor_value(
                "Lap position",
                format_track_position(sample.normalized_car_position),
                None,
            ))
            .push(rule::horizontal(1));
    }
    if let Some(reference) = reference_focused {
        let reference_sample = reference.sample;
        readout = readout
            .push(
                text("Reference cursor")
                    .size(14)
                    .font(typography::SANS_SEMIBOLD),
            )
            .push(cursor_value(
                "Time",
                format_lap_time(reference_sample.current_lap_ms),
                None,
            ))
            .push(cursor_value(
                "Speed",
                format!("{:.1} km/h", reference_sample.speed_kmh),
                Some(Color::from_rgb(0.18, 0.65, 0.95)),
            ))
            .push(cursor_value(
                "Throttle",
                format!("{:.1}%", pedal_percent(reference_sample.throttle)),
                Some(Color::from_rgb(0.12, 0.72, 0.38)),
            ))
            .push(cursor_value(
                "Brake",
                format!("{:.1}%", pedal_percent(reference_sample.brake)),
                Some(Color::from_rgb(0.90, 0.24, 0.24)),
            ))
            .push(rule::horizontal(1));
    }
    let readout = readout
        .push(text("Vehicle").size(14).font(typography::SANS_SEMIBOLD))
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
        .push(rule::horizontal(1))
        .push(text("Inputs").size(14).font(typography::SANS_SEMIBOLD))
        .push(cursor_value(
            "Throttle",
            format!("{:.1}%", pedal_percent(sample.throttle)),
            Some(Color::from_rgb(0.12, 0.72, 0.38)),
        ))
        .push(cursor_value(
            "Brake",
            format!("{:.1}%", pedal_percent(sample.brake)),
            Some(Color::from_rgb(0.90, 0.24, 0.24)),
        ))
        .push(cursor_value(
            "Steering",
            format!("{:.1}°", sample.steering_angle.to_degrees()),
            None,
        ))
        .push(rule::horizontal(1))
        .push(text("Dynamics").size(14).font(typography::SANS_SEMIBOLD))
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
        .push(rule::horizontal(1))
        .push(text("Tyres").size(14).font(typography::SANS_SEMIBOLD))
        .push(cursor_value(
            "FL tyre",
            format!("{:.1}°C", sample.tyre_core_temperature_c[0]),
            None,
        ))
        .push(cursor_value(
            "FR tyre",
            format!("{:.1}°C", sample.tyre_core_temperature_c[1]),
            None,
        ))
        .push(cursor_value(
            "RL tyre",
            format!("{:.1}°C", sample.tyre_core_temperature_c[2]),
            None,
        ))
        .push(cursor_value(
            "RR tyre",
            format!("{:.1}°C", sample.tyre_core_temperature_c[3]),
            None,
        ))
        .push(rule::horizontal(1))
        .push(text("Wheels").size(14).font(typography::SANS_SEMIBOLD));
    let readout = ["FL", "FR", "RL", "RR"].into_iter().enumerate().fold(
        readout,
        |readout, (wheel, label)| {
            readout
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
        },
    );

    let setup = container(
        scrollable(setup)
            .spacing(8)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(12)
    .width(Length::Fixed(SETUP_PANEL_WIDTH))
    .height(Length::Fill)
    .style(setup_panel_style);

    let analysis = container(
        column![
            text("Lap analysis")
                .size(19)
                .font(typography::SANS_SEMIBOLD),
            rule::horizontal(1),
            scrollable(readout)
                .spacing(8)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(12)
    .width(Length::Fixed(ANALYSIS_PANEL_WIDTH))
    .height(Length::Fill)
    .style(analysis_panel_style);

    (setup.into(), analysis.into())
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

    row![text(label).size(13).width(Length::Fixed(72.0)), value,]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn setup_section_heading(label: &'static str) -> Element<'static, DashboardMessage> {
    row![
        text(label).size(13).font(typography::SANS_SEMIBOLD),
        rule::horizontal(1),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn lap_list_viewport_height(lap_count: usize) -> f32 {
    let visible_rows = lap_count.min(LAP_LIST_VISIBLE_ROWS);
    if visible_rows == 0 {
        return 0.0;
    }

    LAP_CHOICE_HEIGHT * visible_rows as f32
        + LAP_CHOICE_SPACING * visible_rows.saturating_sub(1) as f32
}

fn lap_choice_list<'a>(
    choices: &'a [LapChoice],
    selected_index: Option<usize>,
    on_select: fn(usize) -> DashboardMessage,
    show_fuel: bool,
) -> Element<'a, DashboardMessage> {
    if choices.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let laps = choices
        .iter()
        .fold(column![].spacing(LAP_CHOICE_SPACING), |laps, choice| {
            let selected = selected_index == Some(choice.index);
            let duration = if choice.complete {
                format_lap_time(choice.duration_ms)
            } else {
                format!(
                    "{} partial",
                    format_elapsed_milliseconds(choice.duration_ms)
                )
            };
            let marker: Element<'_, DashboardMessage> = if selected {
                lucide::check().size(15).into()
            } else {
                Space::new().width(Length::Fixed(15.0)).into()
            };
            let choice_index = choice.index;
            let mut content = row![
                text(choice.number)
                    .size(14)
                    .font(typography::MONO_SEMIBOLD)
                    .width(Length::Fixed(28.0)),
                text(duration)
                    .size(15)
                    .font(typography::MONO_SEMIBOLD)
                    .width(Length::Fill),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center);
            if show_fuel {
                content = content.push(
                    text(format_lap_fuel(choice.fuel_litres))
                        .size(13)
                        .font(typography::MONO_SEMIBOLD)
                        .width(Length::Fixed(56.0))
                        .align_x(iced::alignment::Horizontal::Right),
                );
            }
            let content = content.push(marker);

            laps.push(
                button(content)
                    .padding([7, 9])
                    .height(Length::Fixed(LAP_CHOICE_HEIGHT))
                    .width(Length::Fill)
                    .style(move |theme, status| {
                        lap_choice_button_style(theme, status, selected, choice.complete)
                    })
                    .on_press(on_select(choice_index)),
            )
        });

    scrollable(laps)
        .width(Length::Fill)
        .height(Length::Fixed(lap_list_viewport_height(choices.len())))
        .spacing(4)
        .anchor_bottom()
        .into()
}

fn format_lap_fuel(fuel_litres: Option<f32>) -> String {
    fuel_litres.map_or_else(
        || "-- L".to_owned(),
        |fuel_litres| format!("{fuel_litres:.1} L"),
    )
}

fn chart_columns_button(columns: ChartColumns, active: bool) -> Element<'static, DashboardMessage> {
    let icon = match columns {
        ChartColumns::One => lucide::layout_list(),
        ChartColumns::Two => lucide::layout_grid(),
    }
    .size(16);
    let button = button(icon)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(6)
        .style(move |theme, status| chart_layout_button_style(theme, status, active))
        .on_press(DashboardMessage::SetChartColumns(columns));
    let label = match columns {
        ChartColumns::One => "Single column",
        ChartColumns::Two => "Two columns",
    };

    tooltip(
        button,
        container(text(label).size(14)).padding(6),
        tooltip::Position::Top,
    )
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

    row![
        text(label).size(14).width(Length::Fill),
        Space::new().width(Length::Fixed(4.0)),
        value,
    ]
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn analysis_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        text_color: Some(palette.background.weakest.text),
        border: Border {
            color: Color {
                a: 0.5,
                ..palette.background.strong.color
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

fn setup_panel_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weakest.color.into()),
        text_color: Some(palette.background.weakest.text),
        border: Border {
            color: with_alpha(palette.background.strong.color, 0.5),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

fn setup_callout_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weaker.color.into()),
        text_color: Some(palette.background.weaker.text),
        border: Border {
            color: with_alpha(palette.background.strong.color, 0.5),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn setup_outline_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    let disabled = status == iced::widget::button::Status::Disabled;

    iced::widget::button::Style {
        background: hovered.then(|| palette.background.weak.color.into()),
        text_color: if disabled {
            with_alpha(palette.background.base.text, 0.4)
        } else {
            palette.background.base.text
        },
        border: Border {
            color: if disabled {
                with_alpha(palette.background.strong.color, 0.3)
            } else {
                with_alpha(palette.background.strong.color, 0.8)
            },
            width: 1.0,
            radius: 5.0.into(),
        },
        ..iced::widget::button::Style::default()
    }
}

fn reference_action_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let disabled = status == iced::widget::button::Status::Disabled;
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );

    iced::widget::button::Style {
        background: hovered.then(|| palette.primary.weak.color.into()),
        text_color: if disabled {
            with_alpha(palette.primary.base.color, 0.4)
        } else {
            palette.primary.base.color
        },
        border: Border {
            color: if disabled {
                with_alpha(palette.primary.base.color, 0.3)
            } else {
                palette.primary.base.color
            },
            width: 1.5,
            radius: 5.0.into(),
        },
        ..iced::widget::button::Style::default()
    }
}

fn lap_choice_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
    complete: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    let background = if selected {
        palette.primary.base.color
    } else if hovered {
        palette.background.weak.color
    } else {
        palette.background.weaker.color
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: if complete || selected {
            if selected {
                palette.primary.base.text
            } else {
                palette.background.weaker.text
            }
        } else {
            with_alpha(palette.background.weaker.text, 0.45)
        },
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..iced::widget::button::Style::default()
    }
}

fn chart_layout_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = status == iced::widget::button::Status::Hovered;

    iced::widget::button::Style {
        background: Some(Background::Color(if active {
            palette.primary.base.color
        } else if hovered {
            palette.background.weak.color
        } else {
            palette.background.weaker.color
        })),
        text_color: if active {
            palette.primary.base.text
        } else {
            palette.background.weaker.text
        },
        border: Border {
            color: if active {
                palette.primary.strong.color
            } else {
                with_alpha(palette.background.strong.color, 0.55)
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        ..iced::widget::button::Style::default()
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
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
    let throttle_color = Color::from_rgb(0.12, 0.72, 0.38);
    let brake_color = Color::from_rgb(0.90, 0.24, 0.24);
    let throttle = LineSeries::new(
        placeholder(),
        "Throttle",
        throttle_color,
        LineStyle::solid().with_pixel_width(PRIMARY_CHART_LINE_WIDTH),
    );
    let brake = LineSeries::new(
        placeholder(),
        "Brake",
        brake_color,
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

fn build_steering_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.20, 0.72, 0.68);
    let steering = LineSeries::new(
        placeholder(),
        "Steering angle",
        color,
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

fn format_lap_time(milliseconds: i32) -> String {
    if milliseconds <= 0 {
        return "--:--.---".to_owned();
    }

    format_elapsed_milliseconds(milliseconds)
}

fn format_elapsed_milliseconds(milliseconds: i32) -> String {
    let milliseconds = milliseconds.max(0);

    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1_000;
    let millis = milliseconds % 1_000;
    format!("{minutes}:{seconds:02}.{millis:03}")
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chiaro_actions::Action;
    use chiaro_time_series_chart::TimeSeriesMessage;
    use chiaroscuro_irsdk::{
        SessionInfo, SessionScalar, TelemetryFrame, TelemetrySample, TelemetryValue,
    };
    use chiaroscuro_telemetry::{IbtInfo, LoadedIbt, Session, TimedSample};

    use super::{
        ChartColumns, ChartId, DROP_TRANSITION_DURATION, DashboardMessage, DashboardState,
        LiveChartNavigation, focus_at, format_chart_time, format_gear, format_lap_time,
        format_position, format_recording_duration, format_session_time, format_track_position,
        lap_list_viewport_height, move_chart_to, pedal_percent, reset_reference_telemetry,
        reset_telemetry as reset_dashboard_telemetry, selected_comparison, session_metadata,
        symmetric_y_limits, update as update_dashboard, update_chart_focus,
    };
    fn update(
        state: &mut DashboardState,
        session: &Session,
        message: DashboardMessage,
    ) -> Option<Action> {
        update_dashboard(state, session, None, message)
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
            Vec::<chiaroscuro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();
        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                path: PathBuf::from(file_name),
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
    fn lap_list_viewport_is_capped_at_six_rows() {
        assert_eq!(lap_list_viewport_height(0), 0.0);
        assert_eq!(lap_list_viewport_height(1), 32.0);
        assert_eq!(lap_list_viewport_height(6), 207.0);
        assert_eq!(lap_list_viewport_height(7), 207.0);
    }

    #[test]
    fn refreshes_charts_only_after_new_packets() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        let _action = update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.rendered_packets, 1);
    }

    #[test]
    fn live_chart_browsing_pauses_and_reset_restores_latest_following() {
        let mut state = DashboardState::default();
        let session = Session::default();

        let _action = update(
            &mut state,
            &session,
            DashboardMessage::SpeedPlot(TimeSeriesMessage::BeginPan),
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
    fn chart_visibility_and_drag_order_are_updated_by_messages() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::ToggleChart(ChartId::Fuel, false),
        );
        assert!(!state.chart_visibility[ChartId::Fuel.index()]);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::HoverChart(ChartId::Gear),
        );
        update(&mut state, &session, DashboardMessage::FinishChartDrag);

        assert_eq!(
            &state.chart_order[..5],
            &[
                ChartId::Pedal,
                ChartId::Steering,
                ChartId::Rpm,
                ChartId::Gear,
                ChartId::Speed,
            ]
        );
        assert_eq!(state.dragging_chart, None);
        assert_eq!(state.drop_target, None);
        assert_eq!(state.settling_chart, Some(ChartId::Speed));
    }

    #[test]
    fn moving_a_chart_up_inserts_it_at_the_target_position() {
        let mut order = ChartId::ALL.to_vec();

        move_chart_to(&mut order, ChartId::Fuel, ChartId::Pedal);

        assert_eq!(order[1], ChartId::Fuel);
        assert_eq!(order[2], ChartId::Pedal);
    }

    #[test]
    fn chart_layout_and_drag_transition_state_are_updated() {
        let mut state = DashboardState::default();
        let session = Session::default();

        update(
            &mut state,
            &session,
            DashboardMessage::SetChartColumns(ChartColumns::Two),
        );
        assert_eq!(state.chart_columns, ChartColumns::Two);

        update(
            &mut state,
            &session,
            DashboardMessage::BeginChartDrag(ChartId::Speed),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(iced::Point::new(10.0, 20.0)),
        );
        update(
            &mut state,
            &session,
            DashboardMessage::DragCursor(iced::Point::new(30.0, 50.0)),
        );
        assert_eq!(state.drag_origin, Some(iced::Point::new(10.0, 20.0)));
        assert_eq!(state.drag_cursor, Some(iced::Point::new(30.0, 50.0)));

        update(&mut state, &session, DashboardMessage::FinishChartDrag);
        let animation_end =
            state.settle_started_at.expect("drop starts a transition") + DROP_TRANSITION_DURATION;
        update(
            &mut state,
            &session,
            DashboardMessage::DragAnimationFrame(animation_end),
        );
        assert_eq!(state.settling_chart, None);
        assert_eq!(state.settle_started_at, None);
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
            Vec::<chiaroscuro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();
        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                path: PathBuf::from("laps.ibt"),
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
        assert_eq!(state.focus_x, Some(50.0));

        let action = update(&mut state, &session, DashboardMessage::SelectLap(1));

        assert_eq!(action, None);
        assert_eq!(state.selected_lap_index, Some(1));
        assert_eq!(state.focus_x, Some(9.0));

        state.focus_from_cursor = true;
        focus_at(&mut state, &session, 4.2);

        assert_eq!(state.focus_x, Some(0.0));
        assert_eq!(state.focused.map(|point| point.elapsed_seconds), Some(0.0));
        assert_eq!(state.steering_chart.focus_index(), Some(0));
        assert_eq!(state.rpm_chart.focus_index(), Some(0));
        assert_eq!(state.gear_chart.focus_index(), Some(0));
        assert_eq!(state.yaw_chart.focus_index(), Some(0));
        assert_eq!(state.wheel_slip_chart.focus_index(), Some(0));
        assert_eq!(state.suspension_chart.focus_index(), Some(0));
        assert_eq!(state.fuel_chart.focus_index(), Some(0));
    }
}
