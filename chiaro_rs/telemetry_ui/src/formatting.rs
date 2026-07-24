//! Pure text formatting shared by Telemetry charts and panels.

use chiaro_telemetry::LAP_DISTANCE_AXIS_MAX;

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

pub(super) fn format_track_position(position: f32) -> String {
    if position < 0.0 {
        "—".to_owned()
    } else {
        format!("{:.1}%", position.clamp(0.0, 1.0) * 100.0)
    }
}

pub(super) fn format_lap_count(lap_count: usize) -> String {
    if lap_count == 1 {
        "1 lap".to_owned()
    } else {
        format!("{lap_count} laps")
    }
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

pub(super) fn format_lap_distance(chart_position: f64) -> String {
    let percentage = chart_position / LAP_DISTANCE_AXIS_MAX * 100.0;
    if percentage == 0.0 {
        "0%".to_owned()
    } else {
        format!("{percentage:.0}%")
    }
}
