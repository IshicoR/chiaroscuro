//! Stable identifiers and persisted layout values for the Telemetry screen.

use iced_fonts::lucide;

use super::CARD_TITLE_ICON_SIZE;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChartColumns {
    #[default]
    One,
    Two,
}

impl ChartColumns {
    pub(super) const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }

    pub(super) const fn persisted_value(self) -> u8 {
        self.count() as u8
    }

    pub(super) const fn from_persisted_value(value: u8) -> Self {
        match value {
            2 => Self::Two,
            _ => Self::One,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartId {
    Speed,
    Pedal,
    BrakePressure,
    Abs,
    Steering,
    SteeringTorque,
    Rpm,
    Gear,
    Dynamics,
    Yaw,
    WheelSlip,
    Tyre,
    Suspension,
    Fuel,
    Delta,
}

impl ChartId {
    pub(super) const ALL: [Self; 15] = [
        Self::Speed,
        Self::Pedal,
        Self::BrakePressure,
        Self::Abs,
        Self::Steering,
        Self::SteeringTorque,
        Self::Rpm,
        Self::Gear,
        Self::Dynamics,
        Self::Yaw,
        Self::WheelSlip,
        Self::Tyre,
        Self::Suspension,
        Self::Fuel,
        Self::Delta,
    ];
    pub(super) const COUNT: usize = Self::ALL.len();
    pub(super) const DEFAULT_ORDER: [Self; Self::COUNT] = [
        Self::Speed,
        Self::Delta,
        Self::Pedal,
        Self::BrakePressure,
        Self::Abs,
        Self::Steering,
        Self::SteeringTorque,
        Self::Gear,
        Self::Rpm,
        Self::Dynamics,
        Self::Yaw,
        Self::WheelSlip,
        Self::Suspension,
        Self::Tyre,
        Self::Fuel,
    ];
    pub(super) const DEFAULT_VISIBLE: [Self; 3] = [Self::Speed, Self::Pedal, Self::Steering];

    pub(super) const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Speed => "speed",
            Self::Pedal => "pedal",
            Self::BrakePressure => "brake_pressure",
            Self::Abs => "abs",
            Self::Steering => "steering",
            Self::SteeringTorque => "steering_torque",
            Self::Rpm => "rpm",
            Self::Gear => "gear",
            Self::Dynamics => "dynamics",
            Self::Yaw => "yaw",
            Self::WheelSlip => "wheel_slip",
            Self::Tyre => "tyre",
            Self::Suspension => "suspension",
            Self::Fuel => "fuel",
            Self::Delta => "delta",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|chart| chart.key() == key)
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Speed => "Speed",
            Self::Pedal => "Pedal",
            Self::BrakePressure => "Brake pressure",
            Self::Abs => "ABS activity",
            Self::Steering => "Steering",
            Self::SteeringTorque => "Steering torque",
            Self::Rpm => "Engine RPM",
            Self::Gear => "Gear",
            Self::Dynamics => "Vehicle dynamics",
            Self::Yaw => "Yaw rate",
            Self::WheelSlip => "Wheel slip",
            Self::Tyre => "Tyre temperature",
            Self::Suspension => "Suspension travel",
            Self::Fuel => "Fuel used",
            Self::Delta => "Delta",
        }
    }

    pub(super) fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Speed => lucide::gauge(),
            Self::Pedal => lucide::sliders_vertical(),
            Self::BrakePressure => lucide::disc_three(),
            Self::Abs => lucide::shield_check(),
            Self::Steering => lucide::ship_wheel(),
            Self::SteeringTorque => lucide::rotate_cw(),
            Self::Rpm => lucide::circle_gauge(),
            Self::Gear => lucide::cog(),
            Self::Dynamics => lucide::activity(),
            Self::Yaw => lucide::rotate_cw(),
            Self::WheelSlip => lucide::circle_dot_dashed(),
            Self::Tyre => lucide::thermometer(),
            Self::Suspension => lucide::move_vertical(),
            Self::Fuel => lucide::fuel(),
            Self::Delta => lucide::diff(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LapAnalysisCardId {
    Cursor,
    ReferenceCursor,
    Vehicle,
    Inputs,
    Dynamics,
    Tyres,
    Wheels,
}

impl LapAnalysisCardId {
    pub(super) const ALL: [Self; 7] = [
        Self::Cursor,
        Self::ReferenceCursor,
        Self::Vehicle,
        Self::Inputs,
        Self::Dynamics,
        Self::Tyres,
        Self::Wheels,
    ];
    pub(super) const COUNT: usize = Self::ALL.len();

    pub(super) const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::ReferenceCursor => "reference_cursor",
            Self::Vehicle => "vehicle",
            Self::Inputs => "inputs",
            Self::Dynamics => "dynamics",
            Self::Tyres => "tyres",
            Self::Wheels => "wheels",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|card| card.key() == key)
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::ReferenceCursor => "Reference cursor",
            Self::Vehicle => "Vehicle",
            Self::Inputs => "Inputs",
            Self::Dynamics => "Dynamics",
            Self::Tyres => "Tyres",
            Self::Wheels => "Wheels",
        }
    }

    pub(super) fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Cursor => lucide::crosshair(),
            Self::ReferenceCursor => lucide::locate_fixed(),
            Self::Vehicle => lucide::car_front(),
            Self::Inputs => lucide::sliders_horizontal(),
            Self::Dynamics => lucide::activity(),
            Self::Tyres => lucide::circle_dashed(),
            Self::Wheels => lucide::disc_three(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCardId {
    Session,
    Reference,
    Laps,
    Charts,
}

impl SetupCardId {
    pub(super) const ALL: [Self; 4] = [Self::Session, Self::Reference, Self::Laps, Self::Charts];
    pub(super) const COUNT: usize = Self::ALL.len();

    pub(super) const fn index(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Reference => "reference",
            Self::Laps => "laps",
            Self::Charts => "charts",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|card| card.key() == key)
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Reference => "Reference",
            Self::Laps => "My laps",
            Self::Charts => "Charts",
        }
    }

    pub(super) fn icon(self) -> iced::widget::Text<'static> {
        match self {
            Self::Session => lucide::clipboard_list(),
            Self::Reference => lucide::target(),
            Self::Laps => lucide::timer(),
            Self::Charts => lucide::chart_line(),
        }
        .size(CARD_TITLE_ICON_SIZE)
    }
}

/// A stable, UI-independent boolean value in a persisted Telemetry layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryLayoutFlag {
    pub key: String,
    pub value: bool,
}

/// The user-customizable parts of the Telemetry screen layout.
///
/// Keys are stable across display-name changes. Unknown, duplicate, or missing
/// keys are normalized when the layout is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryLayout {
    pub chart_order: Vec<String>,
    pub chart_visibility: Vec<TelemetryLayoutFlag>,
    pub chart_collapsed: Vec<TelemetryLayoutFlag>,
    pub chart_columns: u8,
    pub setup_card_order: Vec<String>,
    pub setup_card_collapsed: Vec<TelemetryLayoutFlag>,
    pub lap_analysis_order: Vec<String>,
    pub lap_analysis_collapsed: Vec<TelemetryLayoutFlag>,
}

impl Default for TelemetryLayout {
    fn default() -> Self {
        Self {
            chart_order: ChartId::DEFAULT_ORDER
                .map(|chart| chart.key().to_owned())
                .to_vec(),
            chart_visibility: ChartId::ALL
                .map(|chart| TelemetryLayoutFlag {
                    key: chart.key().to_owned(),
                    value: ChartId::DEFAULT_VISIBLE.contains(&chart),
                })
                .to_vec(),
            chart_collapsed: ChartId::ALL
                .map(|chart| TelemetryLayoutFlag {
                    key: chart.key().to_owned(),
                    value: false,
                })
                .to_vec(),
            chart_columns: ChartColumns::One.persisted_value(),
            setup_card_order: SetupCardId::ALL.map(|card| card.key().to_owned()).to_vec(),
            setup_card_collapsed: SetupCardId::ALL
                .map(|card| TelemetryLayoutFlag {
                    key: card.key().to_owned(),
                    value: false,
                })
                .to_vec(),
            lap_analysis_order: LapAnalysisCardId::ALL
                .map(|card| card.key().to_owned())
                .to_vec(),
            lap_analysis_collapsed: LapAnalysisCardId::ALL
                .map(|card| TelemetryLayoutFlag {
                    key: card.key().to_owned(),
                    value: false,
                })
                .to_vec(),
        }
    }
}

pub(super) fn normalize_layout_order<Id: Copy + Eq, const N: usize>(
    keys: &[String],
    default_order: [Id; N],
    from_key: impl Fn(&str) -> Option<Id>,
) -> Vec<Id> {
    let mut normalized = Vec::with_capacity(N);
    for key in keys {
        let Some(id) = from_key(key) else {
            continue;
        };
        if !normalized.contains(&id) {
            normalized.push(id);
        }
    }
    for id in default_order {
        if !normalized.contains(&id) {
            normalized.push(id);
        }
    }
    normalized
}

pub(super) fn normalize_layout_flags<Id: Copy, const N: usize>(
    flags: &[TelemetryLayoutFlag],
    mut normalized: [bool; N],
    from_key: impl Fn(&str) -> Option<Id>,
    index: impl Fn(Id) -> usize,
) -> [bool; N] {
    let mut seen = [false; N];
    for flag in flags {
        let Some(id) = from_key(&flag.key) else {
            continue;
        };
        let index = index(id);
        if !seen[index] {
            normalized[index] = flag.value;
            seen[index] = true;
        }
    }
    normalized
}
