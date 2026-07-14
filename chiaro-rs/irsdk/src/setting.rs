use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Setting {
    pub bind_addr: String,
    pub require_registration: bool,
    pub mock_telemetry: bool,
}

impl Default for Setting {
    fn default() -> Self {
        Setting {
            bind_addr: "127.0.0.1:35565".to_string(),
            require_registration: false,
            mock_telemetry: false,
        }
    }
}

impl Setting {
    pub fn new() -> Result<Self, ConfigError> {
        let defaults = Self::default();
        let s = Config::builder()
            .set_default("bind_addr", defaults.bind_addr)?
            .set_default("require_registration", defaults.require_registration)?
            .set_default("mock_telemetry", defaults.mock_telemetry)?
            .add_source(File::with_name("settings").required(false))
            .build()?;

        s.try_deserialize()
    }
}
