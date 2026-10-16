//! A3: narrative report after a scan.

use super::{LANGUAGE_RULE, Task, extract_json, text};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportInput {
    pub language: String,
    pub root: String,
    pub total_bytes: u64,
    pub disk_total_bytes: Option<u64>,
    pub categories: Vec<(String, u64)>,
    pub top_dirs: Vec<(String, u64)>,
    pub delta_bytes_since_last: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReportAnswer {
    pub what_fills: String,
    pub start_here: String,
    pub leave_alone: String,
}

pub struct Report;

impl Task for Report {
    const NAME: &'static str = "report";
    type Input = ReportInput;
    type Output = ReportAnswer;

    fn system_prompt() -> &'static str {
        "You summarise a disk scan for a non-technical person in three short, concrete paragraphs. Name sizes in GB with one decimal. Never tell them to delete personal documents or photos; point at caches, old installers, build artefacts and large old videos instead, and always say that Sapu moves things to the Trash first."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly: {{\"what_fills\": \"what mainly fills the disk (2 sentences)\", \"start_here\": \"the safest first step (2 sentences)\", \"leave_alone\": \"what not to touch (1 sentence)\"}}\nScan:\n{input}"
        )
    }

    fn parse(answer: &str) -> Result<ReportAnswer, AiError> {
        let v = extract_json(answer)?;
        let r = ReportAnswer {
            what_fills: text(&v, "what_fills"),
            start_here: text(&v, "start_here"),
            leave_alone: text(&v, "leave_alone"),
        };
        if r.what_fills.is_empty() {
            return Err(AiError::BadAnswer("missing what_fills".into()));
        }
        Ok(r)
    }
}
