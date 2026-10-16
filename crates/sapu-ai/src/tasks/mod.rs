//! Metadata-only tasks. Each task defines its input, the prompt, and a
//! strict parser for the model's JSON answer.

pub mod artefact;
pub mod report;
pub mod search;
pub mod whatis;

use crate::AiError;

/// The only labels an AI answer can carry. There is deliberately no
/// "safe" variant (PRD AI-2): green comes from deterministic rules only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Do not remove.
    Keep,
    /// Probably removable; the user should check first.
    CheckFirst,
}

impl Verdict {
    /// Maps the free-form field models return to a verdict. Anything that
    /// is not clearly "removable" becomes `Keep`.
    pub fn from_model(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "yes" | "removable" | "safe" | "check_first" | "maybe" | "probably" => Self::CheckFirst,
            _ => Self::Keep,
        }
    }
}

pub trait Task {
    const NAME: &'static str;
    type Input: serde::Serialize;
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    fn system_prompt() -> &'static str;
    fn user_prompt(input: &serde_json::Value) -> String;
    fn parse(answer: &str) -> Result<Self::Output, AiError>;
}

/// Extracts the first JSON object from a model answer, tolerating prose
/// and code fences around it.
pub fn extract_json(answer: &str) -> Result<serde_json::Value, AiError> {
    let start = answer
        .find('{')
        .ok_or_else(|| AiError::BadAnswer(answer.chars().take(120).collect()))?;
    let end = answer
        .rfind('}')
        .ok_or_else(|| AiError::BadAnswer(answer.chars().take(120).collect()))?;
    if end < start {
        return Err(AiError::BadAnswer(answer.chars().take(120).collect()));
    }
    serde_json::from_str(&answer[start..=end]).map_err(|e| AiError::BadAnswer(e.to_string()))
}

pub(crate) fn confidence(v: &serde_json::Value) -> f32 {
    v.get("confidence")
        .and_then(|c| c.as_f64())
        .map(|c| c.clamp(0.0, 1.0) as f32)
        .unwrap_or(0.5)
}

pub(crate) fn text(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub(crate) const LANGUAGE_RULE: &str = "Answer in the language of the field `language` in the input (id = Bahasa Indonesia, en = English). Use plain words a non-technical person understands. Output JSON only, no prose around it.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_fences_and_prose() {
        let v = extract_json("Sure! ```json\n{\"a\": 1}\n``` done").expect("json");
        assert_eq!(v["a"], 1);
        assert!(extract_json("no json here").is_err());
    }

    #[test]
    fn verdict_never_safe() {
        assert_eq!(Verdict::from_model("yes"), Verdict::CheckFirst);
        assert_eq!(Verdict::from_model("no"), Verdict::Keep);
        assert_eq!(Verdict::from_model("unsure"), Verdict::Keep);
    }
}
