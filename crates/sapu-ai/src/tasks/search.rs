//! A2: natural-language sentence → strict, bounded filter that core executes.

use super::{LANGUAGE_RULE, Task, extract_json};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchInput {
    pub language: String,
    pub sentence: String,
    pub categories: Vec<String>,
}

/// The only shape a search answer may take (PRD AI-5). Unknown keys are
/// rejected; values are bounded on parse.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SearchFilter {
    pub category: Option<String>,
    pub min_bytes: Option<u64>,
    pub min_age_days: Option<u64>,
    pub max_age_days: Option<u64>,
    pub extensions: Vec<String>,
    pub name_contains: Option<String>,
    /// Model's restatement of the query, for display.
    pub description: String,
}

pub struct Search;

impl Task for Search {
    const NAME: &'static str = "search";
    type Input = SearchInput;
    type Output = SearchFilter;

    fn system_prompt() -> &'static str {
        "Translate a person's sentence about files into a structured filter. Only use the allowed keys. Sizes in bytes (1 GB = 1073741824). Ages in days since the file was last modified: 'older than a year' → min_age_days 365; 'this month' → max_age_days 30."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nAllowed keys: category (one of the given categories or null), min_bytes, min_age_days, max_age_days, extensions (lowercase, no dot), name_contains, description (restate the filter in one short sentence).\nReturn one JSON object with only those keys.\nInput:\n{input}"
        )
    }

    fn parse(answer: &str) -> Result<SearchFilter, AiError> {
        let v = extract_json(answer)?;
        let mut f: SearchFilter =
            serde_json::from_value(v).map_err(|e| AiError::BadAnswer(e.to_string()))?;
        f.extensions = f
            .extensions
            .into_iter()
            .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
            .filter(|e| !e.is_empty() && e.len() <= 10)
            .take(20)
            .collect();
        if let Some(n) = &mut f.name_contains {
            n.truncate(100);
        }
        f.description.truncate(200);
        if f.category.is_none()
            && f.min_bytes.is_none()
            && f.min_age_days.is_none()
            && f.max_age_days.is_none()
            && f.extensions.is_empty()
            && f.name_contains.is_none()
        {
            return Err(AiError::BadAnswer("empty filter".into()));
        }
        Ok(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_schema() {
        let f = Search::parse(r#"{"category":"video","min_bytes":1073741824,"min_age_days":365,"extensions":[".MP4","mov"],"description":"video besar lama"}"#).expect("parse");
        assert_eq!(f.extensions, vec!["mp4", "mov"]);
        assert_eq!(f.min_age_days, Some(365));
        assert!(
            Search::parse(r#"{"delete":true,"category":"video"}"#).is_err(),
            "unknown keys rejected"
        );
        assert!(
            Search::parse(r#"{"description":"nothing"}"#).is_err(),
            "empty filter rejected"
        );
    }
}
