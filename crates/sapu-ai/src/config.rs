//! Provider configuration, stored as JSON in the OS config dir. API keys
//! are never in this file (see `keys`).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMode {
    #[default]
    Off,
    Local,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudKind {
    #[default]
    OpenAiCompatible,
    Anthropic,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub mode: AiMode,
    /// Ollama or any OpenAI-compatible local server.
    pub local_url: String,
    /// Empty = first model Ollama reports as installed.
    pub local_model: String,
    pub cloud_kind: CloudKind,
    pub cloud_url: String,
    pub cloud_model: String,
    /// Replace home folder and account name before sending (§12.4).
    pub anonymize: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            mode: AiMode::Off,
            local_url: "http://localhost:11434".into(),
            local_model: String::new(),
            cloud_kind: CloudKind::OpenAiCompatible,
            cloud_url: "https://api.openai.com".into(),
            cloud_model: String::new(),
            anonymize: true,
        }
    }
}

impl AiConfig {
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("sapu").join("ai.json"))
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?,
        )
    }

    pub fn is_enabled(&self) -> bool {
        self.mode != AiMode::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_private() {
        let c = AiConfig::default();
        assert_eq!(c.mode, AiMode::Off);
        assert!(c.anonymize);
        assert!(!c.is_enabled());
    }

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a/ai.json");
        let c = AiConfig {
            mode: AiMode::Local,
            local_model: "x".into(),
            ..Default::default()
        };
        c.save(&p).expect("save");
        assert_eq!(AiConfig::load(&p), c);
    }
}
