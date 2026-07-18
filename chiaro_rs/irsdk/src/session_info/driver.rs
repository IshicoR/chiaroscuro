use serde::Deserialize;

use super::{SdkBool, SessionInfoExtra, SessionScalar, value::deserialize_vec_or_default};

/// Player-car constants and all entries registered in the session.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct DriverInfo {
    pub driver_car_idx: Option<i32>,
    #[serde(rename = "DriverUserID")]
    pub driver_user_id: Option<i64>,
    pub pace_car_idx: Option<i32>,
    pub driver_head_pos_x: Option<f64>,
    pub driver_head_pos_y: Option<f64>,
    pub driver_head_pos_z: Option<f64>,
    pub driver_car_is_electric: Option<SdkBool>,
    #[serde(rename = "DriverCarIdleRPM")]
    pub driver_car_idle_rpm: Option<f64>,
    pub driver_car_red_line: Option<f64>,
    pub driver_car_eng_cylinder_count: Option<i32>,
    pub driver_car_fuel_kg_per_ltr: Option<f64>,
    pub driver_car_fuel_max_ltr: Option<f64>,
    pub driver_car_max_fuel_pct: Option<f64>,
    pub driver_car_gear_num_forward: Option<i32>,
    pub driver_car_gear_neutral: Option<i32>,
    pub driver_car_gear_reverse: Option<i32>,
    #[serde(rename = "DriverCarSLFirstRPM")]
    pub driver_car_shift_light_first_rpm: Option<f64>,
    #[serde(rename = "DriverCarSLShiftRPM")]
    pub driver_car_shift_light_shift_rpm: Option<f64>,
    #[serde(rename = "DriverCarSLLastRPM")]
    pub driver_car_shift_light_last_rpm: Option<f64>,
    #[serde(rename = "DriverCarSLBlinkRPM")]
    pub driver_car_shift_light_blink_rpm: Option<f64>,
    pub driver_car_version: Option<String>,
    pub driver_pit_trk_pct: Option<f64>,
    pub driver_car_est_lap_time: Option<f64>,
    pub driver_setup_name: Option<String>,
    pub driver_setup_is_modified: Option<SdkBool>,
    pub driver_setup_load_type_name: Option<String>,
    pub driver_setup_passed_tech: Option<SdkBool>,
    pub driver_incident_count: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub driver_tires: Vec<DriverTire>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub drivers: Vec<Driver>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// A tire index-to-compound mapping published for the player car.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct DriverTire {
    pub tire_index: Option<i32>,
    pub tire_compound_type: Option<String>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One car/team entry from `DriverInfo:Drivers`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Driver {
    pub car_idx: Option<i32>,
    pub user_name: Option<String>,
    pub abbrev_name: Option<String>,
    pub initials: Option<String>,
    #[serde(rename = "UserID")]
    pub user_id: Option<i64>,
    #[serde(rename = "TeamID")]
    pub team_id: Option<i64>,
    pub team_name: Option<String>,
    pub car_number: Option<String>,
    pub car_number_raw: Option<i32>,
    pub car_path: Option<String>,
    #[serde(rename = "CarClassID")]
    pub car_class_id: Option<i32>,
    #[serde(rename = "CarID")]
    pub car_id: Option<i32>,
    pub car_is_pace_car: Option<SdkBool>,
    #[serde(rename = "CarIsAI")]
    pub car_is_ai: Option<SdkBool>,
    pub car_is_electric: Option<SdkBool>,
    pub car_screen_name: Option<String>,
    pub car_screen_name_short: Option<String>,
    pub car_class_short_name: Option<String>,
    pub car_class_rel_speed: Option<i32>,
    pub car_class_license_level: Option<i32>,
    pub car_class_max_fuel_pct: Option<String>,
    pub car_class_weight_penalty: Option<String>,
    pub car_class_power_adjust: Option<String>,
    pub car_class_dry_tire_set_limit: Option<String>,
    pub car_class_color: Option<SessionScalar>,
    pub car_class_est_lap_time: Option<f64>,
    #[serde(rename = "IRating")]
    pub i_rating: Option<i32>,
    pub lic_level: Option<i32>,
    pub lic_sub_level: Option<i32>,
    pub lic_string: Option<String>,
    #[serde(rename = "LicColor")]
    pub license_color: Option<SessionScalar>,
    pub is_spectator: Option<SdkBool>,
    pub car_design_str: Option<String>,
    pub helmet_design_str: Option<String>,
    pub suit_design_str: Option<String>,
    pub body_type: Option<i32>,
    pub face_type: Option<i32>,
    pub helmet_type: Option<i32>,
    pub car_number_design_str: Option<String>,
    #[serde(rename = "CarSponsor_1")]
    pub car_sponsor_1: Option<i32>,
    #[serde(rename = "CarSponsor_2")]
    pub car_sponsor_2: Option<i32>,
    #[serde(rename = "ClubID")]
    pub club_id: Option<i32>,
    pub club_name: Option<String>,
    #[serde(rename = "DivisionID")]
    pub division_id: Option<i32>,
    pub division_name: Option<String>,
    pub cur_driver_incident_count: Option<i32>,
    pub team_incident_count: Option<i32>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
