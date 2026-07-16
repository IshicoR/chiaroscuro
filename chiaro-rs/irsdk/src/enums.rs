/// A car's coarse location relative to the racing surface and pit road.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackLocation {
    NotInWorld,
    OffTrack,
    InPitStall,
    ApproachingPits,
    OnTrack,
    Unrecognized(i32),
}

impl TrackLocation {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            -1 => Self::NotInWorld,
            0 => Self::OffTrack,
            1 => Self::InPitStall,
            2 => Self::ApproachingPits,
            3 => Self::OnTrack,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::NotInWorld => -1,
            Self::OffTrack => 0,
            Self::InPitStall => 1,
            Self::ApproachingPits => 2,
            Self::OnTrack => 3,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The material directly below a car.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackSurfaceMaterial {
    NotInWorld,
    Undefined,
    Asphalt1,
    Asphalt2,
    Asphalt3,
    Asphalt4,
    Concrete1,
    Concrete2,
    RacingDirt1,
    RacingDirt2,
    Paint1,
    Paint2,
    Rumble1,
    Rumble2,
    Rumble3,
    Rumble4,
    Grass1,
    Grass2,
    Grass3,
    Grass4,
    Dirt1,
    Dirt2,
    Dirt3,
    Dirt4,
    Sand,
    Gravel1,
    Gravel2,
    Grasscrete,
    Astroturf,
    Unrecognized(i32),
}

impl TrackSurfaceMaterial {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            -1 => Self::NotInWorld,
            0 => Self::Undefined,
            1 => Self::Asphalt1,
            2 => Self::Asphalt2,
            3 => Self::Asphalt3,
            4 => Self::Asphalt4,
            5 => Self::Concrete1,
            6 => Self::Concrete2,
            7 => Self::RacingDirt1,
            8 => Self::RacingDirt2,
            9 => Self::Paint1,
            10 => Self::Paint2,
            11 => Self::Rumble1,
            12 => Self::Rumble2,
            13 => Self::Rumble3,
            14 => Self::Rumble4,
            15 => Self::Grass1,
            16 => Self::Grass2,
            17 => Self::Grass3,
            18 => Self::Grass4,
            19 => Self::Dirt1,
            20 => Self::Dirt2,
            21 => Self::Dirt3,
            22 => Self::Dirt4,
            23 => Self::Sand,
            24 => Self::Gravel1,
            25 => Self::Gravel2,
            26 => Self::Grasscrete,
            27 => Self::Astroturf,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::NotInWorld => -1,
            Self::Undefined => 0,
            Self::Asphalt1 => 1,
            Self::Asphalt2 => 2,
            Self::Asphalt3 => 3,
            Self::Asphalt4 => 4,
            Self::Concrete1 => 5,
            Self::Concrete2 => 6,
            Self::RacingDirt1 => 7,
            Self::RacingDirt2 => 8,
            Self::Paint1 => 9,
            Self::Paint2 => 10,
            Self::Rumble1 => 11,
            Self::Rumble2 => 12,
            Self::Rumble3 => 13,
            Self::Rumble4 => 14,
            Self::Grass1 => 15,
            Self::Grass2 => 16,
            Self::Grass3 => 17,
            Self::Grass4 => 18,
            Self::Dirt1 => 19,
            Self::Dirt2 => 20,
            Self::Dirt3 => 21,
            Self::Dirt4 => 22,
            Self::Sand => 23,
            Self::Gravel1 => 24,
            Self::Gravel2 => 25,
            Self::Grasscrete => 26,
            Self::Astroturf => 27,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The lifecycle state of the current session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Invalid,
    GetInCar,
    Warmup,
    ParadeLaps,
    Racing,
    Checkered,
    CoolDown,
    Unrecognized(i32),
}

impl SessionState {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::GetInCar,
            2 => Self::Warmup,
            3 => Self::ParadeLaps,
            4 => Self::Racing,
            5 => Self::Checkered,
            6 => Self::CoolDown,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::Invalid => 0,
            Self::GetInCar => 1,
            Self::Warmup => 2,
            Self::ParadeLaps => 3,
            Self::Racing => 4,
            Self::Checkered => 5,
            Self::CoolDown => 6,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The spotter's coarse left/right assessment around the player's car.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarLeftRight {
    Off,
    Clear,
    CarLeft,
    CarRight,
    CarsLeftAndRight,
    TwoCarsLeft,
    TwoCarsRight,
    Unrecognized(i32),
}

impl CarLeftRight {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Off,
            1 => Self::Clear,
            2 => Self::CarLeft,
            3 => Self::CarRight,
            4 => Self::CarsLeftAndRight,
            5 => Self::TwoCarsLeft,
            6 => Self::TwoCarsRight,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::Off => 0,
            Self::Clear => 1,
            Self::CarLeft => 2,
            Self::CarRight => 3,
            Self::CarsLeftAndRight => 4,
            Self::TwoCarsLeft => 5,
            Self::TwoCarsRight => 6,
            Self::Unrecognized(value) => value,
        }
    }

    /// Decodes the unsigned SDK primitive used by the `CarLeftRight` variable.
    pub const fn from_bit_field(raw: u32) -> Self {
        Self::from_raw(raw as i32)
    }

    /// Encodes this value using the `CarLeftRight` variable's SDK primitive.
    pub const fn bit_field(self) -> u32 {
        self.raw() as u32
    }
}

/// The player's current pit-service state or positioning error.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitServiceStatus {
    None,
    InProgress,
    Complete,
    TooFarLeft,
    TooFarRight,
    TooFarForward,
    TooFarBack,
    BadAngle,
    CannotFix,
    Unrecognized(i32),
}

impl PitServiceStatus {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::None,
            1 => Self::InProgress,
            2 => Self::Complete,
            100 => Self::TooFarLeft,
            101 => Self::TooFarRight,
            102 => Self::TooFarForward,
            103 => Self::TooFarBack,
            104 => Self::BadAngle,
            105 => Self::CannotFix,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::None => 0,
            Self::InProgress => 1,
            Self::Complete => 2,
            Self::TooFarLeft => 100,
            Self::TooFarRight => 101,
            Self::TooFarForward => 102,
            Self::TooFarBack => 103,
            Self::BadAngle => 104,
            Self::CannotFix => 105,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The formation mode used for a start or restart.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaceMode {
    SingleFileStart,
    DoubleFileStart,
    SingleFileRestart,
    DoubleFileRestart,
    NotPacing,
    Unrecognized(i32),
}

impl PaceMode {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::SingleFileStart,
            1 => Self::DoubleFileStart,
            2 => Self::SingleFileRestart,
            3 => Self::DoubleFileRestart,
            4 => Self::NotPacing,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::SingleFileStart => 0,
            Self::DoubleFileStart => 1,
            Self::SingleFileRestart => 2,
            Self::DoubleFileRestart => 3,
            Self::NotPacing => 4,
            Self::Unrecognized(value) => value,
        }
    }
}

/// iRacing's estimate of average track wetness.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackWetness {
    Unknown,
    Dry,
    MostlyDry,
    VeryLightlyWet,
    LightlyWet,
    ModeratelyWet,
    VeryWet,
    ExtremelyWet,
    Unrecognized(i32),
}

impl TrackWetness {
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Dry,
            2 => Self::MostlyDry,
            3 => Self::VeryLightlyWet,
            4 => Self::LightlyWet,
            5 => Self::ModeratelyWet,
            6 => Self::VeryWet,
            7 => Self::ExtremelyWet,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::Unknown => 0,
            Self::Dry => 1,
            Self::MostlyDry => 2,
            Self::VeryLightlyWet => 3,
            Self::LightlyWet => 4,
            Self::ModeratelyWet => 5,
            Self::VeryWet => 6,
            Self::ExtremelyWet => 7,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The reason encoded in the low byte of an incident value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncidentReport {
    NoReport,
    OutOfControl,
    OffTrack,
    OffTrackOngoing,
    ContactWithWorld,
    CollisionWithWorld,
    CollisionWithWorldOngoing,
    ContactWithCar,
    CollisionWithCar,
    Unrecognized(u8),
}

impl IncidentReport {
    pub const fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::NoReport,
            1 => Self::OutOfControl,
            2 => Self::OffTrack,
            3 => Self::OffTrackOngoing,
            4 => Self::ContactWithWorld,
            5 => Self::CollisionWithWorld,
            6 => Self::CollisionWithWorldOngoing,
            7 => Self::ContactWithCar,
            8 => Self::CollisionWithCar,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::NoReport => 0,
            Self::OutOfControl => 1,
            Self::OffTrack => 2,
            Self::OffTrackOngoing => 3,
            Self::ContactWithWorld => 4,
            Self::CollisionWithWorld => 5,
            Self::CollisionWithWorldOngoing => 6,
            Self::ContactWithCar => 7,
            Self::CollisionWithCar => 8,
            Self::Unrecognized(value) => value,
        }
    }
}

/// The incident-point penalty encoded in the second byte of an incident value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncidentPenalty {
    NoReport,
    ZeroX,
    OneX,
    TwoX,
    FourX,
    Unrecognized(u8),
}

impl IncidentPenalty {
    pub const fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::NoReport,
            1 => Self::ZeroX,
            2 => Self::OneX,
            3 => Self::TwoX,
            4 => Self::FourX,
            value => Self::Unrecognized(value),
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::NoReport => 0,
            Self::ZeroX => 1,
            Self::OneX => 2,
            Self::TwoX => 3,
            Self::FourX => 4,
            Self::Unrecognized(value) => value,
        }
    }
}

macro_rules! impl_i32_conversions {
    ($($type:ty),+ $(,)?) => {
        $(
            impl From<i32> for $type {
                fn from(raw: i32) -> Self {
                    Self::from_raw(raw)
                }
            }

            impl From<$type> for i32 {
                fn from(value: $type) -> Self {
                    value.raw()
                }
            }
        )+
    };
}

impl_i32_conversions!(
    TrackLocation,
    TrackSurfaceMaterial,
    SessionState,
    CarLeftRight,
    PitServiceStatus,
    PaceMode,
    TrackWetness,
);

impl From<u32> for CarLeftRight {
    fn from(raw: u32) -> Self {
        Self::from_bit_field(raw)
    }
}

impl From<CarLeftRight> for u32 {
    fn from(value: CarLeftRight) -> Self {
        value.bit_field()
    }
}

macro_rules! impl_u8_conversions {
    ($($type:ty),+ $(,)?) => {
        $(
            impl From<u8> for $type {
                fn from(raw: u8) -> Self {
                    Self::from_raw(raw)
                }
            }

            impl From<$type> for u8 {
                fn from(value: $type) -> Self {
                    value.raw()
                }
            }
        )+
    };
}

impl_u8_conversions!(IncidentReport, IncidentPenalty);

#[cfg(test)]
mod tests {
    use super::{
        CarLeftRight, IncidentPenalty, IncidentReport, PaceMode, PitServiceStatus, SessionState,
        TrackLocation, TrackSurfaceMaterial, TrackWetness,
    };

    #[test]
    fn track_locations_round_trip_known_and_unknown_values() {
        let known = [
            (-1, TrackLocation::NotInWorld),
            (0, TrackLocation::OffTrack),
            (1, TrackLocation::InPitStall),
            (2, TrackLocation::ApproachingPits),
            (3, TrackLocation::OnTrack),
        ];

        for (raw, value) in known {
            assert_eq!(TrackLocation::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(TrackLocation::from_raw(91), TrackLocation::Unrecognized(91));
        assert_eq!(TrackLocation::Unrecognized(-7).raw(), -7);
    }

    #[test]
    fn track_surface_materials_round_trip_every_sdk_value() {
        let known = [
            TrackSurfaceMaterial::NotInWorld,
            TrackSurfaceMaterial::Undefined,
            TrackSurfaceMaterial::Asphalt1,
            TrackSurfaceMaterial::Asphalt2,
            TrackSurfaceMaterial::Asphalt3,
            TrackSurfaceMaterial::Asphalt4,
            TrackSurfaceMaterial::Concrete1,
            TrackSurfaceMaterial::Concrete2,
            TrackSurfaceMaterial::RacingDirt1,
            TrackSurfaceMaterial::RacingDirt2,
            TrackSurfaceMaterial::Paint1,
            TrackSurfaceMaterial::Paint2,
            TrackSurfaceMaterial::Rumble1,
            TrackSurfaceMaterial::Rumble2,
            TrackSurfaceMaterial::Rumble3,
            TrackSurfaceMaterial::Rumble4,
            TrackSurfaceMaterial::Grass1,
            TrackSurfaceMaterial::Grass2,
            TrackSurfaceMaterial::Grass3,
            TrackSurfaceMaterial::Grass4,
            TrackSurfaceMaterial::Dirt1,
            TrackSurfaceMaterial::Dirt2,
            TrackSurfaceMaterial::Dirt3,
            TrackSurfaceMaterial::Dirt4,
            TrackSurfaceMaterial::Sand,
            TrackSurfaceMaterial::Gravel1,
            TrackSurfaceMaterial::Gravel2,
            TrackSurfaceMaterial::Grasscrete,
            TrackSurfaceMaterial::Astroturf,
        ];

        for (raw, value) in (-1..=27).zip(known) {
            assert_eq!(TrackSurfaceMaterial::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(
            TrackSurfaceMaterial::from_raw(28),
            TrackSurfaceMaterial::Unrecognized(28)
        );
        assert_eq!(TrackSurfaceMaterial::Unrecognized(-2).raw(), -2);
    }

    #[test]
    fn session_states_round_trip_known_and_unknown_values() {
        let known = [
            SessionState::Invalid,
            SessionState::GetInCar,
            SessionState::Warmup,
            SessionState::ParadeLaps,
            SessionState::Racing,
            SessionState::Checkered,
            SessionState::CoolDown,
        ];

        for (raw, value) in (0..=6).zip(known) {
            assert_eq!(SessionState::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(SessionState::from_raw(7), SessionState::Unrecognized(7));
        assert_eq!(SessionState::Unrecognized(-1).raw(), -1);
    }

    #[test]
    fn car_left_right_values_round_trip_known_and_unknown_values() {
        let known = [
            CarLeftRight::Off,
            CarLeftRight::Clear,
            CarLeftRight::CarLeft,
            CarLeftRight::CarRight,
            CarLeftRight::CarsLeftAndRight,
            CarLeftRight::TwoCarsLeft,
            CarLeftRight::TwoCarsRight,
        ];

        for (raw, value) in (0..=6).zip(known) {
            assert_eq!(CarLeftRight::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(CarLeftRight::from_raw(7), CarLeftRight::Unrecognized(7));
        assert_eq!(CarLeftRight::Unrecognized(-1).raw(), -1);
        assert_eq!(
            CarLeftRight::from_bit_field(4),
            CarLeftRight::CarsLeftAndRight
        );
        assert_eq!(CarLeftRight::Unrecognized(-1).bit_field(), u32::MAX);
    }

    #[test]
    fn pit_service_statuses_preserve_the_gap_and_unknown_values() {
        let known = [
            (0, PitServiceStatus::None),
            (1, PitServiceStatus::InProgress),
            (2, PitServiceStatus::Complete),
            (100, PitServiceStatus::TooFarLeft),
            (101, PitServiceStatus::TooFarRight),
            (102, PitServiceStatus::TooFarForward),
            (103, PitServiceStatus::TooFarBack),
            (104, PitServiceStatus::BadAngle),
            (105, PitServiceStatus::CannotFix),
        ];

        for (raw, value) in known {
            assert_eq!(PitServiceStatus::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(
            PitServiceStatus::from_raw(3),
            PitServiceStatus::Unrecognized(3)
        );
        assert_eq!(PitServiceStatus::Unrecognized(106).raw(), 106);
    }

    #[test]
    fn pace_modes_round_trip_known_and_unknown_values() {
        let known = [
            PaceMode::SingleFileStart,
            PaceMode::DoubleFileStart,
            PaceMode::SingleFileRestart,
            PaceMode::DoubleFileRestart,
            PaceMode::NotPacing,
        ];

        for (raw, value) in (0..=4).zip(known) {
            assert_eq!(PaceMode::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(PaceMode::from_raw(5), PaceMode::Unrecognized(5));
        assert_eq!(PaceMode::Unrecognized(-1).raw(), -1);
    }

    #[test]
    fn track_wetness_distinguishes_sdk_unknown_from_unrecognized_values() {
        let known = [
            TrackWetness::Unknown,
            TrackWetness::Dry,
            TrackWetness::MostlyDry,
            TrackWetness::VeryLightlyWet,
            TrackWetness::LightlyWet,
            TrackWetness::ModeratelyWet,
            TrackWetness::VeryWet,
            TrackWetness::ExtremelyWet,
        ];

        for (raw, value) in (0..=7).zip(known) {
            assert_eq!(TrackWetness::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(TrackWetness::from_raw(8), TrackWetness::Unrecognized(8));
        assert_eq!(TrackWetness::Unrecognized(-1).raw(), -1);
    }

    #[test]
    fn incident_reports_round_trip_every_sdk_value_and_unknown_values() {
        let known = [
            IncidentReport::NoReport,
            IncidentReport::OutOfControl,
            IncidentReport::OffTrack,
            IncidentReport::OffTrackOngoing,
            IncidentReport::ContactWithWorld,
            IncidentReport::CollisionWithWorld,
            IncidentReport::CollisionWithWorldOngoing,
            IncidentReport::ContactWithCar,
            IncidentReport::CollisionWithCar,
        ];

        for (raw, value) in (0_u8..=8).zip(known) {
            assert_eq!(IncidentReport::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(IncidentReport::from_raw(9), IncidentReport::Unrecognized(9));
        assert_eq!(IncidentReport::Unrecognized(u8::MAX).raw(), u8::MAX);
    }

    #[test]
    fn incident_penalties_round_trip_every_sdk_value_and_unknown_values() {
        let known = [
            IncidentPenalty::NoReport,
            IncidentPenalty::ZeroX,
            IncidentPenalty::OneX,
            IncidentPenalty::TwoX,
            IncidentPenalty::FourX,
        ];

        for (raw, value) in (0_u8..=4).zip(known) {
            assert_eq!(IncidentPenalty::from_raw(raw), value);
            assert_eq!(value.raw(), raw);
        }
        assert_eq!(
            IncidentPenalty::from_raw(5),
            IncidentPenalty::Unrecognized(5)
        );
        assert_eq!(IncidentPenalty::Unrecognized(u8::MAX).raw(), u8::MAX);
    }
}
