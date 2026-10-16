//! Rule file schema and validation.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Localized {
    pub id: String,
    pub en: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Green,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    General,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    /// Remove the children of the directory, keep the directory.
    DirContents,
    /// Remove the directory itself.
    Dir,
    /// Remove files matched by a glob.
    Files,
    /// Find directories by name under `roots`, bounded by `max_depth`.
    DirName,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Match {
    pub path: Option<String>,
    pub kind: MatchKind,
    pub min_age_days: u64,
    // dir_name only
    pub name: Option<String>,
    pub roots: Vec<String>,
    pub max_depth: usize,
    pub unused_days: u64,
    pub skip: Vec<String>,
}

impl Default for Match {
    fn default() -> Self {
        Self {
            path: None,
            kind: MatchKind::DirContents,
            min_age_days: 0,
            name: None,
            roots: Vec::new(),
            max_depth: 6,
            unused_days: 0,
            skip: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub requires: Option<String>,
    pub estimate: Vec<String>,
    /// Only count estimate output lines containing this text (e.g. "Images").
    pub estimate_filter: Option<String>,
    pub confirm_separately: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct RuleHeader {
    id: String,
    name: Localized,
    description: Localized,
    risk: Risk,
    profile: Profile,
    group: String,
    #[serde(default)]
    permanent: bool,
    #[serde(default)]
    os: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct RuleFile {
    rule: RuleHeader,
    #[serde(default, rename = "match")]
    matches: Vec<Match>,
    command: Option<Command>,
}

/// A validated rule.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rule {
    pub id: String,
    pub name: Localized,
    pub description: Localized,
    pub risk: Risk,
    pub profile: Profile,
    pub group: String,
    pub permanent: bool,
    pub os: Vec<String>,
    pub matches: Vec<Match>,
    pub command: Option<Command>,
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("{file}: {message}")]
    Invalid { file: String, message: String },
    #[error("{file}: {source}")]
    Parse {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("duplicate rule id `{0}`")]
    Duplicate(String),
}

impl Rule {
    pub(super) fn parse(file: &str, text: &str) -> Result<Self, RuleError> {
        let parsed: RuleFile = toml::from_str(text).map_err(|source| RuleError::Parse {
            file: file.to_string(),
            source,
        })?;
        let rule = Self {
            id: parsed.rule.id,
            name: parsed.rule.name,
            description: parsed.rule.description,
            risk: parsed.rule.risk,
            profile: parsed.rule.profile,
            group: parsed.rule.group,
            permanent: parsed.rule.permanent,
            os: parsed.rule.os,
            matches: parsed.matches,
            command: parsed.command,
        };
        rule.validate().map_err(|message| RuleError::Invalid {
            file: file.to_string(),
            message,
        })?;
        Ok(rule)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!("id `{}` must be kebab-case", self.id));
        }
        for (what, l) in [("name", &self.name), ("description", &self.description)] {
            if l.id.trim().is_empty() || l.en.trim().is_empty() {
                return Err(format!("{what} must be given in both id and en"));
            }
        }
        if self.group.is_empty() {
            return Err("group is required".into());
        }
        match (&self.command, self.matches.is_empty()) {
            (Some(_), false) => {
                return Err("a rule has either [[match]] or [command], not both".into());
            }
            (None, true) => return Err("a rule needs at least one [[match]] or a [command]".into()),
            (Some(c), true) if c.program.is_empty() => {
                return Err("command.program is required".into());
            }
            _ => {}
        }
        if self.permanent && self.risk == Risk::Green {
            return Err("permanent rules must be yellow".into());
        }
        for (i, m) in self.matches.iter().enumerate() {
            match m.kind {
                MatchKind::DirName => {
                    if m.name.as_deref().unwrap_or("").is_empty() || m.roots.is_empty() {
                        return Err(format!("match[{i}]: dir_name needs `name` and `roots`"));
                    }
                }
                _ => {
                    if m.path.as_deref().unwrap_or("").is_empty() {
                        return Err(format!("match[{i}]: `path` is required"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether the rule applies to the current OS.
    pub fn applies_to_os(&self, os: &str) -> bool {
        self.os.is_empty() || self.os.iter().any(|o| o == os)
    }
}

pub(super) fn check_unique(rules: &[Rule]) -> Result<(), RuleError> {
    let mut seen = HashSet::new();
    for r in rules {
        if !seen.insert(r.id.as_str()) {
            return Err(RuleError::Duplicate(r.id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"
[rule]
id = "chrome-cache"
name = { id = "Cache Chrome", en = "Chrome cache" }
description = { id = "Dibuat ulang otomatis.", en = "Recreated automatically." }
risk = "green"
profile = "general"
group = "browser"

[[match]]
path = "~/Library/Caches/Google/Chrome"
kind = "dir_contents"
"#;

    #[test]
    fn parses_a_valid_rule() {
        let r = Rule::parse("ok.toml", OK).expect("parse");
        assert_eq!(r.id, "chrome-cache");
        assert_eq!(r.matches[0].kind, MatchKind::DirContents);
        assert!(r.applies_to_os("macos"));
    }

    #[test]
    fn rejects_missing_translation() {
        let bad = OK.replace(r#"en = "Chrome cache""#, r#"en = """#);
        assert!(Rule::parse("bad.toml", &bad).is_err());
    }

    #[test]
    fn rejects_match_and_command_together() {
        let bad = format!("{OK}\n[command]\nprogram = \"docker\"\n");
        assert!(Rule::parse("bad.toml", &bad).is_err());
    }

    #[test]
    fn rejects_green_permanent() {
        let bad = OK.replace(
            "group = \"browser\"",
            "group = \"browser\"\npermanent = true",
        );
        assert!(Rule::parse("bad.toml", &bad).is_err());
    }

    #[test]
    fn rejects_bad_id() {
        let bad = OK.replace("chrome-cache", "Chrome Cache");
        assert!(Rule::parse("bad.toml", &bad).is_err());
    }
}
