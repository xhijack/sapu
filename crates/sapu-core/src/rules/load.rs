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

/// `<config dir>/sapu/rules`: the user's own and community rules.
pub fn user_rules_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("sapu").join("rules"))
}

/// Embedded rules plus every `*.toml` in `user_dir` (if it exists). A
/// broken user rule is skipped with a warning rather than failing the set.
pub fn load_with_user_dir(
    os: &str,
    user_dir: Option<&std::path::Path>,
) -> Result<RuleSet, RuleError> {
    let mut set = load_embedded(os)?;
    let Some(dir) = user_dir else { return Ok(set) };
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(set);
    };
    let mut extra = Vec::new();
    for entry in rd.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let name = path.display().to_string();
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| Rule::parse(&name, &t).map_err(|e| e.to_string()))
        {
            Ok(rule) if rule.applies_to_os(os) => extra.push(rule),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "skipping user rule"),
        }
    }
    // Later files override embedded rules with the same id.
    for rule in extra {
        set.rules.retain(|r| r.id != rule.id);
        set.rules.push(rule);
    }
    set.rules.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(set)
}

/// Writes a user rule that marks the contents of `path` as removable with
/// a yellow label (AI answers never produce green, PRD AI-2). Returns the file written.
pub fn save_user_rule(
    user_dir: &std::path::Path,
    id: &str,
    name: (&str, &str),
    description: (&str, &str),
    path: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(user_dir)?;
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let text = format!(
        "[rule]\nid = \"{id}\"\nname = {{ id = \"{}\", en = \"{}\" }}\ndescription = {{ id = \"{}\", en = \"{}\" }}\nrisk = \"yellow\"\nprofile = \"general\"\ngroup = \"user\"\n\n[[match]]\npath = \"{}\"\nkind = \"dir_contents\"\n",
        esc(name.0),
        esc(name.1),
        esc(description.0),
        esc(description.1),
        esc(&path.to_string_lossy())
    );
    let file = user_dir.join(format!("{id}.toml"));
    Rule::parse(&file.display().to_string(), &text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(&file, text)?;
    Ok(file)
}

#[cfg(test)]
mod user_dir_tests {
    use super::*;

    #[test]
    fn user_rules_are_added_and_override_by_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("rules");
        let file = save_user_rule(
            &dir,
            "ai-spotify-cache",
            ("Cache Spotify", "Spotify cache"),
            ("Lagu offline diunduh ulang.", "Offline songs re-download."),
            std::path::Path::new("/tmp/x/Caches/com.spotify.client"),
        )
        .expect("save");
        assert!(file.exists());
        std::fs::write(dir.join("broken.toml"), b"not = [toml").expect("write");
        let set = load_with_user_dir("macos", Some(&dir)).expect("load");
        let r = set.get("ai-spotify-cache").expect("user rule present");
        assert_eq!(r.risk, Risk::Yellow);
        assert_eq!(r.group, "user");
        assert!(
            set.get("macos-browser-caches").is_some(),
            "embedded rules kept"
        );
    }

    use crate::rules::Risk;
}

/// Behavioural test for every shipped rule: build a fake home with the
/// rule's own paths, run the evaluator, and prove the rule finds exactly
/// the files under those paths and nothing else (PRD Tahap 2 acceptance).
#[cfg(test)]
mod per_rule_tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::cleaner::ProjectScanOptions;
    use crate::cleaner::evaluate_with;
    use crate::rules::{Env, MatchKind, Profile};
    use crate::safety::ProtectedPaths;

    /// Replace glob tokens with a concrete sample segment.
    fn concretize(template: &str) -> String {
        template
            .split('/')
            .map(|seg| {
                if seg.contains(['*', '?', '[']) {
                    seg.replace(['*', '?', '[', ']'], "x")
                } else {
                    seg.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn fake_env(root: &Path) -> Env {
        let home = root.join("home");
        let mut env = Env::with_home(&home);
        for (k, v) in [
            ("LOCALAPPDATA", "appdata/Local"),
            ("USERPROFILE", "home"),
            ("SystemDrive", "C"),
            ("ProgramData", "programdata"),
        ] {
            env.vars
                .insert(k.into(), root.join(v).to_string_lossy().into_owned());
        }
        env
    }

    fn write(p: &Path) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, b"sapu-test").expect("write");
        // Old enough for any min_age_days the rules use.
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(400 * 86_400);
        let _ = std::fs::File::options()
            .write(true)
            .open(p)
            .and_then(|f| f.set_modified(t));
    }

    #[test]
    fn every_match_finds_its_own_files_and_nothing_else() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let env = fake_env(tmp.path());
        let decoy = tmp.path().join("home/Documents/keep-me.pdf");
        write(&decoy);
        let protected = ProtectedPaths::os_defaults();
        let no_projects = ProjectScanOptions {
            roots: Vec::new(),
            ..Default::default()
        };

        let mut checked = 0;
        for (file, rule) in load_all_embedded().expect("rules") {
            let mut expected_files: Vec<PathBuf> = Vec::new();
            for m in &rule.matches {
                let Some(template) = m.path.as_deref() else {
                    continue;
                };
                let concrete = concretize(template);
                let Some(base) = env.expand(&concrete) else {
                    continue;
                };
                match m.kind {
                    MatchKind::DirContents => {
                        for n in ["a.bin", "sub/b.bin"] {
                            let p = base.join(n);
                            write(&p);
                        }
                        // Folders carry their own mtime; age the sample subfolder too.
                        let old = std::time::SystemTime::now()
                            - std::time::Duration::from_secs(400 * 86_400);
                        let _ =
                            std::fs::File::open(base.join("sub")).and_then(|f| f.set_modified(old));
                        expected_files.push(base.join("a.bin"));
                        expected_files.push(base.join("sub"));
                    }
                    MatchKind::Dir => {
                        write(&base.join("inner.bin"));
                        expected_files.push(base.clone());
                    }
                    MatchKind::Files => {
                        write(&base);
                        expected_files.push(base.clone());
                    }
                    MatchKind::DirName => {}
                }
            }
            if expected_files.is_empty() {
                continue;
            }
            let set = RuleSet {
                rules: vec![rule.clone()],
            };
            let report = evaluate_with(&set, Profile::Developer, &protected, &env, &no_projects);
            let found: Vec<PathBuf> = report
                .findings
                .iter()
                .filter(|f| f.rule_id == rule.id)
                .flat_map(|f| f.items.iter().map(|c| c.path.clone()))
                .collect();
            for e in &expected_files {
                assert!(
                    found.contains(e),
                    "{file}: expected {} to be found; found {found:?}",
                    e.display()
                );
            }
            assert!(
                !found.contains(&decoy),
                "{file}: must not touch files outside its paths"
            );
            for f in &found {
                assert!(
                    f.starts_with(tmp.path()),
                    "{file}: escaped the fake tree: {}",
                    f.display()
                );
            }
            checked += 1;
        }
        assert!(checked >= 20, "checked {checked} rules");
    }
}
