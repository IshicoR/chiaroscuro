use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};

const APPLICATION_DIRECTORY: &str = "chiaroscuro";
const SETTINGS_FILE: &str = "settings.toml";
pub const DASHBOARD_LAYOUT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub show_diagnostics: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard: Option<DashboardLayoutConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardLayoutConfig {
    pub schema_version: u32,
    pub chart_order: Vec<String>,
    pub chart_visibility: BTreeMap<String, bool>,
    pub chart_collapsed: BTreeMap<String, bool>,
    pub chart_columns: u8,
    pub setup_card_order: Vec<String>,
    pub setup_card_collapsed: BTreeMap<String, bool>,
    pub lap_analysis_order: Vec<String>,
    pub lap_analysis_collapsed: BTreeMap<String, bool>,
    pub car_setup_card_order: Vec<String>,
    pub car_setup_card_collapsed: BTreeMap<String, bool>,
}

impl Default for DashboardLayoutConfig {
    fn default() -> Self {
        Self {
            schema_version: DASHBOARD_LAYOUT_SCHEMA_VERSION,
            chart_order: Vec::new(),
            chart_visibility: BTreeMap::new(),
            chart_collapsed: BTreeMap::new(),
            chart_columns: 1,
            setup_card_order: Vec::new(),
            setup_card_collapsed: BTreeMap::new(),
            lap_analysis_order: Vec::new(),
            lap_analysis_collapsed: BTreeMap::new(),
            car_setup_card_order: Vec::new(),
            car_setup_card_collapsed: BTreeMap::new(),
        }
    }
}

impl DesktopConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let defaults = Self::default();
        let mut builder =
            Config::builder().set_default("show_diagnostics", defaults.show_diagnostics)?;

        if let Some(path) = settings_path() {
            builder = builder.add_source(File::from(path).required(false));
        }

        builder.build()?.try_deserialize()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = settings_path().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME, APPDATA nor HOME is available",
            )
        })?;
        let Some(parent) = path.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "desktop settings path has no parent directory",
            ));
        };

        fs::create_dir_all(parent)?;
        fs::write(path, self.encode()?)
    }

    fn encode(&self) -> io::Result<String> {
        toml::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

pub fn settings_path() -> Option<PathBuf> {
    settings_path_from(
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("APPDATA").as_deref(),
        env::var_os("HOME").as_deref(),
    )
}

fn settings_path_from(
    xdg_config_home: Option<&OsStr>,
    app_data: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    let base = xdg_config_home
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| app_data.filter(|path| !path.is_empty()).map(PathBuf::from))
        .or_else(|| {
            home.filter(|path| !path.is_empty())
                .map(|path| Path::new(path).join(".config"))
        })?;

    Some(base.join(APPLICATION_DIRECTORY).join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

    use super::{
        DASHBOARD_LAYOUT_SCHEMA_VERSION, DashboardLayoutConfig, DesktopConfig, settings_path_from,
    };

    #[test]
    fn settings_path_prefers_xdg_config_home() {
        let path = settings_path_from(
            Some(OsStr::new("/tmp/xdg")),
            Some(OsStr::new("/tmp/appdata")),
            Some(OsStr::new("/tmp/home")),
        );

        assert_eq!(
            path,
            Some(PathBuf::from("/tmp/xdg/chiaroscuro/settings.toml"))
        );
    }

    #[test]
    fn encodes_desktop_settings_as_toml() {
        let config = DesktopConfig {
            show_diagnostics: true,
            dashboard: None,
        };

        assert_eq!(config.encode().unwrap(), "show_diagnostics = true\n");
    }

    #[test]
    fn loads_legacy_desktop_settings_without_dashboard_layout() {
        let config: DesktopConfig = toml::from_str("show_diagnostics = true\n").unwrap();

        assert!(config.show_diagnostics);
        assert_eq!(config.dashboard, None);
    }

    #[test]
    fn dashboard_layout_round_trips_through_toml() {
        let dashboard = DashboardLayoutConfig {
            schema_version: DASHBOARD_LAYOUT_SCHEMA_VERSION,
            chart_order: vec!["speed".into(), "pedal".into(), "steering".into()],
            chart_visibility: BTreeMap::from([
                ("pedal".into(), true),
                ("speed".into(), true),
                ("steering".into(), false),
            ]),
            chart_collapsed: BTreeMap::from([("pedal".into(), true)]),
            chart_columns: 2,
            setup_card_order: vec!["session".into(), "charts".into(), "laps".into()],
            setup_card_collapsed: BTreeMap::from([("session".into(), false)]),
            lap_analysis_order: vec!["cursor".into(), "vehicle".into(), "inputs".into()],
            lap_analysis_collapsed: BTreeMap::from([("cursor".into(), true)]),
            car_setup_card_order: vec![
                "summary".into(),
                "vehicle:specifications".into(),
                "setup:tires".into(),
            ],
            car_setup_card_collapsed: BTreeMap::from([("setup:tires".into(), true)]),
        };
        let config = DesktopConfig {
            show_diagnostics: false,
            dashboard: Some(dashboard),
        };

        let encoded = config.encode().unwrap();
        let decoded: DesktopConfig = toml::from_str(&encoded).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn omits_dashboard_table_when_layout_is_not_saved() {
        let encoded = DesktopConfig::default().encode().unwrap();

        assert!(!encoded.contains("dashboard"));
        assert_eq!(encoded, "show_diagnostics = false\n");
    }

    #[test]
    fn missing_dashboard_fields_use_layout_defaults() {
        let config: DesktopConfig = toml::from_str("[dashboard]\n").unwrap();
        let dashboard = config.dashboard.unwrap();

        assert_eq!(dashboard.schema_version, DASHBOARD_LAYOUT_SCHEMA_VERSION);
        assert_eq!(dashboard.chart_columns, 1);
        assert!(dashboard.chart_order.is_empty());
        assert!(dashboard.chart_visibility.is_empty());
        assert!(dashboard.car_setup_card_order.is_empty());
        assert!(dashboard.car_setup_card_collapsed.is_empty());
    }
}
