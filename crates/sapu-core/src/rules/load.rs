//! Embedded rule files, filtered for the current OS.

use include_dir::{Dir, include_dir};

use super::schema::{Rule, RuleError, check_unique};

static RULES_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../rules");

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

impl RuleSet {
    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }
}

/// Parse every `.toml` under `common/` and `<os>/`. Fails loudly: a broken
/// shipped rule is a build problem, not a runtime surprise.
pub fn load_embedded(os: &str) -> Result<RuleSet, RuleError> {
    let mut rules = Vec::new();
    for sub in ["common", os] {
        if let Some(dir) = RULES_DIR.get_dir(sub) {
            for f in dir.files() {
                if f.path().extension().is_some_and(|e| e == "toml") {
                    let text = f.contents_utf8().unwrap_or("");
                    let rule = Rule::parse(&f.path().display().to_string(), text)?;
                    if rule.applies_to_os(os) {
                        rules.push(rule);
                    }
                }
            }
        }
    }
    rules.sort_by(|a, b| a.id.cmp(&b.id));
    check_unique(&rules)?;
    Ok(RuleSet { rules })
}

/// Parse rule files given as `(name, text)` pairs (community rules, tests).
pub fn parse_rules<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<RuleSet, RuleError> {
    let mut rules = Vec::new();
    for (name, text) in files {
        rules.push(Rule::parse(name, text)?);
    }
    check_unique(&rules)?;
    Ok(RuleSet { rules })
}

/// Every embedded rule for every OS directory, for tests.
#[cfg(test)]
pub(crate) fn load_all_embedded() -> Result<Vec<(String, Rule)>, RuleError> {
    let mut out = Vec::new();
    for dir in RULES_DIR.dirs() {
        for f in dir.files() {
            if f.path().extension().is_some_and(|e| e == "toml") {
                let name = f.path().display().to_string();
                let rule = Rule::parse(&name, f.contents_utf8().unwrap_or(""))?;
                out.push((name, rule));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Env;
    use crate::safety::ProtectedPaths;

    #[test]
    fn every_shipped_rule_parses_and_validates() {
        let all = load_all_embedded().expect("all rules valid");
        assert!(
            all.len() >= 20,
            "expected the initial rule set, got {}",
            all.len()
        );
        for os in ["macos", "linux", "windows"] {
            let set = load_embedded(os).expect("load");
            assert!(
                set.rules.iter().any(|r| r.group == "browser"),
                "{os} has a browser rule"
            );
            assert!(
                set.rules.iter().any(|r| r.command.is_some()),
                "{os} includes docker commands"
            );
        }
    }

    /// PRD Tahap 2 acceptance: no rule may point at a protected path.
    #[test]
    fn no_shipped_rule_targets_a_protected_path() {
        let protected = ProtectedPaths::os_defaults();
        let mut env = Env::with_home(dirs::home_dir().unwrap_or_else(|| "/home/sapu".into()));
        for (k, v) in [
            ("LOCALAPPDATA", "C:\\Users\\sapu\\AppData\\Local"),
            ("USERPROFILE", "C:\\Users\\sapu"),
        ] {
            env.vars.insert(k.into(), v.into());
        }
        for (file, rule) in load_all_embedded().expect("rules") {
            for m in &rule.matches {
                let templates: Vec<&str> = match m.kind {
                    MatchKind::DirName => m.roots.iter().map(String::as_str).collect(),
                    _ => m.path.iter().map(String::as_str).collect(),
                };
                for t in templates {
                    let Some(p) = env.expand(t) else { continue };
                    // Strip glob tails: the literal prefix is what gets touched.
                    let literal: std::path::PathBuf = p
                        .components()
                        .take_while(|c| !c.as_os_str().to_string_lossy().contains(['*', '?', '[']))
                        .collect();
                    if m.kind == MatchKind::DirName {
                        // Roots themselves are never removed, only named children.
                        continue;
                    }
                    let has_glob = p.to_string_lossy().contains(['*', '?', '[']);
                    // `dir`/`files` without a glob remove the path itself; every
                    // other form only removes children of the literal prefix.
                    let removes_self =
                        !has_glob && matches!(m.kind, MatchKind::Dir | MatchKind::Files);
                    let hit = protected.check(&literal);
                    let violation = match hit {
                        None => false,
                        Some(e) if removes_self => {
                            let _ = e;
                            true
                        }
                        Some(e) => e.protection == crate::safety::Protection::Subtree,
                    };
                    assert!(
                        !violation,
                        "{file}: `{t}` resolves to protected path {}",
                        literal.display()
                    );
                }
            }
        }
    }

    use crate::rules::MatchKind;
}
