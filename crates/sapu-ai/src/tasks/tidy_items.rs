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

impl TidyItemsAnswer {
    /// The provider answers about the names it was sent, which may be
    /// anonymised ("form <user>.pdf"). Maps each answer back to the real
    /// item whose anonymised name it is, and restores the user name in
    /// suggested names, categories and reasons.
    pub fn restore_names(&mut self, originals: &[String], a: &crate::anonymize::Anonymizer) {
        for item in &mut self.items {
            if !originals.contains(&item.name)
                && let Some(o) = originals.iter().find(|o| a.text(o) == item.name)
            {
                item.name = o.clone();
            }
            item.rename = item.rename.as_deref().map(|r| a.restore(r));
            item.category = item.category.as_deref().map(|c| a.restore(c));
            item.reason = a.restore(&item.reason);
        }
        self.categories = self.categories.iter().map(|c| a.restore(c)).collect();
    }
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
3. Do not create a category for a single item unless it clearly stands apart. Every item should get a category; use null only when nothing fits at all.\n\
4. Category names are short, plain folder names in the person's language, with no slashes. When a file's name contains the name of an existing folder (for example 'My Saham.xlsx' and folder 'Saham'), give the file and that folder the same category: the file then goes inside that folder.\n\
5. People often save files with unclear names. Suggest a rename for every file whose name does not say what it is: camera or scanner names (scan0001.pdf, IMG_2015..., 'WhatsApp Image ...'), generic words ('form', 'foto', 'dokumen', 'Untitled', 'new'), typos, names that are only a person or an ID, 'final final(2)'. Use what the name, the year and the neighbouring items tell you. Keep it short and descriptive, keep the extension. Never rename folders. If you cannot guess a better name, give none.\n\
6. Mark junk=true for files that are probably not needed, and say why in 'reason': copies by name ('copy', '(1)', '- Copy'), an older version next to a newer one of the same document (the oldest CVs when several years exist), a .zip next to a folder with the same name (already extracted), a saved web page (.htm) next to the same document as PDF, schedules or announcements for a date long past, installers. These are suggestions the person checks, so propose them when it looks likely. Never mark folders as junk, and never the newest or only version of a document.\n\
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
        // Bolder about names and junk: a real folder gave zero suggestions.
        assert!(
            sys.contains("inside that folder"),
            "files named after a folder go into it"
        );
        assert!(
            sys.contains("older version"),
            "old versions next to a newer one are junk candidates"
        );
        assert!(
            sys.contains("unclear"),
            "unclear names get a suggestion, not only broken ones"
        );
        assert!(
            !sys.contains("very likely useless"),
            "the junk rule is no longer that strict"
        );
    }

    #[test]
    fn answers_about_anonymised_names_map_back_to_the_real_files() {
        let a = crate::anonymize::Anonymizer::with(None, Some("ramdani".into()), true);
        let mut answer = TidyItems::parse(
            r#"{"categories":["Identitas"],"items":[
                {"name":"form <user>.pdf","category":"Identitas","rename":"Formulir <user>.pdf","junk":false,"reason":"","confidence":0.8},
                {"name":"<user>-bpjs.jpeg","category":"Identitas","rename":null,"junk":false,"reason":"","confidence":0.9},
                {"name":"lain.pdf","category":null,"rename":null,"junk":false,"reason":"","confidence":0.5}
            ]}"#,
        )
        .expect("parse");
        let originals = vec![
            "form ramdani.pdf".to_string(),
            "ramdani-bpjs.jpeg".to_string(),
            "lain.pdf".to_string(),
        ];
        answer.restore_names(&originals, &a);
        assert_eq!(answer.items[0].name, "form ramdani.pdf");
        assert_eq!(
            answer.items[0].rename.as_deref(),
            Some("Formulir ramdani.pdf")
        );
        assert_eq!(answer.items[1].name, "ramdani-bpjs.jpeg");
        assert_eq!(answer.items[2].name, "lain.pdf");
    }
}
