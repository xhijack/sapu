//! Tidy, per item: for every entry of a folder, a destination category,
//! optionally a better file name and a junk flag. Sees names only: file
//! names, and the names inside folders. Never file contents.

use super::{LANGUAGE_RULE, Task, extract_json};
use crate::AiError;

/// One entry of the folder, described by names and metadata only.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyItem {
    pub name: String,
    /// file | folder | project | bundle (an app)
    pub unit: String,
    pub kind: String,
    pub year: Option<i32>,
    pub size_kb: u64,
    /// For folders: up to eight names inside it.
    pub children: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyItemsInput {
    pub language: String,
    pub folder_name: String,
    pub items: Vec<TidyItem>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyItemAdvice {
    pub name: String,
    pub category: Option<String>,
    pub rename: Option<String>,
    pub junk: bool,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyItemsAnswer {
    pub categories: Vec<String>,
    pub items: Vec<TidyItemAdvice>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(default)]
    categories: Vec<WireCategory>,
    items: Vec<WireItem>,
}

/// `"Keuangan"` or `{"name": "Keuangan"}`; models use both.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum WireCategory {
    Name(String),
    Object(WireCategoryObject),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCategoryObject {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireItem {
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    rename: Option<String>,
    #[serde(default)]
    junk: bool,
    #[serde(default)]
    reason: String,
    #[serde(default = "half")]
    confidence: f64,
}

fn half() -> f64 {
    0.5
}

pub struct TidyItems;

impl Task for TidyItems {
    const NAME: &'static str = "tidy_items";
    type Input = TidyItemsInput;
    type Output = TidyItemsAnswer;

    fn system_prompt() -> &'static str {
        "You tidy one messy folder on a personal computer. You never see file contents, only names: the file names in the folder and, for each folder inside it, the names it contains. \
Decide for EVERY item where it belongs:\n\
1. Group by purpose or topic (for example finance, work, identity documents, business, worship, letters, study, photos of family), never by file type. A folder of mixed PDFs and spreadsheets about money is 'Finance', not 'Documents'.\n\
2. Make between 4 and 8 categories in total. Reuse an existing folder name as the category when it already fits; an existing folder can also go into a broader category (for example folders 'Saham' and 'Rekening Koran' into 'Keuangan').\n\
3. Do not create a category for a single item unless it clearly stands apart. Leave an item where it is (category null) only when nothing fits.\n\
4. Category names are short, plain folder names in the person's language, with no slashes.\n\
5. Suggest a rename only for files whose name is meaningless or broken (scan0001.pdf, IMG_2015..., 'Untitled', 'final final(2)'). Keep it short and descriptive, keep the extension. Never rename folders.\n\
6. Mark junk=true only for files that are very likely useless: temporary exports, obvious duplicates by name such as 'copy' or '(1)', empty-looking or meaningless leftovers. Explain why in 'reason'. When unsure, junk=false. Never mark folders as junk.\n\
7. confidence is 0.0 to 1.0 for the category choice. Answer with JSON only."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly: {{\"categories\": [{{\"name\": \"<folder name>\"}}], \"items\": [{{\"name\": \"<exact item name>\", \"category\": \"<category or null>\", \"rename\": \"<new file name or null>\", \"junk\": false, \"reason\": \"<short, only for rename or junk>\", \"confidence\": 0.0}}]}}. One entry per item, names copied exactly. Folder and items:\n{}",
            serde_json::to_string_pretty(input).unwrap_or_default()
        )
    }

    fn parse(answer: &str) -> Result<TidyItemsAnswer, AiError> {
        let v = extract_json(answer)?;
        let wire: Wire =
            serde_json::from_value(v).map_err(|e| AiError::BadAnswer(e.to_string()))?;
        Ok(TidyItemsAnswer {
            categories: wire
                .categories
                .into_iter()
                .map(|c| match c {
                    WireCategory::Name(n) => n.trim().to_string(),
                    WireCategory::Object(o) => o.name.trim().to_string(),
                })
                .collect(),
            items: wire
                .items
                .into_iter()
                .map(|i| TidyItemAdvice {
                    name: i.name,
                    category: i
                        .category
                        .map(|c| c.trim().to_string())
                        .filter(|c| !c.is_empty()),
                    rename: i
                        .rename
                        .map(|r| r.trim().to_string())
                        .filter(|r| !r.is_empty()),
                    junk: i.junk,
                    reason: i.reason.trim().to_string(),
                    confidence: i.confidence.clamp(0.0, 1.0) as f32,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TidyItemsInput {
        TidyItemsInput {
            language: "id".into(),
            folder_name: "Pribadi".into(),
            items: vec![
                TidyItem {
                    name: "My Saham.xlsx".into(),
                    unit: "file".into(),
                    kind: "spreadsheet".into(),
                    year: Some(2021),
                    size_kb: 12,
                    children: vec![],
                },
                TidyItem {
                    name: "Saham".into(),
                    unit: "folder".into(),
                    kind: "folder".into(),
                    year: None,
                    size_kb: 12,
                    children: vec!["analis .xlsx".into()],
                },
            ],
        }
    }

    #[test]
    fn parses_categories_and_items() {
        let a = TidyItems::parse(r#"Here: {"categories":[{"name":"Keuangan"}],"items":[{"name":"My Saham.xlsx","category":"Saham","rename":null,"junk":false,"reason":"","confidence":0.9},{"name":"Saham","category":"Keuangan","confidence":0.7}]}"#).expect("parse");
        assert_eq!(a.categories, vec!["Keuangan".to_string()]);
        let b = TidyItems::parse(r#"{"categories":["Pekerjaan","Ibadah"],"items":[]}"#)
            .expect("strings");
        assert_eq!(
            b.categories,
            vec!["Pekerjaan".to_string(), "Ibadah".to_string()]
        );
        assert_eq!(a.items[0].category.as_deref(), Some("Saham"));
        assert!(!a.items[1].junk, "junk defaults to false");
        assert_eq!(a.items[1].confidence, 0.7);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(
            TidyItems::parse(r#"{"categories":[],"items":[{"name":"a","delete":true}]}"#).is_err()
        );
        assert!(TidyItems::parse(r#"{"categories":[],"items":[],"commands":["rm -rf"]}"#).is_err());
    }

    #[test]
    fn prompt_carries_names_children_and_the_rules() {
        let p = TidyItems::user_prompt(&serde_json::to_value(input()).expect("json"));
        assert!(p.contains("analis .xlsx"), "folder children are visible");
        assert!(p.contains("My Saham.xlsx"));
        let sys = TidyItems::system_prompt();
        assert!(
            sys.contains("purpose")
                && sys.contains("existing folder")
                && sys.contains("never see file contents")
        );
    }
}
