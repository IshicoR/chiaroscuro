use serde::Deserialize;

use super::{SdkBool, SessionInfoExtra, SessionScalar};

/// Track, event, build, and environment metadata for the current weekend.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct WeekendInfo {
    pub encoding: Option<String>,
    pub track_name: Option<String>,
    #[serde(rename = "TrackID")]
    pub track_id: Option<i32>,
    pub track_length: Option<String>,
    pub track_length_official: Option<String>,
    pub track_display_name: Option<String>,
    pub track_display_short_name: Option<String>,
    pub track_config_name: Option<String>,
    pub track_city: Option<String>,
    pub track_country: Option<String>,
    pub track_altitude: Option<String>,
    pub track_latitude: Option<String>,
    pub track_longitude: Option<String>,
    pub track_north_offset: Option<String>,
    pub track_num_turns: Option<i32>,
    pub track_pit_speed_limit: Option<String>,
    pub track_type: Option<String>,
    pub track_direction: Option<String>,
    pub track_weather_type: Option<String>,
    pub track_skies: Option<String>,
    pub track_surface_temp: Option<String>,
    pub track_air_temp: Option<String>,
    pub track_air_pressure: Option<String>,
    pub track_wind_vel: Option<String>,
    pub track_wind_dir: Option<String>,
    pub track_relative_humidity: Option<String>,
    pub track_fog_level: Option<String>,
    pub track_precipitation: Option<String>,
    pub track_cleanup: Option<SdkBool>,
    pub track_dynamic_track: Option<SdkBool>,
    pub track_version: Option<String>,
    #[serde(rename = "SeriesID")]
    pub series_id: Option<i64>,
    #[serde(rename = "SeasonID")]
    pub season_id: Option<i64>,
    #[serde(rename = "SessionID")]
    pub session_id: Option<i64>,
    #[serde(rename = "SubSessionID")]
    pub sub_session_id: Option<i64>,
    #[serde(rename = "LeagueID")]
    pub league_id: Option<i64>,
    pub official: Option<SdkBool>,
    pub race_week: Option<i32>,
    pub event_type: Option<String>,
    pub category: Option<String>,
    pub sim_mode: Option<String>,
    pub team_racing: Option<SdkBool>,
    pub min_drivers: Option<i32>,
    pub max_drivers: Option<i32>,
    #[serde(rename = "DCRuleSet")]
    pub driver_change_rule_set: Option<String>,
    pub qualifier_must_start_race: Option<SdkBool>,
    pub num_car_classes: Option<i32>,
    pub num_car_types: Option<i32>,
    pub heat_racing: Option<SdkBool>,
    pub build_type: Option<String>,
    pub build_target: Option<String>,
    pub build_version: Option<String>,
    pub weekend_options: Option<WeekendOptions>,
    pub telemetry_options: Option<TelemetryOptions>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// Rules, limits, and configured weather for the current weekend.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct WeekendOptions {
    pub num_starters: Option<i32>,
    pub starting_grid: Option<String>,
    pub qualify_scoring: Option<String>,
    pub course_cautions: Option<String>,
    pub standing_start: Option<SdkBool>,
    pub short_parade_lap: Option<SdkBool>,
    pub restarts: Option<String>,
    pub weather_type: Option<String>,
    pub skies: Option<String>,
    pub wind_direction: Option<String>,
    pub wind_speed: Option<String>,
    pub weather_temp: Option<String>,
    pub relative_humidity: Option<String>,
    pub fog_level: Option<String>,
    pub time_of_day: Option<String>,
    pub date: Option<String>,
    pub earth_rotation_speedup_factor: Option<i32>,
    pub unofficial: Option<SdkBool>,
    pub commercial_mode: Option<String>,
    pub night_mode: Option<SessionScalar>,
    pub is_fixed_setup: Option<SdkBool>,
    pub strict_laps_checking: Option<String>,
    pub has_open_registration: Option<SdkBool>,
    pub hardcore_level: Option<i32>,
    pub num_joker_laps: Option<i32>,
    pub incident_limit: Option<SessionScalar>,
    pub fast_repairs_limit: Option<SessionScalar>,
    pub green_white_checkered_limit: Option<SessionScalar>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// Disk telemetry configuration embedded in `WeekendInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct TelemetryOptions {
    pub telemetry_disk_file: Option<String>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
