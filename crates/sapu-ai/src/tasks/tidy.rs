//! Tidy: name and merge candidate groups of a folder from file names only.

use super::{LANGUAGE_RULE, Task, extract_json};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyCandidate {
    pub id: String,
    pub name: String,
    pub source: String,
    pub count: usize,
    /// At most twelve file names, chosen by the caller.
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyInput {
    pub language: String,
    pub folder_name: String,
    pub candidates: Vec<TidyCandidate>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyName {
    pub id: String,
    pub name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyAnswer {
    pub groups: Vec<TidyName>,
    pub merge: Vec<(String, String)>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    groups: Vec<WireName>,
    #[serde(default)]
    merge: Vec<Vec<String>>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireName {
    id: String,
    name: String,
    #[serde(default = "half")]
    confidence: f64,
}

fn half() -> f64 {
    0.5
}

pub struct Tidy;

impl Task for Tidy {
    const NAME: &'static str = "tidy";
    type Input = TidyInput;
    type Output = TidyAnswer;

    fn system_prompt() -> &'static str {
        "You help tidy a messy folder. You only see candidate groups with a few example file names, never file contents. Give each group a short, specific folder name a person would recognise (a project, a client, a topic, or a kind of file), and propose merging groups that clearly belong together (for example a backend and a frontend of the same product). Never invent paths. Answer with JSON only."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly: {{\"groups\": [{{\"id\": \"<candidate id>\", \"name\": \"<folder name, no slashes>\", \"confidence\": 0.0-1.0}}], \"merge\": [[\"<id to fold>\", \"<id to keep>\"]]}}. Keep ids as given. Candidates:\n{}",
            serde_json::to_string_pretty(input).unwrap_or_default()
        )
    }

    fn parse(answer: &str) -> Result<TidyAnswer, AiError> {
        let v = extract_json(answer)?;
        let wire: Wire =
            serde_json::from_value(v).map_err(|e| AiError::BadAnswer(e.to_string()))?;
        let groups = wire
            .groups
            .into_iter()
            .map(|g| TidyName {
                id: g.id,
                name: g.name.trim().to_string(),
                confidence: g.confidence.clamp(0.0, 1.0) as f32,
            })
            .collect();
        let merge = wire
            .merge
            .into_iter()
            .filter_map(|pair| match pair.as_slice() {
                [a, b] => Some((a.clone(), b.clone())),
                _ => None,
            })
            .collect();
        Ok(TidyAnswer { groups, merge })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_merges() {
        let a = Tidy::parse(r#"Sure! {"groups":[{"id":"g1","name":" Project A ","confidence":0.9},{"id":"g2","name":"Invoices"}],"merge":[["g3","g1"],["bad"]]}"#).expect("parse");
        assert_eq!(
            a.groups[0],
            TidyName {
                id: "g1".into(),
                name: "Project A".into(),
                confidence: 0.9
            }
        );
        assert_eq!(
            a.groups[1].confidence, 0.5,
            "missing confidence defaults to 0.5"
        );
        assert_eq!(
            a.merge,
            vec![("g3".to_string(), "g1".to_string())],
            "malformed pairs dropped"
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(Tidy::parse(r#"{"groups":[],"merge":[],"delete":["g1"]}"#).is_err());
        assert!(Tidy::parse(r#"{"groups":[{"id":"g1","name":"x","path":"/etc"}]}"#).is_err());
    }

    #[test]
    fn prompt_carries_candidates_and_language() {
        let input = TidyInput {
            language: "id".into(),
            folder_name: "Documents".into(),
            candidates: vec![TidyCandidate {
                id: "g1".into(),
                name: "Invoice".into(),
                source: "prefix".into(),
                count: 3,
                examples: vec!["Invoice-1.pdf".into()],
            }],
        };
        let p = Tidy::user_prompt(&serde_json::to_value(&input).expect("json"));
        assert!(p.contains("Invoice-1.pdf"));
        assert!(p.contains("\"merge\""));
    }
}
