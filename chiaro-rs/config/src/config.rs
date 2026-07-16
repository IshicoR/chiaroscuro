use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use config::{Config, ConfigError, File};
use serde::Deserialize;

const APPLICATION_DIRECTORY: &str = "chiaroscuro";
const SETTINGS_FILE: &str = "settings.toml";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DesktopConfig {
    pub show_diagnostics: bool,
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
        fs::write(path, self.encode())
    }

    fn encode(&self) -> String {
        format!("show_diagnostics = {}\n", self.show_diagnostics)
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
    use std::{ffi::OsStr, path::PathBuf};

    use super::{DesktopConfig, settings_path_from};

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
        };

        assert_eq!(config.encode(), "show_diagnostics = true\n");
    }
}
