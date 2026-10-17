//! User settings shared by GUI and CLI, stored as JSON in the OS config dir.

use std::path::{Path, PathBuf};

use crate::error::{Result, SapuError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    #[default]
    General,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMode {
    #[default]
    Off,
    Local,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub onboarding_done: bool,
    pub profile: Profile,
    /// BCP-47 tag, e.g. `id` or `en`. `None` = follow the system.
    pub locale: Option<String>,
    pub ai_mode: AiMode,
    pub theme: Theme,
    /// System notification when a long scan/clean/dedupe finishes.
    pub notify_when_done: bool,
    /// Menu bar / tray monitor with disk, memory and CPU (needs restart).
    pub menu_bar_monitor: bool,
    /// The user has read and accepted the no-warranty notice. Shown before
    /// anything else on first run, and once more for settings from before
    /// the notice existed.
    pub disclaimer_accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            profile: Profile::General,
            locale: None,
            ai_mode: AiMode::Off,
            theme: Theme::System,
            notify_when_done: true,
            menu_bar_monitor: true,
            disclaimer_accepted: false,
        }
    }
}

impl Settings {
    pub fn default_path() -> Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| {
            SapuError::io(
                "locate config dir",
                std::io::Error::new(std::io::ErrorKind::NotFound, "no config dir"),
            )
        })?;
        Ok(base.join("sapu").join("settings.json"))
    }

    /// Missing file → defaults. Corrupt file → error (never silently reset).
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(SapuError::io("read settings", e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SapuError::io("create config dir", e))?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, json).map_err(|e| SapuError::io("write settings", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let s = Settings::load(&tmp.path().join("nope.json")).expect("load");
        assert_eq!(s, Settings::default());
        assert_eq!(s.ai_mode, AiMode::Off, "AI is off by default (P5)");
    }

    #[test]
    fn roundtrip_and_unknown_fields_get_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("cfg/settings.json");
        let s = Settings {
            onboarding_done: true,
            profile: Profile::Developer,
            locale: Some("id".into()),
            ai_mode: AiMode::Local,
            theme: Theme::Dark,
            notify_when_done: false,
            menu_bar_monitor: false,
            disclaimer_accepted: true,
        };
        s.save(&p).expect("save");
        assert_eq!(Settings::load(&p).expect("load"), s);

        std::fs::write(&p, br#"{"profile":"developer"}"#).expect("write");
        let partial = Settings::load(&p).expect("load");
        assert_eq!(partial.profile, Profile::Developer);
        assert!(!partial.onboarding_done);
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_reset() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("settings.json");
        std::fs::write(&p, b"{oops").expect("write");
        assert!(Settings::load(&p).is_err());
    }
}
