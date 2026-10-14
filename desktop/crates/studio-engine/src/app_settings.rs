//! Preferences of the studio app itself, kept in `app-settings.json` under the app data
//! directory. A missing or unreadable file means the defaults; settings never stop the
//! app from starting.

use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use studio_proto::AppTheme;

use crate::app_paths::AppPaths;

pub const APP_SETTINGS_FILE: &str = "app-settings.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Light, dark, or following the operating system.
    pub theme: AppTheme,
}

pub fn app_settings_path(paths: &AppPaths) -> PathBuf {
    paths.data.join(APP_SETTINGS_FILE)
}

impl AppSettings {
    /// The saved settings, or the defaults when none were saved or the file cannot be read.
    pub fn load(paths: &AppPaths) -> Self {
        std::fs::read_to_string(app_settings_path(paths))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Replaces the saved settings through a temporary file renamed over the old one.
    pub fn save(&self, paths: &AppPaths) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Cannot encode the app settings: {e}"))?;
        let mut file = tempfile::NamedTempFile::new_in(&paths.data)
            .map_err(|e| format!("Cannot write the app settings: {e}"))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.as_file().sync_all())
            .map_err(|e| format!("Cannot write the app settings: {e}"))?;
        file.persist(app_settings_path(paths))
            .map_err(|e| format!("Cannot save the app settings: {}", e.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_survives_a_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path()).unwrap();
        assert_eq!(AppSettings::load(&paths), AppSettings::default());
        assert_eq!(AppSettings::load(&paths).theme, AppTheme::System);

        let dark = AppSettings {
            theme: AppTheme::Dark,
        };
        dark.save(&paths).unwrap();
        assert_eq!(AppSettings::load(&paths), dark);
        AppSettings {
            theme: AppTheme::Light,
        }
        .save(&paths)
        .unwrap();
        assert_eq!(AppSettings::load(&paths).theme, AppTheme::Light);
        let leftovers = std::fs::read_dir(&paths.data).unwrap().count();
        assert_eq!(leftovers, 1, "only the settings file remains");
    }

    #[test]
    fn an_unreadable_or_partial_file_means_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path()).unwrap();
        std::fs::write(app_settings_path(&paths), "{ not json").unwrap();
        assert_eq!(AppSettings::load(&paths), AppSettings::default());
        std::fs::write(app_settings_path(&paths), "{}").unwrap();
        assert_eq!(AppSettings::load(&paths).theme, AppTheme::System);
        std::fs::write(app_settings_path(&paths), r#"{"theme":"dark","later":1}"#).unwrap();
        assert_eq!(AppSettings::load(&paths).theme, AppTheme::Dark);
    }
}
