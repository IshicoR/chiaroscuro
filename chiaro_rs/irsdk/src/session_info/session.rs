use serde::Deserialize;

use super::{SdkBool, SessionInfoExtra, SessionScalar, value::deserialize_vec_or_default};

/// All scheduled or completed phases within the server session.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SessionInfoSection {
    pub num_sessions: Option<i32>,
    pub current_session_num: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub sessions: Vec<Session>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One practice, qualifying, warmup, heat, or race phase.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Session {
    pub session_num: Option<i32>,
    pub session_laps: Option<SessionScalar>,
    pub session_time: Option<SessionScalar>,
    pub session_num_laps_to_avg: Option<i32>,
    pub session_type: Option<String>,
    pub session_track_rubber_state: Option<String>,
    pub session_name: Option<String>,
    pub session_sub_type: Option<String>,
    pub session_skipped: Option<SdkBool>,
    pub session_run_groups_used: Option<i32>,
    pub session_enforce_tire_compound_change: Option<SdkBool>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub qualify_positions: Vec<QualifyingResult>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub results_positions: Vec<SessionResult>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub results_fastest_lap: Vec<FastestLap>,
    pub results_average_lap_time: Option<f64>,
    pub results_num_caution_flags: Option<i32>,
    pub results_num_caution_laps: Option<i32>,
    pub results_num_lead_changes: Option<i32>,
    pub results_laps_complete: Option<i32>,
    pub results_official: Option<SdkBool>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One classified result entry for a session phase.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SessionResult {
    pub position: Option<i32>,
    pub class_position: Option<i32>,
    pub car_idx: Option<i32>,
    pub lap: Option<i32>,
    pub time: Option<f64>,
    pub fastest_lap: Option<i32>,
    pub fastest_time: Option<f64>,
    pub last_time: Option<f64>,
    pub laps_led: Option<i32>,
    pub laps_complete: Option<i32>,
    pub joker_laps_complete: Option<i32>,
    pub laps_driven: Option<f64>,
    pub incidents: Option<i32>,
    pub reason_out_id: Option<i32>,
    pub reason_out_str: Option<String>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One entry in a session's fastest-lap summary.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FastestLap {
    pub car_idx: Option<i32>,
    pub fastest_lap: Option<i32>,
    pub fastest_time: Option<f64>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// Standalone qualifying results, when iRacing publishes that section.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct QualifyingResultsInfo {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub results: Vec<QualifyingResult>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One qualifying or heat-grid position.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct QualifyingResult {
    pub position: Option<i32>,
    pub class_position: Option<i32>,
    pub car_idx: Option<i32>,
    pub fastest_lap: Option<i32>,
    pub fastest_time: Option<f64>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
