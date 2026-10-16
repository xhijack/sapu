//! A1 "Apa ini?": explain an unknown folder or file from its metadata.

use super::{LANGUAGE_RULE, Task, Verdict, confidence, extract_json, text};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhatIsInput {
    pub language: String,
    pub os: String,
    pub path: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub file_count: u64,
    /// Most common extensions with counts, e.g. `[["jpg", 120], ["mov", 4]]`.
    pub top_extensions: Vec<(String, u64)>,
    pub newest_age_days: Option<u64>,
    pub oldest_age_days: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WhatIsAnswer {
    pub summary: String,
    pub likely_owner: String,
    pub verdict: Verdict,
    pub consequence: String,
    pub confidence: f32,
    pub reason: String,
}

pub struct WhatIs;

impl Task for WhatIs {
    const NAME: &'static str = "whatis";
    type Input = WhatIsInput;
    type Output = WhatIsAnswer;

    fn system_prompt() -> &'static str {
        "You help a person understand what a folder or file on their computer is, using only its path and metadata. Be honest about uncertainty. Never claim something is safe to delete unless it is clearly a cache, log or regenerable artefact; personal documents, photos and anything under credentials are never removable."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly this JSON: {{\"summary\": \"one sentence: what this is\", \"likely_owner\": \"app or system that owns it, or 'user data'\", \"removable\": \"yes|maybe|no\", \"consequence\": \"what happens if removed\", \"confidence\": 0.0-1.0, \"reason\": \"why you think so\"}}\nMetadata:\n{input}"
        )
    }

    fn parse(answer: &str) -> Result<WhatIsAnswer, AiError> {
        let v = extract_json(answer)?;
        let summary = text(&v, "summary");
        if summary.is_empty() {
            return Err(AiError::BadAnswer("missing summary".into()));
        }
        Ok(WhatIsAnswer {
            summary,
            likely_owner: text(&v, "likely_owner"),
            verdict: Verdict::from_model(&text(&v, "removable")),
            consequence: text(&v, "consequence"),
            confidence: confidence(&v),
            reason: text(&v, "reason"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_caps_verdict() {
        let a = WhatIs::parse(r#"{"summary":"Cache Spotify","likely_owner":"Spotify","removable":"yes","consequence":"lagu offline diunduh ulang","confidence":1.7,"reason":"folder Caches"}"#).expect("parse");
        assert_eq!(a.verdict, Verdict::CheckFirst);
        assert_eq!(a.confidence, 1.0);
        assert!(WhatIs::parse("{}").is_err());
    }
}
