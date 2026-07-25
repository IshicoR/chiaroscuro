//! Construction and axis specifications for Telemetry charts.

use chiaro_telemetry::{HISTORY_WINDOW, LAP_DISTANCE_AXIS_MAX};
use chiaro_time_series_chart::{AxisSpec, LineSeries, TimeSeriesChart, TimeSeriesSpec};
use iced::Color;
use iced_plot::{AxisLink, LineStyle};

use super::{
    BRAKE_LINE_COLOR, STATUS_WARNING, STEERING_LINE_COLOR, THROTTLE_LINE_COLOR,
    formatting::{format_chart_time, format_gear, format_lap_distance},
};

const PRIMARY_LINE_WIDTH: f32 = 1.8;
const DYNAMICS_LINE_WIDTH: f32 = 1.6;
const WHEEL_LINE_WIDTH: f32 = 1.4;
const BRAKE_PRESSURE_LINE_WIDTH: f32 = 1.2;
const REFERENCE_LINE_WIDTH: f32 = 1.1;

const REFERENCE_AMBER: Color = Color::from_rgb(0.96, 0.76, 0.28);
const REFERENCE_CYAN: Color = Color::from_rgb(0.30, 0.78, 0.88);
const REFERENCE_PINK: Color = Color::from_rgb(0.94, 0.42, 0.66);
const REFERENCE_LAVENDER: Color = Color::from_rgb(0.69, 0.56, 0.94);

pub(super) fn build_speed_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.18, 0.65, 0.95);
    let speed = LineSeries::new(
        placeholder(),
        "Speed",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_pedal_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let throttle = LineSeries::new(
        placeholder(),
        "Throttle",
        THROTTLE_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
    );
    let brake = LineSeries::new(
        placeholder(),
        "Brake",
        BRAKE_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_brake_pressure_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(BRAKE_PRESSURE_LINE_WIDTH));

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

pub(super) fn build_abs_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let abs = LineSeries::new(
        placeholder(),
        "ABS active",
        STATUS_WARNING,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_steering_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let steering = LineSeries::new(
        placeholder(),
        "Steering angle",
        STEERING_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_steering_torque_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let torque = LineSeries::new(
        placeholder(),
        "Steering torque",
        STEERING_LINE_COLOR,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_rpm_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.92, 0.46, 0.18);
    let rpm = LineSeries::new(
        placeholder(),
        "RPM",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_gear_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.55, 0.58, 0.65);
    let gear = LineSeries::new(
        placeholder(),
        "Gear",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_dynamics_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let lateral_color = Color::from_rgb(0.23, 0.55, 0.95);
    let longitudinal_color = Color::from_rgb(0.95, 0.55, 0.18);
    let lateral = LineSeries::new(
        placeholder(),
        "Lateral G",
        lateral_color,
        LineStyle::solid().with_pixel_width(DYNAMICS_LINE_WIDTH),
    );
    let longitudinal = LineSeries::new(
        placeholder(),
        "Longitudinal G",
        longitudinal_color,
        LineStyle::solid().with_pixel_width(DYNAMICS_LINE_WIDTH),
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

pub(super) fn build_yaw_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.72, 0.34, 0.95);
    let yaw = LineSeries::new(
        placeholder(),
        "Yaw rate",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_wheel_slip_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_LINE_WIDTH));
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

pub(super) fn build_tyre_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_LINE_WIDTH));
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

pub(super) fn build_suspension_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let series = wheel_series(LineStyle::solid().with_pixel_width(WHEEL_LINE_WIDTH));
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

pub(super) fn build_fuel_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let color = Color::from_rgb(0.84, 0.65, 0.16);
    let fuel = LineSeries::new(
        placeholder(),
        "Fuel used",
        color,
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn build_delta_chart(x_axis_link: AxisLink) -> TimeSeriesChart {
    let delta = LineSeries::new(
        placeholder(),
        "Delta",
        Color::from_rgb(0.72, 0.34, 0.95),
        LineStyle::solid().with_pixel_width(PRIMARY_LINE_WIDTH),
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

pub(super) fn time_axis() -> AxisSpec {
    AxisSpec::new("Time", 0.0, HISTORY_WINDOW.as_secs_f64(), format_chart_time)
}

pub(super) fn lap_distance_axis() -> AxisSpec {
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
        LineStyle::solid().with_pixel_width(REFERENCE_LINE_WIDTH),
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
