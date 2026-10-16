//! A7: contextual analysis of build artefacts from deterministic signals.

use super::{LANGUAGE_RULE, Task, Verdict, confidence, extract_json, text};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtefactSignal {
    pub kind: String,
    pub bytes: u64,
    pub gitignored: bool,
    /// None = project has no git.
    pub tracked: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtefactInput {
    pub language: String,
    pub project_name: String,
    pub ecosystem: String,
    pub frameworks: Vec<String>,
    pub scripts: Vec<String>,
    pub idle_days: Option<u64>,
    pub rebuild: Vec<String>,
    pub artefacts: Vec<ArtefactSignal>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArtefactVerdict {
    pub kind: String,
    pub regenerable: bool,
    pub verdict: Verdict,
    pub explanation: String,
    pub rebuild: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArtefactAnswer {
    pub project_summary: String,
    pub artefacts: Vec<ArtefactVerdict>,
}

pub struct Artefact;

impl Task for Artefact {
    const NAME: &'static str = "artefact";
    type Input = ArtefactInput;
    type Output = ArtefactAnswer;

    fn system_prompt() -> &'static str {
        "You are a senior developer explaining to a colleague whether build artefacts of a project can be deleted and recreated. You only see derived signals, never source code. A folder that git tracks is never an artefact. Prefer caution when signals conflict."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly: {{\"project_summary\": \"one sentence about the project and its activity\", \"artefacts\": [{{\"kind\": \"folder name\", \"regenerable\": true|false, \"removable\": \"yes|maybe|no\", \"explanation\": \"one or two sentences\", \"rebuild\": \"command to recreate\", \"confidence\": 0.0-1.0}}]}}\nSignals:\n{input}"
        )
    }

    fn parse(answer: &str) -> Result<ArtefactAnswer, AiError> {
        let v = extract_json(answer)?;
        let artefacts = v["artefacts"]
            .as_array()
            .ok_or_else(|| AiError::BadAnswer("missing artefacts".into()))?
            .iter()
            .map(|a| ArtefactVerdict {
                kind: text(a, "kind"),
                regenerable: a["regenerable"].as_bool().unwrap_or(false),
                verdict: Verdict::from_model(&text(a, "removable")),
                explanation: text(a, "explanation"),
                rebuild: text(a, "rebuild"),
                confidence: confidence(a),
            })
            .filter(|a| !a.kind.is_empty())
            .collect();
        Ok(ArtefactAnswer {
            project_summary: text(&v, "project_summary"),
            artefacts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list() {
        let a = Artefact::parse(r#"{"project_summary":"Landing page Next.js, 7 bulan tidak disentuh","artefacts":[{"kind":".next","regenerable":true,"removable":"yes","explanation":"output build","rebuild":"pnpm run build","confidence":0.9},{"kind":"","removable":"no"}]}"#).expect("parse");
        assert_eq!(a.artefacts.len(), 1);
        assert_eq!(a.artefacts[0].verdict, Verdict::CheckFirst);
        assert!(a.artefacts[0].regenerable);
    }
}
