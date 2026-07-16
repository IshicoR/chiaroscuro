use std::{collections::BTreeMap, error::Error, fmt};

use serde::Deserialize;
use serde_yaml_ng::Value;

mod camera;
mod driver;
mod radio;
mod session;
mod split_time;
mod value;
mod weekend;

pub use camera::{Camera, CameraGroup, CameraInfo};
pub use driver::{Driver, DriverInfo, DriverTire};
pub use radio::{Radio, RadioFrequency, RadioInfo};
pub use session::{
    FastestLap, QualifyingResult, QualifyingResultsInfo, Session, SessionInfoSection, SessionResult,
};
pub use split_time::{Sector, SplitTimeInfo};
pub use value::{SdkBool, SessionScalar};
pub use weekend::{TelemetryOptions, WeekendInfo, WeekendOptions};

/// Fields added by a newer SDK or specific to an iRacing session mode.
pub type SessionInfoExtra = BTreeMap<String, Value>;

/// The slowly-changing YAML session information published by iRacing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub update_count: i32,
    pub yaml: String,
    pub raw: Vec<u8>,
}

impl SessionInfo {
    pub(crate) fn from_raw(update_count: i32, raw: Vec<u8>) -> Self {
        let encoding = session_info_encoding(&raw);
        let yaml = match encoding {
            SessionInfoEncoding::Iso8859_1 => raw.iter().copied().map(char::from).collect(),
            SessionInfoEncoding::Utf8 => String::from_utf8_lossy(&raw).into_owned(),
        };
        Self {
            update_count,
            yaml,
            raw,
        }
    }

    /// Returns the character encoding declared by the session document.
    pub fn encoding(&self) -> SessionInfoEncoding {
        session_info_encoding(&self.raw)
    }

    /// Parses the YAML while keeping this raw envelope available as a fallback.
    pub fn parse(&self) -> Result<SessionInfoDocument, SessionInfoParseError> {
        serde_yaml_ng::from_str(&self.yaml).map_err(SessionInfoParseError::new)
    }
}

/// Character encodings used by iRacing session information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionInfoEncoding {
    Iso8859_1,
    Utf8,
}

fn session_info_encoding(raw: &[u8]) -> SessionInfoEncoding {
    #[derive(Deserialize)]
    struct DocumentEncoding {
        #[serde(rename = "WeekendInfo")]
        weekend_info: Option<WeekendEncoding>,
    }

    #[derive(Deserialize)]
    struct WeekendEncoding {
        #[serde(rename = "Encoding")]
        encoding: Option<String>,
    }

    let declares_utf8 = std::str::from_utf8(raw)
        .ok()
        .and_then(|yaml| serde_yaml_ng::from_str::<DocumentEncoding>(yaml).ok())
        .and_then(|document| document.weekend_info)
        .and_then(|weekend| weekend.encoding)
        .is_some_and(|encoding| encoding.eq_ignore_ascii_case("UTF8"));

    if declares_utf8 {
        SessionInfoEncoding::Utf8
    } else {
        SessionInfoEncoding::Iso8859_1
    }
}

/// Typed stable sections of the iRacing session YAML document.
///
/// Sections can be absent while spectating, replaying, or transitioning
/// between sessions. Car setup data and unknown top-level sections are kept
/// as generic YAML because their shape is not part of a stable SDK contract.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct SessionInfoDocument {
    #[serde(rename = "WeekendInfo")]
    pub weekend_info: Option<WeekendInfo>,
    #[serde(rename = "SessionInfo")]
    pub session_info: Option<SessionInfoSection>,
    #[serde(rename = "QualifyResultsInfo")]
    pub qualify_results_info: Option<QualifyingResultsInfo>,
    #[serde(rename = "DriverInfo")]
    pub driver_info: Option<DriverInfo>,
    #[serde(rename = "CameraInfo")]
    pub camera_info: Option<CameraInfo>,
    #[serde(rename = "RadioInfo")]
    pub radio_info: Option<RadioInfo>,
    #[serde(rename = "SplitTimeInfo")]
    pub split_time_info: Option<SplitTimeInfo>,
    #[serde(rename = "CarSetup")]
    pub car_setup: Option<Value>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// A YAML syntax or type error encountered while parsing session information.
#[derive(Debug)]
pub struct SessionInfoParseError {
    source: serde_yaml_ng::Error,
}

impl SessionInfoParseError {
    fn new(source: serde_yaml_ng::Error) -> Self {
        Self { source }
    }
}

impl fmt::Display for SessionInfoParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to parse iRacing session information: {}",
            self.source
        )
    }
}

impl Error for SessionInfoParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml_ng::Value;

    use super::{SdkBool, SessionInfo, SessionInfoEncoding, SessionScalar};

    const REPRESENTATIVE_SESSION: &str = include_str!("fixtures/representative.yaml");

    fn parse_fixture() -> super::SessionInfoDocument {
        SessionInfo {
            update_count: 12,
            yaml: REPRESENTATIVE_SESSION.to_owned(),
            raw: REPRESENTATIVE_SESSION.as_bytes().to_vec(),
        }
        .parse()
        .expect("representative iRacing YAML should parse")
    }

    #[test]
    fn parses_stable_sections_without_discarding_dynamic_sections() {
        let document = parse_fixture();
        let weekend = document.weekend_info.expect("WeekendInfo");
        assert_eq!(weekend.track_id, Some(403));
        assert_eq!(weekend.track_length.as_deref(), Some("4.28 km"));
        assert_eq!(weekend.track_config_name, None);
        assert_eq!(weekend.track_precipitation.as_deref(), Some("0 %"));
        assert!(matches!(
            weekend
                .weekend_options
                .as_ref()
                .and_then(|options| options.incident_limit.as_ref()),
            Some(SessionScalar::String(value)) if value == "unlimited"
        ));

        assert!(matches!(document.car_setup, Some(Value::Mapping(_))));
        assert!(document.extra.contains_key("SessionLogInfo"));
    }

    #[test]
    fn accepts_nulls_mixed_scalars_and_integer_flags() {
        let document = parse_fixture();
        let driver_info = document.driver_info.expect("DriverInfo");
        let driver = driver_info.drivers.first().expect("driver");
        assert_eq!(driver.abbrev_name, None);
        assert!(matches!(
            driver.car_class_color,
            Some(SessionScalar::Integer(0x00ff_ffff))
        ));
        assert!(matches!(
            driver.license_color,
            Some(SessionScalar::String(ref value)) if value == "0xundefined"
        ));
        assert_eq!(driver.is_spectator.and_then(SdkBool::as_bool), Some(false));

        let sessions = &document.session_info.expect("SessionInfo").sessions;
        assert!(matches!(
            sessions[0].session_laps,
            Some(SessionScalar::String(ref value)) if value == "unlimited"
        ));
        assert!(matches!(
            sessions[1].session_laps,
            Some(SessionScalar::Integer(50))
        ));
        assert_eq!(sessions[0].session_sub_type, None);
    }

    #[test]
    fn accepts_legacy_numeric_night_mode_and_session_count() {
        let yaml = "WeekendInfo:\n  WeekendOptions:\n    NightMode: 0\nSessionInfo:\n  NumSessions: 1\n  Sessions: []\n";
        let document = SessionInfo {
            update_count: 1,
            yaml: yaml.to_owned(),
            raw: yaml.as_bytes().to_vec(),
        }
        .parse()
        .expect("legacy scalar forms should parse");

        assert!(matches!(
            document
                .weekend_info
                .and_then(|weekend| weekend.weekend_options)
                .and_then(|options| options.night_mode),
            Some(SessionScalar::Integer(0))
        ));
        assert_eq!(
            document
                .session_info
                .and_then(|session_info| session_info.num_sessions),
            Some(1)
        );
    }

    #[test]
    fn accepts_null_sequences_and_preserves_nested_unknown_fields() {
        let yaml = r#"
WeekendInfo:
  FutureTrackField: retained
SessionInfo:
  CurrentSessionNum: 2
  FutureSessionField: 17
  Sessions:
    - SessionNum: 2
      QualifyPositions:
      ResultsPositions: null
      ResultsFastestLap: ~
      FuturePhaseField: retained
QualifyResultsInfo:
  Results:
DriverInfo:
  DriverTires: null
  Drivers:
CameraInfo:
  Groups: null
RadioInfo:
  Radios:
SplitTimeInfo:
  Sectors: ~
"#;
        let info = SessionInfo::from_raw(2, yaml.as_bytes().to_vec());
        let document = info.parse().expect("null sequences should become empty");

        let weekend = document.weekend_info.expect("WeekendInfo");
        assert_eq!(
            weekend.extra.get("FutureTrackField"),
            Some(&Value::String("retained".to_owned()))
        );

        let session_info = document.session_info.expect("SessionInfo");
        assert_eq!(session_info.current_session_num, Some(2));
        assert_eq!(
            session_info.extra.get("FutureSessionField"),
            Some(&Value::Number(17.into()))
        );
        assert!(session_info.sessions[0].qualify_positions.is_empty());
        assert!(session_info.sessions[0].results_positions.is_empty());
        assert!(session_info.sessions[0].results_fastest_lap.is_empty());
        assert!(
            session_info.sessions[0]
                .extra
                .contains_key("FuturePhaseField")
        );

        assert!(
            document
                .qualify_results_info
                .expect("QualifyResultsInfo")
                .results
                .is_empty()
        );
        assert!(document.driver_info.expect("DriverInfo").drivers.is_empty());
        assert!(document.camera_info.expect("CameraInfo").groups.is_empty());
        assert!(document.radio_info.expect("RadioInfo").radios.is_empty());
        assert!(
            document
                .split_time_info
                .expect("SplitTimeInfo")
                .sectors
                .is_empty()
        );
    }

    #[test]
    fn parses_driver_tires_camera_radio_and_sectors() {
        let document = parse_fixture();
        let driver_info = document.driver_info.expect("DriverInfo");
        assert_eq!(driver_info.driver_tires[1].tire_index, Some(1));
        assert_eq!(
            driver_info.driver_tires[1].tire_compound_type.as_deref(),
            Some("Wet")
        );

        let camera_info = document.camera_info.expect("CameraInfo");
        assert_eq!(camera_info.groups[1].is_scenic, Some(true));
        assert_eq!(camera_info.groups[0].is_scenic, None);

        let radio_info = document.radio_info.expect("RadioInfo");
        assert_eq!(
            radio_info.radios[0]
                .scanning_is_on
                .and_then(SdkBool::as_bool),
            Some(true)
        );

        let split_info = document.split_time_info.expect("SplitTimeInfo");
        assert_eq!(split_info.sectors[1].sector_start_pct, Some(0.271918));
    }

    #[test]
    fn reports_invalid_yaml_without_modifying_raw_data() {
        let raw = b"WeekendInfo: [".to_vec();
        let info = SessionInfo::from_raw(3, raw.clone());

        assert!(info.parse().is_err());
        assert_eq!(info.raw, raw);
        assert_eq!(info.yaml, "WeekendInfo: [");
    }

    #[test]
    fn decodes_iso_8859_1_and_retains_source_bytes() {
        let raw = vec![b'J', 0xfc, b'r', b'g', b'e', b'n'];
        let info = SessionInfo::from_raw(7, raw.clone());

        assert_eq!(info.update_count, 7);
        assert_eq!(info.yaml, "J\u{fc}rgen");
        assert_eq!(info.raw, raw);
        assert_eq!(info.encoding(), SessionInfoEncoding::Iso8859_1);
    }

    #[test]
    fn decodes_legacy_control_range_as_iso_8859_1() {
        let info = SessionInfo::from_raw(7, vec![0x80]);

        assert_eq!(info.yaml, "\u{80}");
        assert_eq!(info.encoding(), SessionInfoEncoding::Iso8859_1);
    }

    #[test]
    fn honors_the_utf8_encoding_declared_by_weekend_info() {
        let yaml = "WeekendInfo:\n  Encoding: UTF8\n  TrackCity: M\u{00fc}nchen\n";
        let info = SessionInfo::from_raw(8, yaml.as_bytes().to_vec());

        assert_eq!(info.encoding(), SessionInfoEncoding::Utf8);
        assert_eq!(info.yaml, yaml);
        assert_eq!(
            info.parse()
                .expect("UTF8 session info")
                .weekend_info
                .and_then(|weekend| weekend.track_city)
                .as_deref(),
            Some("M\u{00fc}nchen")
        );
    }
}
