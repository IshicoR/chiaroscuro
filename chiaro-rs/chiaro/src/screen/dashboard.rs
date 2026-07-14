use std::time::Duration;

use iced::{
    Color, Element, Length, Subscription,
    widget::{button, column, container, row, scrollable, text},
};
use iced_plot::{LineStyle, PlotUiMessage};

use crate::{
    action::Action,
    appearance::{self, HISTORY_WINDOW},
    session::{ConnectionStatus, Session},
    widget::telemetry::{
        AxisSpec, LineSeries, TimeSeriesChart, TimeSeriesSpec, chart_card, metric_card,
    },
};

const CHART_HEIGHT: f32 = 280.0;
const REFRESH_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug)]
pub struct DashboardState {
    pedal_chart: TimeSeriesChart,
    dynamics_chart: TimeSeriesChart,
    tyre_chart: TimeSeriesChart,
    rendered_packets: u64,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            pedal_chart: build_pedal_chart(),
            dynamics_chart: build_dynamics_chart(),
            tyre_chart: build_tyre_chart(),
            rendered_packets: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    ToggleConnection,
    Refresh,
    PedalPlot(PlotUiMessage),
    DynamicsPlot(PlotUiMessage),
    TyrePlot(PlotUiMessage),
}

pub fn update(
    state: &mut DashboardState,
    session: &Session,
    message: DashboardMessage,
) -> Option<Action> {
    match message {
        DashboardMessage::ToggleConnection => {
            Some(Action::SetConnected(!session.wants_connection()))
        },
        DashboardMessage::Refresh => {
            if state.rendered_packets != session.packets_received() {
                sync_telemetry(state, session);
            }
            None
        },
        DashboardMessage::PedalPlot(message) => {
            state.pedal_chart.update(message);
            None
        },
        DashboardMessage::DynamicsPlot(message) => {
            state.dynamics_chart.update(message);
            None
        },
        DashboardMessage::TyrePlot(message) => {
            state.tyre_chart.update(message);
            None
        },
    }
}

pub fn subscription(active: bool) -> Subscription<DashboardMessage> {
    if active {
        iced::time::every(REFRESH_INTERVAL).map(|_| DashboardMessage::Refresh)
    } else {
        Subscription::none()
    }
}

pub fn sync_telemetry(state: &mut DashboardState, session: &Session) {
    state
        .pedal_chart
        .set_series_points(0, &session.points(|sample| sample.throttle * 100.0));
    state
        .pedal_chart
        .set_series_points(1, &session.points(|sample| sample.brake * 100.0));

    state
        .dynamics_chart
        .set_series_points(0, &session.points(|sample| sample.acceleration_g[0]));
    state
        .dynamics_chart
        .set_series_points(1, &session.points(|sample| sample.acceleration_g[1]));

    for wheel in 0..4 {
        state.tyre_chart.set_series_points(
            wheel,
            &session.points(|sample| sample.tyre_core_temperature_c[wheel]),
        );
    }

    state.rendered_packets = session.packets_received();
}

pub fn view<'a>(state: &'a DashboardState, session: &'a Session) -> Element<'a, DashboardMessage> {
    let latest = session.latest().copied().unwrap_or_default();
    let (connection_label, connection_color) = match session.connection() {
        ConnectionStatus::Disconnected => ("Disconnected", Color::from_rgb(0.52, 0.55, 0.60)),
        ConnectionStatus::Connecting => {
            ("Waiting for telemetry", Color::from_rgb(0.95, 0.62, 0.12))
        },
        ConnectionStatus::Connected => ("Live", Color::from_rgb(0.12, 0.72, 0.38)),
    };
    let connection_action = if session.wants_connection() {
        "Disconnect"
    } else {
        "Connect"
    };
    let runtime_description = session
        .last_error()
        .unwrap_or("Assetto Corsa real-time telemetry");

    let header = column![
        text("Dashboard").size(28),
        text(runtime_description).size(14),
    ]
    .spacing(4);

    let primary_metrics = row![
        metric_card("Connection", connection_label, Some(connection_color)),
        metric_card("Speed", format!("{:.0} km/h", latest.speed_kmh), None),
        metric_card("RPM", latest.rpm.max(0).to_string(), None),
        metric_card("Gear", format_gear(latest.gear), None),
    ]
    .spacing(12);

    let secondary_metrics = row![
        metric_card("Fuel", format!("{:.1} L", latest.fuel_litres), None),
        metric_card("Current lap", format_lap_time(latest.current_lap_ms), None),
        metric_card("Last lap", format_lap_time(latest.last_lap_ms), None),
        metric_card("Position", format_position(latest.position), None),
    ]
    .spacing(12);

    let actions = row![
        button(connection_action)
            .style(appearance::action_button)
            .on_press(DashboardMessage::ToggleConnection),
        text(format!(
            "{} · {} packets",
            session.server_addr(),
            session.packets_received()
        ))
        .size(12),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);

    let pedal_chart = fixed_height(chart_card(
        "Pedal input",
        format!(
            "Throttle and brake · rolling {}-second window",
            HISTORY_WINDOW.as_secs()
        ),
        state.pedal_chart.view().map(DashboardMessage::PedalPlot),
    ));
    let dynamics_chart = fixed_height(chart_card(
        "Vehicle dynamics",
        "Lateral and longitudinal acceleration",
        state
            .dynamics_chart
            .view()
            .map(DashboardMessage::DynamicsPlot),
    ));
    let tyre_chart = fixed_height(chart_card(
        "Tyre core temperature",
        "Front-left, front-right, rear-left and rear-right",
        state.tyre_chart.view().map(DashboardMessage::TyrePlot),
    ));

    scrollable(
        column![
            header,
            primary_metrics,
            secondary_metrics,
            actions,
            pedal_chart,
            dynamics_chart,
            tyre_chart,
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn fixed_height<'a>(content: Element<'a, DashboardMessage>) -> Element<'a, DashboardMessage> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(CHART_HEIGHT))
        .into()
}

fn build_pedal_chart() -> TimeSeriesChart {
    let throttle = LineSeries::new(
        placeholder(),
        "Throttle",
        Color::from_rgb(0.12, 0.72, 0.38),
        LineStyle::solid().with_pixel_width(2.5),
    );
    let brake = LineSeries::new(
        placeholder(),
        "Brake",
        Color::from_rgb(0.90, 0.24, 0.24),
        LineStyle::solid().with_pixel_width(2.5),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("Pedal input", 0.0, 100.0, |value| format!("{value:.0}%")),
            |label, x, y| format!("{label}\n{x:.1}s  {y:.0}%"),
        ),
        [throttle, brake],
    )
}

fn build_dynamics_chart() -> TimeSeriesChart {
    let lateral = LineSeries::new(
        placeholder(),
        "Lateral G",
        Color::from_rgb(0.23, 0.55, 0.95),
        LineStyle::solid().with_pixel_width(2.2),
    );
    let longitudinal = LineSeries::new(
        placeholder(),
        "Longitudinal G",
        Color::from_rgb(0.95, 0.55, 0.18),
        LineStyle::solid().with_pixel_width(2.2),
    );

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("Acceleration", -3.0, 3.0, |value| format!("{value:.1}G")),
            |label, x, y| format!("{label}\n{x:.1}s  {y:.2}G"),
        ),
        [lateral, longitudinal],
    )
}

fn build_tyre_chart() -> TimeSeriesChart {
    let series = [
        ("Front left", Color::from_rgb(0.18, 0.65, 0.95)),
        ("Front right", Color::from_rgb(0.35, 0.78, 0.65)),
        ("Rear left", Color::from_rgb(0.95, 0.62, 0.20)),
        ("Rear right", Color::from_rgb(0.90, 0.32, 0.38)),
    ]
    .map(|(label, color)| {
        LineSeries::new(
            placeholder(),
            label,
            color,
            LineStyle::solid().with_pixel_width(2.0),
        )
    });

    TimeSeriesChart::new(
        TimeSeriesSpec::new(
            time_axis(),
            AxisSpec::new("Temperature", 0.0, 150.0, |value| format!("{value:.0}°C")),
            |label, x, y| format!("{label}\n{x:.1}s  {y:.1}°C"),
        ),
        series,
    )
}

fn time_axis() -> AxisSpec {
    AxisSpec::new("Time", 0.0, HISTORY_WINDOW.as_secs_f64(), |value| {
        format!("{value:.0}s")
    })
}

fn placeholder() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0]]
}

fn format_gear(gear: i32) -> String {
    match gear {
        0 => "R".to_owned(),
        1 => "N".to_owned(),
        gear if gear > 1 => (gear - 1).to_string(),
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

fn format_lap_time(milliseconds: i32) -> String {
    if milliseconds <= 0 {
        return "--:--.---".to_owned();
    }

    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1_000;
    let millis = milliseconds % 1_000;
    format!("{minutes}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use chiaroscuro_telemetry::TelemetrySample;

    use super::{
        DashboardMessage, DashboardState, format_gear, format_lap_time, format_position, update,
    };
    use crate::session::Session;

    #[test]
    fn formats_assetto_corsa_gears() {
        assert_eq!(format_gear(0), "R");
        assert_eq!(format_gear(1), "N");
        assert_eq!(format_gear(6), "5");
    }

    #[test]
    fn formats_lap_times() {
        assert_eq!(format_lap_time(91_234), "1:31.234");
        assert_eq!(format_lap_time(0), "--:--.---");
    }

    #[test]
    fn formats_race_position() {
        assert_eq!(format_position(3), "P3");
        assert_eq!(format_position(0), "—");
    }

    #[test]
    fn refreshes_charts_only_after_new_packets() {
        let mut state = DashboardState::default();
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        let _action = update(&mut state, &session, DashboardMessage::Refresh);

        assert_eq!(state.rendered_packets, 1);
    }
}
