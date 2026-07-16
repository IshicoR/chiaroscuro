use bitflags::bitflags;

use crate::enums::{IncidentPenalty, IncidentReport};

bitflags! {
    /// Warning and limiter states reported by the player's engine.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct EngineWarnings: u32 {
        const WATER_TEMPERATURE_WARNING = 0x0001;
        const FUEL_PRESSURE_WARNING = 0x0002;
        const OIL_PRESSURE_WARNING = 0x0004;
        const ENGINE_STALLED = 0x0008;
        const PIT_SPEED_LIMITER = 0x0010;
        const REV_LIMITER_ACTIVE = 0x0020;
        const OIL_TEMPERATURE_WARNING = 0x0040;
        const MANDATORY_REPAIR_NEEDED = 0x0080;
        const OPTIONAL_REPAIR_NEEDED = 0x0100;
    }

    /// Race-control flags, driver penalties, and start-light states.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct SessionFlags: u32 {
        const CHECKERED = 0x0000_0001;
        const WHITE = 0x0000_0002;
        const GREEN = 0x0000_0004;
        const YELLOW = 0x0000_0008;
        const RED = 0x0000_0010;
        const BLUE = 0x0000_0020;
        const DEBRIS = 0x0000_0040;
        const CROSSED = 0x0000_0080;
        const YELLOW_WAVING = 0x0000_0100;
        const ONE_LAP_TO_GREEN = 0x0000_0200;
        const GREEN_HELD = 0x0000_0400;
        const TEN_TO_GO = 0x0000_0800;
        const FIVE_TO_GO = 0x0000_1000;
        const RANDOM_WAVING = 0x0000_2000;
        const CAUTION = 0x0000_4000;
        const CAUTION_WAVING = 0x0000_8000;
        const BLACK = 0x0001_0000;
        const DISQUALIFY = 0x0002_0000;
        const SERVICEABLE = 0x0004_0000;
        const FURLED = 0x0008_0000;
        const REPAIR = 0x0010_0000;
        const DISQUALIFIED_SCORING_INVALID = 0x0020_0000;
        const START_HIDDEN = 0x1000_0000;
        const START_READY = 0x2000_0000;
        const START_SET = 0x4000_0000;
        const START_GO = 0x8000_0000;
    }

    /// Current camera-system state.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct CameraState: u32 {
        const SESSION_SCREEN = 0x0001;
        const SCENIC_ACTIVE = 0x0002;
        const CAMERA_TOOL_ACTIVE = 0x0004;
        const UI_HIDDEN = 0x0008;
        const AUTO_SHOT_SELECTION = 0x0010;
        const TEMPORARY_EDITS = 0x0020;
        const KEY_ACCELERATION = 0x0040;
        const KEY_10X_ACCELERATION = 0x0080;
        const MOUSE_AIM_MODE = 0x0100;
    }

    /// Pit services selected for the next stop.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct PitServiceFlags: u32 {
        const LEFT_FRONT_TIRE_CHANGE = 0x0001;
        const RIGHT_FRONT_TIRE_CHANGE = 0x0002;
        const LEFT_REAR_TIRE_CHANGE = 0x0004;
        const RIGHT_REAR_TIRE_CHANGE = 0x0008;
        const FUEL_FILL = 0x0010;
        const WINDSHIELD_TEAROFF = 0x0020;
        const FAST_REPAIR = 0x0040;
    }

    /// Per-car instructions and privileges during pacing.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct PaceFlags: u32 {
        const END_OF_LINE = 0x0001;
        const FREE_PASS = 0x0002;
        const WAVED_AROUND = 0x0004;
    }
}

macro_rules! impl_flag_value {
    ($($type:ty),+ $(,)?) => {
        $(
            impl $type {
                /// Decodes known flags while retaining bits added by a newer SDK.
                pub const fn from_raw(raw: u32) -> Self {
                    Self::from_bits_retain(raw)
                }

                /// Returns every source bit, including bits unknown to this crate.
                pub const fn raw(self) -> u32 {
                    self.bits()
                }

                /// Returns only bits that are not defined by this SDK version.
                pub const fn unknown_bits(self) -> u32 {
                    self.bits() & !Self::all().bits()
                }
            }

            impl From<u32> for $type {
                fn from(raw: u32) -> Self {
                    Self::from_raw(raw)
                }
            }

            impl From<$type> for u32 {
                fn from(value: $type) -> Self {
                    value.raw()
                }
            }
        )+
    };
}

impl_flag_value!(
    EngineWarnings,
    SessionFlags,
    CameraState,
    PitServiceFlags,
    PaceFlags,
);

impl PaceFlags {
    /// Decodes the signed SDK primitive used by `CarIdxPaceFlags`.
    pub const fn from_int(raw: i32) -> Self {
        Self::from_raw(raw as u32)
    }

    /// Encodes these flags using the `CarIdxPaceFlags` SDK primitive.
    pub const fn int(self) -> i32 {
        self.raw() as i32
    }
}

impl From<i32> for PaceFlags {
    fn from(raw: i32) -> Self {
        Self::from_int(raw)
    }
}

impl From<PaceFlags> for i32 {
    fn from(value: PaceFlags) -> Self {
        value.int()
    }
}

/// A packed incident report copied without discarding unrecognized fields.
///
/// Unlike the other SDK flag values, each of the two low bytes represents one
/// mutually exclusive value rather than a set of independent bits.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IncidentFlags(u32);

impl IncidentFlags {
    pub const REPORT_MASK: u32 = 0x0000_00ff;
    pub const PENALTY_MASK: u32 = 0x0000_ff00;

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn report(self) -> IncidentReport {
        IncidentReport::from_raw((self.0 & Self::REPORT_MASK) as u8)
    }

    pub const fn penalty(self) -> IncidentPenalty {
        IncidentPenalty::from_raw(((self.0 & Self::PENALTY_MASK) >> 8) as u8)
    }

    pub const fn unknown_upper_bits(self) -> u32 {
        self.0 & !(Self::REPORT_MASK | Self::PENALTY_MASK)
    }
}

impl From<u32> for IncidentFlags {
    fn from(raw: u32) -> Self {
        Self::from_raw(raw)
    }
}

impl From<IncidentFlags> for u32 {
    fn from(value: IncidentFlags) -> Self {
        value.raw()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CameraState, EngineWarnings, IncidentFlags, PaceFlags, PitServiceFlags, SessionFlags,
    };
    use crate::enums::{IncidentPenalty, IncidentReport};

    #[test]
    fn engine_warning_bits_match_the_sdk_and_round_trip() {
        let known = [
            (EngineWarnings::WATER_TEMPERATURE_WARNING, 0x0001),
            (EngineWarnings::FUEL_PRESSURE_WARNING, 0x0002),
            (EngineWarnings::OIL_PRESSURE_WARNING, 0x0004),
            (EngineWarnings::ENGINE_STALLED, 0x0008),
            (EngineWarnings::PIT_SPEED_LIMITER, 0x0010),
            (EngineWarnings::REV_LIMITER_ACTIVE, 0x0020),
            (EngineWarnings::OIL_TEMPERATURE_WARNING, 0x0040),
            (EngineWarnings::MANDATORY_REPAIR_NEEDED, 0x0080),
            (EngineWarnings::OPTIONAL_REPAIR_NEEDED, 0x0100),
        ];

        for (flag, raw) in known {
            assert_eq!(flag.raw(), raw);
            assert_eq!(EngineWarnings::from_raw(raw), flag);
        }

        let raw = EngineWarnings::WATER_TEMPERATURE_WARNING.raw() | 0x8000_0000;
        let decoded = EngineWarnings::from_raw(raw);
        assert!(decoded.contains(EngineWarnings::WATER_TEMPERATURE_WARNING));
        assert_eq!(decoded.unknown_bits(), 0x8000_0000);
        assert_eq!(decoded.raw(), raw);
    }

    #[test]
    fn session_flag_bits_match_the_sdk_and_retain_the_high_bit() {
        let known = [
            (SessionFlags::CHECKERED, 0x0000_0001),
            (SessionFlags::WHITE, 0x0000_0002),
            (SessionFlags::GREEN, 0x0000_0004),
            (SessionFlags::YELLOW, 0x0000_0008),
            (SessionFlags::RED, 0x0000_0010),
            (SessionFlags::BLUE, 0x0000_0020),
            (SessionFlags::DEBRIS, 0x0000_0040),
            (SessionFlags::CROSSED, 0x0000_0080),
            (SessionFlags::YELLOW_WAVING, 0x0000_0100),
            (SessionFlags::ONE_LAP_TO_GREEN, 0x0000_0200),
            (SessionFlags::GREEN_HELD, 0x0000_0400),
            (SessionFlags::TEN_TO_GO, 0x0000_0800),
            (SessionFlags::FIVE_TO_GO, 0x0000_1000),
            (SessionFlags::RANDOM_WAVING, 0x0000_2000),
            (SessionFlags::CAUTION, 0x0000_4000),
            (SessionFlags::CAUTION_WAVING, 0x0000_8000),
            (SessionFlags::BLACK, 0x0001_0000),
            (SessionFlags::DISQUALIFY, 0x0002_0000),
            (SessionFlags::SERVICEABLE, 0x0004_0000),
            (SessionFlags::FURLED, 0x0008_0000),
            (SessionFlags::REPAIR, 0x0010_0000),
            (SessionFlags::DISQUALIFIED_SCORING_INVALID, 0x0020_0000),
            (SessionFlags::START_HIDDEN, 0x1000_0000),
            (SessionFlags::START_READY, 0x2000_0000),
            (SessionFlags::START_SET, 0x4000_0000),
            (SessionFlags::START_GO, 0x8000_0000),
        ];

        for (flag, raw) in known {
            assert_eq!(flag.raw(), raw);
            assert_eq!(SessionFlags::from_raw(raw), flag);
        }

        let decoded = SessionFlags::from_raw(0x8000_0004);
        assert!(decoded.contains(SessionFlags::START_GO | SessionFlags::GREEN));
        assert_eq!(decoded.raw(), 0x8000_0004);

        let with_unknown = SessionFlags::from_raw(0x0400_0004);
        assert!(with_unknown.contains(SessionFlags::GREEN));
        assert_eq!(with_unknown.unknown_bits(), 0x0400_0000);
        assert_eq!(with_unknown.raw(), 0x0400_0004);
    }

    #[test]
    fn camera_state_bits_match_the_sdk_and_retain_unknown_bits() {
        let known = [
            (CameraState::SESSION_SCREEN, 0x0001),
            (CameraState::SCENIC_ACTIVE, 0x0002),
            (CameraState::CAMERA_TOOL_ACTIVE, 0x0004),
            (CameraState::UI_HIDDEN, 0x0008),
            (CameraState::AUTO_SHOT_SELECTION, 0x0010),
            (CameraState::TEMPORARY_EDITS, 0x0020),
            (CameraState::KEY_ACCELERATION, 0x0040),
            (CameraState::KEY_10X_ACCELERATION, 0x0080),
            (CameraState::MOUSE_AIM_MODE, 0x0100),
        ];

        for (flag, raw) in known {
            assert_eq!(flag.raw(), raw);
            assert_eq!(CameraState::from_raw(raw), flag);
        }

        let raw = CameraState::UI_HIDDEN.raw() | 0x8000_0000;
        let decoded = CameraState::from_raw(raw);
        assert!(decoded.contains(CameraState::UI_HIDDEN));
        assert_eq!(decoded.unknown_bits(), 0x8000_0000);
        assert_eq!(decoded.raw(), raw);
    }

    #[test]
    fn pit_service_flag_bits_match_the_sdk_and_retain_unknown_bits() {
        let known = [
            (PitServiceFlags::LEFT_FRONT_TIRE_CHANGE, 0x0001),
            (PitServiceFlags::RIGHT_FRONT_TIRE_CHANGE, 0x0002),
            (PitServiceFlags::LEFT_REAR_TIRE_CHANGE, 0x0004),
            (PitServiceFlags::RIGHT_REAR_TIRE_CHANGE, 0x0008),
            (PitServiceFlags::FUEL_FILL, 0x0010),
            (PitServiceFlags::WINDSHIELD_TEAROFF, 0x0020),
            (PitServiceFlags::FAST_REPAIR, 0x0040),
        ];

        for (flag, raw) in known {
            assert_eq!(flag.raw(), raw);
            assert_eq!(PitServiceFlags::from_raw(raw), flag);
        }

        let raw = PitServiceFlags::FAST_REPAIR.raw() | 0x8000_0000;
        let decoded = PitServiceFlags::from_raw(raw);
        assert!(decoded.contains(PitServiceFlags::FAST_REPAIR));
        assert_eq!(decoded.unknown_bits(), 0x8000_0000);
        assert_eq!(decoded.raw(), raw);
    }

    #[test]
    fn pace_flag_bits_match_the_sdk_and_retain_unknown_bits() {
        let known = [
            (PaceFlags::END_OF_LINE, 0x0001),
            (PaceFlags::FREE_PASS, 0x0002),
            (PaceFlags::WAVED_AROUND, 0x0004),
        ];

        for (flag, raw) in known {
            assert_eq!(flag.raw(), raw);
            assert_eq!(PaceFlags::from_raw(raw), flag);
        }

        let raw = PaceFlags::FREE_PASS.raw() | 0x8000_0000;
        let decoded = PaceFlags::from_raw(raw);
        assert!(decoded.contains(PaceFlags::FREE_PASS));
        assert_eq!(decoded.unknown_bits(), 0x8000_0000);
        assert_eq!(decoded.raw(), raw);
        assert_eq!(PaceFlags::from_int(raw as i32), decoded);
        assert_eq!(decoded.int(), raw as i32);
    }

    #[test]
    fn incident_fields_decode_as_exclusive_values() {
        let incident = IncidentFlags::from_raw(0x0408);
        assert_eq!(incident.report(), IncidentReport::CollisionWithCar);
        assert_eq!(incident.penalty(), IncidentPenalty::FourX);
        assert_eq!(incident.raw(), 0x0408);
        assert_eq!(incident.unknown_upper_bits(), 0);

        let ongoing_off_track = IncidentFlags::from_raw(0x0003);
        assert_eq!(ongoing_off_track.report(), IncidentReport::OffTrackOngoing);
        assert_eq!(ongoing_off_track.penalty(), IncidentPenalty::NoReport);
    }

    #[test]
    fn incident_fields_and_upper_bits_retain_unrecognized_values() {
        let raw = 0xabcd_090f;
        let incident = IncidentFlags::from_raw(raw);

        assert_eq!(incident.report(), IncidentReport::Unrecognized(0x0f));
        assert_eq!(incident.penalty(), IncidentPenalty::Unrecognized(0x09));
        assert_eq!(incident.unknown_upper_bits(), 0xabcd_0000);
        assert_eq!(incident.raw(), raw);
        assert_eq!(u32::from(incident), raw);
    }
}
