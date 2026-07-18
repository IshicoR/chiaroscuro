//! Typed keys for documented iRacing live telemetry variables.
//!
//! Availability still depends on the loaded car and session. Call
//! `TelemetryFrame::get_optional` for variables that may not be published.

use crate::{ArrayKey, ScalarKey};

/// Session timing, state, and race-control variables.
pub mod session {
    use super::ScalarKey;

    pub const SESSION_TIME: ScalarKey<f64> = ScalarKey::new("SessionTime");
    pub const SESSION_TICK: ScalarKey<i32> = ScalarKey::new("SessionTick");
    pub const SESSION_NUM: ScalarKey<i32> = ScalarKey::new("SessionNum");
    pub const SESSION_STATE: ScalarKey<i32> = ScalarKey::new("SessionState");
    pub const SESSION_UNIQUE_ID: ScalarKey<i32> = ScalarKey::new("SessionUniqueID");
    pub const SESSION_FLAGS: ScalarKey<u32> = ScalarKey::new("SessionFlags");
    pub const SESSION_TIME_REMAIN: ScalarKey<f64> = ScalarKey::new("SessionTimeRemain");
    pub const SESSION_LAPS_REMAIN: ScalarKey<i32> = ScalarKey::new("SessionLapsRemain");
    pub const SESSION_LAPS_REMAIN_EX: ScalarKey<i32> = ScalarKey::new("SessionLapsRemainEx");
    pub const SESSION_TIME_OF_DAY: ScalarKey<f32> = ScalarKey::new("SessionTimeOfDay");
    pub const RACE_LAPS: ScalarKey<i32> = ScalarKey::new("RaceLaps");
    pub const PITS_OPEN: ScalarKey<bool> = ScalarKey::new("PitsOpen");
    pub const PACE_MODE: ScalarKey<i32> = ScalarKey::new("PaceMode");
}

/// Player-car controls, lap timing, scoring, and track position.
pub mod player {
    use super::ScalarKey;

    pub const PLAYER_CAR_IDX: ScalarKey<i32> = ScalarKey::new("PlayerCarIdx");
    pub const IS_ON_TRACK: ScalarKey<bool> = ScalarKey::new("IsOnTrack");
    pub const IS_ON_TRACK_CAR: ScalarKey<bool> = ScalarKey::new("IsOnTrackCar");
    pub const IS_IN_GARAGE: ScalarKey<bool> = ScalarKey::new("IsInGarage");
    pub const DRIVER_MARKER: ScalarKey<bool> = ScalarKey::new("DriverMarker");

    pub const SPEED: ScalarKey<f32> = ScalarKey::new("Speed");
    pub const RPM: ScalarKey<f32> = ScalarKey::new("RPM");
    pub const GEAR: ScalarKey<i32> = ScalarKey::new("Gear");
    pub const THROTTLE: ScalarKey<f32> = ScalarKey::new("Throttle");
    pub const BRAKE: ScalarKey<f32> = ScalarKey::new("Brake");
    pub const CLUTCH: ScalarKey<f32> = ScalarKey::new("Clutch");
    pub const STEERING_WHEEL_ANGLE: ScalarKey<f32> = ScalarKey::new("SteeringWheelAngle");
    pub const FUEL_LEVEL: ScalarKey<f32> = ScalarKey::new("FuelLevel");
    pub const FUEL_LEVEL_PCT: ScalarKey<f32> = ScalarKey::new("FuelLevelPct");

    pub const LAP: ScalarKey<i32> = ScalarKey::new("Lap");
    pub const LAP_COMPLETED: ScalarKey<i32> = ScalarKey::new("LapCompleted");
    pub const LAP_DIST: ScalarKey<f32> = ScalarKey::new("LapDist");
    pub const LAP_DIST_PCT: ScalarKey<f32> = ScalarKey::new("LapDistPct");
    pub const LAP_BEST_LAP: ScalarKey<i32> = ScalarKey::new("LapBestLap");
    pub const LAP_BEST_LAP_TIME: ScalarKey<f32> = ScalarKey::new("LapBestLapTime");
    pub const LAP_LAST_LAP_TIME: ScalarKey<f32> = ScalarKey::new("LapLastLapTime");
    pub const LAP_CURRENT_LAP_TIME: ScalarKey<f32> = ScalarKey::new("LapCurrentLapTime");

    pub const PLAYER_CAR_POSITION: ScalarKey<i32> = ScalarKey::new("PlayerCarPosition");
    pub const PLAYER_CAR_CLASS_POSITION: ScalarKey<i32> = ScalarKey::new("PlayerCarClassPosition");
    pub const ON_PIT_ROAD: ScalarKey<bool> = ScalarKey::new("OnPitRoad");
    pub const PLAYER_TRACK_SURFACE: ScalarKey<i32> = ScalarKey::new("PlayerTrackSurface");
    pub const PLAYER_TRACK_SURFACE_MATERIAL: ScalarKey<i32> =
        ScalarKey::new("PlayerTrackSurfaceMaterial");
    pub const CAR_LEFT_RIGHT: ScalarKey<u32> = ScalarKey::new("CarLeftRight");
    pub const PLAYER_INCIDENTS: ScalarKey<u32> = ScalarKey::new("PlayerIncidents");
}

/// Per-car arrays indexed by iRacing car index.
pub mod cars {
    use super::ArrayKey;

    pub const CAR_IDX_LAP: ArrayKey<i32> = ArrayKey::new("CarIdxLap");
    pub const CAR_IDX_LAP_COMPLETED: ArrayKey<i32> = ArrayKey::new("CarIdxLapCompleted");
    pub const CAR_IDX_LAP_DIST_PCT: ArrayKey<f32> = ArrayKey::new("CarIdxLapDistPct");
    pub const CAR_IDX_POSITION: ArrayKey<i32> = ArrayKey::new("CarIdxPosition");
    pub const CAR_IDX_CLASS_POSITION: ArrayKey<i32> = ArrayKey::new("CarIdxClassPosition");
    pub const CAR_IDX_TRACK_SURFACE: ArrayKey<i32> = ArrayKey::new("CarIdxTrackSurface");
    pub const CAR_IDX_TRACK_SURFACE_MATERIAL: ArrayKey<i32> =
        ArrayKey::new("CarIdxTrackSurfaceMaterial");
    pub const CAR_IDX_ON_PIT_ROAD: ArrayKey<bool> = ArrayKey::new("CarIdxOnPitRoad");

    pub const CAR_IDX_EST_TIME: ArrayKey<f32> = ArrayKey::new("CarIdxEstTime");
    pub const CAR_IDX_F2_TIME: ArrayKey<f32> = ArrayKey::new("CarIdxF2Time");
    pub const CAR_IDX_GEAR: ArrayKey<i32> = ArrayKey::new("CarIdxGear");
    pub const CAR_IDX_RPM: ArrayKey<f32> = ArrayKey::new("CarIdxRPM");
    pub const CAR_IDX_STEER: ArrayKey<f32> = ArrayKey::new("CarIdxSteer");
    pub const CAR_IDX_LAST_LAP_TIME: ArrayKey<f32> = ArrayKey::new("CarIdxLastLapTime");
    pub const CAR_IDX_BEST_LAP_TIME: ArrayKey<f32> = ArrayKey::new("CarIdxBestLapTime");
    pub const CAR_IDX_BEST_LAP_NUM: ArrayKey<i32> = ArrayKey::new("CarIdxBestLapNum");

    pub const CAR_IDX_SESSION_FLAGS: ArrayKey<u32> = ArrayKey::new("CarIdxSessionFlags");
    // Despite the flag semantics, the SDK metadata declares this as an int array.
    pub const CAR_IDX_PACE_FLAGS: ArrayKey<i32> = ArrayKey::new("CarIdxPaceFlags");
    pub const CAR_IDX_PACE_LINE: ArrayKey<i32> = ArrayKey::new("CarIdxPaceLine");
    pub const CAR_IDX_PACE_ROW: ArrayKey<i32> = ArrayKey::new("CarIdxPaceRow");
}

/// Ambient weather and track-condition variables.
pub mod weather {
    use super::ScalarKey;

    pub const AIR_DENSITY: ScalarKey<f32> = ScalarKey::new("AirDensity");
    pub const AIR_PRESSURE: ScalarKey<f32> = ScalarKey::new("AirPressure");
    pub const AIR_TEMP: ScalarKey<f32> = ScalarKey::new("AirTemp");
    pub const FOG_LEVEL: ScalarKey<f32> = ScalarKey::new("FogLevel");
    pub const RELATIVE_HUMIDITY: ScalarKey<f32> = ScalarKey::new("RelativeHumidity");
    pub const SKIES: ScalarKey<i32> = ScalarKey::new("Skies");
    /// Legacy live variable; use `WeekendInfo::track_weather_type` on current builds.
    pub const WEATHER_TYPE: ScalarKey<i32> = ScalarKey::new("WeatherType");
    pub const WEATHER_DECLARED_WET: ScalarKey<bool> = ScalarKey::new("WeatherDeclaredWet");
    pub const TRACK_TEMP: ScalarKey<f32> = ScalarKey::new("TrackTemp");
    pub const TRACK_TEMP_CREW: ScalarKey<f32> = ScalarKey::new("TrackTempCrew");
    pub const TRACK_WETNESS: ScalarKey<i32> = ScalarKey::new("TrackWetness");
    pub const WIND_DIR: ScalarKey<f32> = ScalarKey::new("WindDir");
    pub const WIND_VEL: ScalarKey<f32> = ScalarKey::new("WindVel");
}

/// Player-car motion, powertrain, wheel, tyre, and suspension variables.
pub mod chassis {
    use super::ScalarKey;

    pub const LAT_ACCEL: ScalarKey<f32> = ScalarKey::new("LatAccel");
    pub const LONG_ACCEL: ScalarKey<f32> = ScalarKey::new("LongAccel");
    pub const VERT_ACCEL: ScalarKey<f32> = ScalarKey::new("VertAccel");
    pub const VELOCITY_X: ScalarKey<f32> = ScalarKey::new("VelocityX");
    pub const VELOCITY_Y: ScalarKey<f32> = ScalarKey::new("VelocityY");
    pub const VELOCITY_Z: ScalarKey<f32> = ScalarKey::new("VelocityZ");
    pub const YAW: ScalarKey<f32> = ScalarKey::new("Yaw");
    pub const YAW_NORTH: ScalarKey<f32> = ScalarKey::new("YawNorth");
    pub const YAW_RATE: ScalarKey<f32> = ScalarKey::new("YawRate");
    pub const PITCH: ScalarKey<f32> = ScalarKey::new("Pitch");
    pub const PITCH_RATE: ScalarKey<f32> = ScalarKey::new("PitchRate");
    pub const ROLL: ScalarKey<f32> = ScalarKey::new("Roll");
    pub const ROLL_RATE: ScalarKey<f32> = ScalarKey::new("RollRate");

    pub const ENGINE_WARNINGS: ScalarKey<u32> = ScalarKey::new("EngineWarnings");
    pub const MANIFOLD_PRESS: ScalarKey<f32> = ScalarKey::new("ManifoldPress");
    pub const FUEL_PRESS: ScalarKey<f32> = ScalarKey::new("FuelPress");
    pub const FUEL_USE_PER_HOUR: ScalarKey<f32> = ScalarKey::new("FuelUsePerHour");
    pub const OIL_LEVEL: ScalarKey<f32> = ScalarKey::new("OilLevel");
    pub const OIL_PRESS: ScalarKey<f32> = ScalarKey::new("OilPress");
    pub const OIL_TEMP: ScalarKey<f32> = ScalarKey::new("OilTemp");
    pub const WATER_LEVEL: ScalarKey<f32> = ScalarKey::new("WaterLevel");
    pub const WATER_TEMP: ScalarKey<f32> = ScalarKey::new("WaterTemp");
    pub const VOLTAGE: ScalarKey<f32> = ScalarKey::new("Voltage");

    pub const STEERING_WHEEL_ANGLE_MAX: ScalarKey<f32> = ScalarKey::new("SteeringWheelAngleMax");
    pub const STEERING_WHEEL_TORQUE: ScalarKey<f32> = ScalarKey::new("SteeringWheelTorque");
    pub const STEERING_WHEEL_PCT_TORQUE: ScalarKey<f32> = ScalarKey::new("SteeringWheelPctTorque");
    pub const STEERING_WHEEL_PEAK_FORCE_NM: ScalarKey<f32> =
        ScalarKey::new("SteeringWheelPeakForceNm");

    pub const LF_SPEED: ScalarKey<f32> = ScalarKey::new("LFspeed");
    pub const RF_SPEED: ScalarKey<f32> = ScalarKey::new("RFspeed");
    pub const LR_SPEED: ScalarKey<f32> = ScalarKey::new("LRspeed");
    pub const RR_SPEED: ScalarKey<f32> = ScalarKey::new("RRspeed");
    pub const LF_TEMP_CM: ScalarKey<f32> = ScalarKey::new("LFtempCM");
    pub const RF_TEMP_CM: ScalarKey<f32> = ScalarKey::new("RFtempCM");
    pub const LR_TEMP_CM: ScalarKey<f32> = ScalarKey::new("LRtempCM");
    pub const RR_TEMP_CM: ScalarKey<f32> = ScalarKey::new("RRtempCM");
    pub const LF_SHOCK_DEFL: ScalarKey<f32> = ScalarKey::new("LFshockDefl");
    pub const RF_SHOCK_DEFL: ScalarKey<f32> = ScalarKey::new("RFshockDefl");
    pub const LR_SHOCK_DEFL: ScalarKey<f32> = ScalarKey::new("LRshockDefl");
    pub const RR_SHOCK_DEFL: ScalarKey<f32> = ScalarKey::new("RRshockDefl");
    pub const LF_SHOCK_VEL: ScalarKey<f32> = ScalarKey::new("LFshockVel");
    pub const RF_SHOCK_VEL: ScalarKey<f32> = ScalarKey::new("RFshockVel");
    pub const LR_SHOCK_VEL: ScalarKey<f32> = ScalarKey::new("LRshockVel");
    pub const RR_SHOCK_VEL: ScalarKey<f32> = ScalarKey::new("RRshockVel");
}

/// Player pit-service request and progress variables.
pub mod pit {
    use super::ScalarKey;

    pub const PLAYER_CAR_PIT_SV_STATUS: ScalarKey<i32> = ScalarKey::new("PlayerCarPitSvStatus");
    pub const PIT_SV_FLAGS: ScalarKey<u32> = ScalarKey::new("PitSvFlags");
    pub const PIT_SV_FUEL: ScalarKey<f32> = ScalarKey::new("PitSvFuel");
    pub const PIT_SV_LFP: ScalarKey<f32> = ScalarKey::new("PitSvLFP");
    pub const PIT_SV_RFP: ScalarKey<f32> = ScalarKey::new("PitSvRFP");
    pub const PIT_SV_LRP: ScalarKey<f32> = ScalarKey::new("PitSvLRP");
    pub const PIT_SV_RRP: ScalarKey<f32> = ScalarKey::new("PitSvRRP");
    pub const PIT_REPAIR_LEFT: ScalarKey<f32> = ScalarKey::new("PitRepairLeft");
    pub const PIT_OPT_REPAIR_LEFT: ScalarKey<f32> = ScalarKey::new("PitOptRepairLeft");
}

/// Camera selection and replay-state variables.
pub mod camera {
    use super::ScalarKey;

    pub const CAM_CAR_IDX: ScalarKey<i32> = ScalarKey::new("CamCarIdx");
    pub const CAM_CAMERA_NUMBER: ScalarKey<i32> = ScalarKey::new("CamCameraNumber");
    pub const CAM_GROUP_NUMBER: ScalarKey<i32> = ScalarKey::new("CamGroupNumber");
    pub const CAM_CAMERA_STATE: ScalarKey<u32> = ScalarKey::new("CamCameraState");
    pub const IS_REPLAY_PLAYING: ScalarKey<bool> = ScalarKey::new("IsReplayPlaying");
    pub const REPLAY_FRAME_NUM: ScalarKey<i32> = ScalarKey::new("ReplayFrameNum");
    pub const REPLAY_FRAME_NUM_END: ScalarKey<i32> = ScalarKey::new("ReplayFrameNumEnd");
    pub const REPLAY_PLAY_SLOW_MOTION: ScalarKey<bool> = ScalarKey::new("ReplayPlaySlowMotion");
    pub const REPLAY_PLAY_SPEED: ScalarKey<i32> = ScalarKey::new("ReplayPlaySpeed");
    pub const REPLAY_SESSION_NUM: ScalarKey<i32> = ScalarKey::new("ReplaySessionNum");
    pub const REPLAY_SESSION_TIME: ScalarKey<f64> = ScalarKey::new("ReplaySessionTime");
}

/// Live voice-radio transmitter identifiers.
pub mod radio {
    use super::ScalarKey;

    pub const RADIO_TRANSMIT_CAR_IDX: ScalarKey<i32> = ScalarKey::new("RadioTransmitCarIdx");
    pub const RADIO_TRANSMIT_RADIO_IDX: ScalarKey<i32> = ScalarKey::new("RadioTransmitRadioIdx");
    pub const RADIO_TRANSMIT_FREQUENCY_IDX: ScalarKey<i32> =
        ScalarKey::new("RadioTransmitFrequencyIdx");
}

#[cfg(test)]
mod tests {
    use super::{camera, cars, chassis, pit, player, radio, session, weather};
    use crate::{TelemetryKey, VariableShape, VariableType};

    macro_rules! assert_key_names {
        ($($key:path => $name:literal),+ $(,)?) => {
            $(assert_eq!($key.name(), $name);)+
        };
    }

    #[test]
    fn session_and_player_keys_use_sdk_names() {
        assert_key_names!(
            session::SESSION_TIME => "SessionTime",
            session::SESSION_TICK => "SessionTick",
            session::SESSION_NUM => "SessionNum",
            session::SESSION_STATE => "SessionState",
            session::SESSION_UNIQUE_ID => "SessionUniqueID",
            session::SESSION_FLAGS => "SessionFlags",
            session::SESSION_TIME_REMAIN => "SessionTimeRemain",
            session::SESSION_LAPS_REMAIN => "SessionLapsRemain",
            session::SESSION_LAPS_REMAIN_EX => "SessionLapsRemainEx",
            session::SESSION_TIME_OF_DAY => "SessionTimeOfDay",
            session::RACE_LAPS => "RaceLaps",
            session::PITS_OPEN => "PitsOpen",
            session::PACE_MODE => "PaceMode",
            player::PLAYER_CAR_IDX => "PlayerCarIdx",
            player::IS_ON_TRACK => "IsOnTrack",
            player::IS_ON_TRACK_CAR => "IsOnTrackCar",
            player::IS_IN_GARAGE => "IsInGarage",
            player::DRIVER_MARKER => "DriverMarker",
            player::SPEED => "Speed",
            player::RPM => "RPM",
            player::GEAR => "Gear",
            player::THROTTLE => "Throttle",
            player::BRAKE => "Brake",
            player::CLUTCH => "Clutch",
            player::STEERING_WHEEL_ANGLE => "SteeringWheelAngle",
            player::FUEL_LEVEL => "FuelLevel",
            player::FUEL_LEVEL_PCT => "FuelLevelPct",
            player::LAP => "Lap",
            player::LAP_COMPLETED => "LapCompleted",
            player::LAP_DIST => "LapDist",
            player::LAP_DIST_PCT => "LapDistPct",
            player::LAP_BEST_LAP => "LapBestLap",
            player::LAP_BEST_LAP_TIME => "LapBestLapTime",
            player::LAP_LAST_LAP_TIME => "LapLastLapTime",
            player::LAP_CURRENT_LAP_TIME => "LapCurrentLapTime",
            player::PLAYER_CAR_POSITION => "PlayerCarPosition",
            player::PLAYER_CAR_CLASS_POSITION => "PlayerCarClassPosition",
            player::ON_PIT_ROAD => "OnPitRoad",
            player::PLAYER_TRACK_SURFACE => "PlayerTrackSurface",
            player::PLAYER_TRACK_SURFACE_MATERIAL => "PlayerTrackSurfaceMaterial",
            player::CAR_LEFT_RIGHT => "CarLeftRight",
            player::PLAYER_INCIDENTS => "PlayerIncidents",
        );
    }

    #[test]
    fn car_array_keys_use_sdk_names() {
        assert_key_names!(
            cars::CAR_IDX_LAP => "CarIdxLap",
            cars::CAR_IDX_LAP_COMPLETED => "CarIdxLapCompleted",
            cars::CAR_IDX_LAP_DIST_PCT => "CarIdxLapDistPct",
            cars::CAR_IDX_POSITION => "CarIdxPosition",
            cars::CAR_IDX_CLASS_POSITION => "CarIdxClassPosition",
            cars::CAR_IDX_TRACK_SURFACE => "CarIdxTrackSurface",
            cars::CAR_IDX_TRACK_SURFACE_MATERIAL => "CarIdxTrackSurfaceMaterial",
            cars::CAR_IDX_ON_PIT_ROAD => "CarIdxOnPitRoad",
            cars::CAR_IDX_EST_TIME => "CarIdxEstTime",
            cars::CAR_IDX_F2_TIME => "CarIdxF2Time",
            cars::CAR_IDX_GEAR => "CarIdxGear",
            cars::CAR_IDX_RPM => "CarIdxRPM",
            cars::CAR_IDX_STEER => "CarIdxSteer",
            cars::CAR_IDX_LAST_LAP_TIME => "CarIdxLastLapTime",
            cars::CAR_IDX_BEST_LAP_TIME => "CarIdxBestLapTime",
            cars::CAR_IDX_BEST_LAP_NUM => "CarIdxBestLapNum",
            cars::CAR_IDX_SESSION_FLAGS => "CarIdxSessionFlags",
            cars::CAR_IDX_PACE_FLAGS => "CarIdxPaceFlags",
            cars::CAR_IDX_PACE_LINE => "CarIdxPaceLine",
            cars::CAR_IDX_PACE_ROW => "CarIdxPaceRow",
        );
    }

    #[test]
    fn weather_and_chassis_keys_use_sdk_names() {
        assert_key_names!(
            weather::AIR_DENSITY => "AirDensity",
            weather::AIR_PRESSURE => "AirPressure",
            weather::AIR_TEMP => "AirTemp",
            weather::FOG_LEVEL => "FogLevel",
            weather::RELATIVE_HUMIDITY => "RelativeHumidity",
            weather::SKIES => "Skies",
            weather::WEATHER_TYPE => "WeatherType",
            weather::WEATHER_DECLARED_WET => "WeatherDeclaredWet",
            weather::TRACK_TEMP => "TrackTemp",
            weather::TRACK_TEMP_CREW => "TrackTempCrew",
            weather::TRACK_WETNESS => "TrackWetness",
            weather::WIND_DIR => "WindDir",
            weather::WIND_VEL => "WindVel",
            chassis::LAT_ACCEL => "LatAccel",
            chassis::LONG_ACCEL => "LongAccel",
            chassis::VERT_ACCEL => "VertAccel",
            chassis::VELOCITY_X => "VelocityX",
            chassis::VELOCITY_Y => "VelocityY",
            chassis::VELOCITY_Z => "VelocityZ",
            chassis::YAW => "Yaw",
            chassis::YAW_NORTH => "YawNorth",
            chassis::YAW_RATE => "YawRate",
            chassis::PITCH => "Pitch",
            chassis::PITCH_RATE => "PitchRate",
            chassis::ROLL => "Roll",
            chassis::ROLL_RATE => "RollRate",
            chassis::ENGINE_WARNINGS => "EngineWarnings",
            chassis::MANIFOLD_PRESS => "ManifoldPress",
            chassis::FUEL_PRESS => "FuelPress",
            chassis::FUEL_USE_PER_HOUR => "FuelUsePerHour",
            chassis::OIL_LEVEL => "OilLevel",
            chassis::OIL_PRESS => "OilPress",
            chassis::OIL_TEMP => "OilTemp",
            chassis::WATER_LEVEL => "WaterLevel",
            chassis::WATER_TEMP => "WaterTemp",
            chassis::VOLTAGE => "Voltage",
            chassis::STEERING_WHEEL_ANGLE_MAX => "SteeringWheelAngleMax",
            chassis::STEERING_WHEEL_TORQUE => "SteeringWheelTorque",
            chassis::STEERING_WHEEL_PCT_TORQUE => "SteeringWheelPctTorque",
            chassis::STEERING_WHEEL_PEAK_FORCE_NM => "SteeringWheelPeakForceNm",
            chassis::LF_SPEED => "LFspeed",
            chassis::RF_SPEED => "RFspeed",
            chassis::LR_SPEED => "LRspeed",
            chassis::RR_SPEED => "RRspeed",
            chassis::LF_TEMP_CM => "LFtempCM",
            chassis::RF_TEMP_CM => "RFtempCM",
            chassis::LR_TEMP_CM => "LRtempCM",
            chassis::RR_TEMP_CM => "RRtempCM",
            chassis::LF_SHOCK_DEFL => "LFshockDefl",
            chassis::RF_SHOCK_DEFL => "RFshockDefl",
            chassis::LR_SHOCK_DEFL => "LRshockDefl",
            chassis::RR_SHOCK_DEFL => "RRshockDefl",
            chassis::LF_SHOCK_VEL => "LFshockVel",
            chassis::RF_SHOCK_VEL => "RFshockVel",
            chassis::LR_SHOCK_VEL => "LRshockVel",
            chassis::RR_SHOCK_VEL => "RRshockVel",
        );
    }

    #[test]
    fn pit_camera_and_radio_keys_use_sdk_names() {
        assert_key_names!(
            pit::PLAYER_CAR_PIT_SV_STATUS => "PlayerCarPitSvStatus",
            pit::PIT_SV_FLAGS => "PitSvFlags",
            pit::PIT_SV_FUEL => "PitSvFuel",
            pit::PIT_SV_LFP => "PitSvLFP",
            pit::PIT_SV_RFP => "PitSvRFP",
            pit::PIT_SV_LRP => "PitSvLRP",
            pit::PIT_SV_RRP => "PitSvRRP",
            pit::PIT_REPAIR_LEFT => "PitRepairLeft",
            pit::PIT_OPT_REPAIR_LEFT => "PitOptRepairLeft",
            camera::CAM_CAR_IDX => "CamCarIdx",
            camera::CAM_CAMERA_NUMBER => "CamCameraNumber",
            camera::CAM_GROUP_NUMBER => "CamGroupNumber",
            camera::CAM_CAMERA_STATE => "CamCameraState",
            camera::IS_REPLAY_PLAYING => "IsReplayPlaying",
            camera::REPLAY_FRAME_NUM => "ReplayFrameNum",
            camera::REPLAY_FRAME_NUM_END => "ReplayFrameNumEnd",
            camera::REPLAY_PLAY_SLOW_MOTION => "ReplayPlaySlowMotion",
            camera::REPLAY_PLAY_SPEED => "ReplayPlaySpeed",
            camera::REPLAY_SESSION_NUM => "ReplaySessionNum",
            camera::REPLAY_SESSION_TIME => "ReplaySessionTime",
            radio::RADIO_TRANSMIT_CAR_IDX => "RadioTransmitCarIdx",
            radio::RADIO_TRANSMIT_RADIO_IDX => "RadioTransmitRadioIdx",
            radio::RADIO_TRANSMIT_FREQUENCY_IDX => "RadioTransmitFrequencyIdx",
        );
    }

    #[test]
    fn nonobvious_enum_and_flag_keys_match_runtime_metadata() {
        assert_eq!(player::CAR_LEFT_RIGHT.value_type(), VariableType::BitField);
        assert_eq!(player::CAR_LEFT_RIGHT.shape(), VariableShape::Scalar);
        assert_eq!(cars::CAR_IDX_PACE_FLAGS.value_type(), VariableType::Int);
        assert_eq!(cars::CAR_IDX_PACE_FLAGS.shape(), VariableShape::Array);
    }
}
