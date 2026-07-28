//! Pure text formatting shared by Telemetry charts and panels.

pub(super) fn format_gear(gear: i32) -> String {
    match gear {
        -1 => "R".to_owned(),
        0 => "N".to_owned(),
        gear if gear > 0 => gear.to_string(),
        _ => "—".to_owned(),
    }
}

pub(super) fn format_position(position: i32) -> String {
    if position > 0 {
        format!("P{position}")
    } else {
        "—".to_owned()
    }
}

pub(super) fn format_track_position(position: f32, track_length_meters: f64) -> String {
    if position < 0.0 {
        "—".to_owned()
    } else {
        format_lap_distance(f64::from(position.clamp(0.0, 1.0)) * track_length_meters)
    }
}

pub(super) fn format_lap_count(lap_count: usize) -> String {
    count_laps(lap_count)
}

pub(super) fn format_recording_duration(seconds: f64) -> String {
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

pub(super) fn format_chart_time(seconds: f64) -> String {
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

pub(super) fn format_lap_distance(meters: f64) -> String {
    let meters = meters.max(0.0);
    if meters == 0.0 {
        "0 m".to_owned()
    } else {
        format!("{meters:.0} m")
    }
}

pub(super) fn parse_track_length_meters(value: &str) -> Option<f64> {
    let mut parts = value.split_whitespace();
    let length = parts.next()?.replace(',', ".").parse::<f64>().ok()?;
    let unit = parts.next()?.to_ascii_lowercase();
    let meters = match unit.as_str() {
        "km" => length * 1_000.0,
        "m" => length,
        _ => return None,
    };
    meters
        .is_finite()
        .then_some(meters)
        .filter(|meters| *meters > 0.0)
}
use chiaro_i18n::count_laps;
