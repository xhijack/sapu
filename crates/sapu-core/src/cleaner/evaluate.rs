//! Turns rules into concrete, sized candidates on this machine.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::rules::{Command, Env, Match, MatchKind, Profile, Risk, Rule, RuleSet};
use crate::safety::ProtectedPaths;

use super::command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub mtime: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub risk: Risk,
    pub profile: Profile,
    pub group: String,
    pub permanent: bool,
    /// Scope for the safety plan: the directory the candidates live in.
    pub scope: PathBuf,
    pub items: Vec<Candidate>,
    pub bytes: u64,
    pub skipped_protected: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandFinding {
    pub rule_id: String,
    pub risk: Risk,
    pub profile: Profile,
    pub group: String,
    pub command_line: String,
    pub available: bool,
    pub confirm_separately: bool,
    pub estimate_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CleanReport {
    pub findings: Vec<Finding>,
    pub commands: Vec<CommandFinding>,
}

impl CleanReport {
    pub fn bytes_by_risk(&self, risk: Risk) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.risk == risk)
            .map(|f| f.bytes)
            .sum()
    }
}

/// Evaluate every rule for `profile`; developer profile includes general rules.
pub fn evaluate(
    rules: &RuleSet,
    profile: Profile,
    protected: &ProtectedPaths,
    env: &Env,
) -> CleanReport {
    let now = SystemTime::now();
    let mut report = CleanReport::default();
    for rule in &rules.rules {
        if rule.profile == Profile::Developer && profile == Profile::General {
            continue;
        }
        if let Some(cmd) = &rule.command {
            report.commands.push(command_finding(rule, cmd));
            continue;
        }
        for m in &rule.matches {
            let mut finding = Finding {
                rule_id: rule.id.clone(),
                risk: rule.risk,
                profile: rule.profile,
                group: rule.group.clone(),
                permanent: rule.permanent,
                scope: PathBuf::new(),
                items: Vec::new(),
                bytes: 0,
                skipped_protected: 0,
            };
            let (scope, raw) = candidates_for(m, env, now);
            finding.scope = scope;
            for c in raw {
                if protected.is_protected(&c.path) {
                    finding.skipped_protected += 1;
                    continue;
                }
                finding.bytes += c.size;
                finding.items.push(c);
            }
            if !finding.items.is_empty() {
                finding.items.sort_by_key(|c| std::cmp::Reverse(c.size));
                report.findings.push(finding);
            }
        }
    }
    report
}

fn command_finding(rule: &Rule, cmd: &Command) -> CommandFinding {
    let available = command::is_available(cmd);
    CommandFinding {
        rule_id: rule.id.clone(),
        risk: rule.risk,
        profile: rule.profile,
        group: rule.group.clone(),
        command_line: command::command_line(cmd),
        confirm_separately: cmd.confirm_separately,
        estimate_bytes: if available {
            command::estimate_bytes(cmd)
        } else {
            None
        },
        available,
    }
}

/// Returns (scope, candidates) for one match.
fn candidates_for(m: &Match, env: &Env, now: SystemTime) -> (PathBuf, Vec<Candidate>) {
    let min_age = Duration::from_secs(m.min_age_days * 86_400);
    let old_enough = |c: &Candidate| {
        m.min_age_days == 0
            || c.mtime
                .is_some_and(|t| now.duration_since(t).unwrap_or_default() >= min_age)
    };

    match m.kind {
        MatchKind::DirContents => {
            let Some(dir) = env.expand(m.path.as_deref().unwrap_or("")) else {
                return (PathBuf::new(), Vec::new());
            };
            let items = children(&dir).into_iter().filter(old_enough).collect();
            (dir, items)
        }
        MatchKind::Dir | MatchKind::Files => {
            let template = m.path.as_deref().unwrap_or("");
            let hits = env.expand_glob(template);
            let scope = hits
                .first()
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
                .unwrap_or_default();
            let items = hits
                .iter()
                .filter_map(|p| candidate(p))
                .filter(old_enough)
                .collect();
            (scope, items)
        }
        MatchKind::DirName => {
            let name = m.name.as_deref().unwrap_or("");
            let mut items = Vec::new();
            let mut scope = PathBuf::new();
            for root in &m.roots {
                let Some(root) = env.expand(root) else {
                    continue;
                };
                if scope.as_os_str().is_empty() {
                    scope = root.clone();
                }
                find_dirs_named(&root, name, m.max_depth, &m.skip, &mut items);
            }
            let unused = Duration::from_secs(m.unused_days * 86_400);
            let items = items
                .into_iter()
                .filter(|c| {
                    m.unused_days == 0
                        || project_untouched_for(&c.path).is_some_and(|age| age >= unused)
                })
                .collect();
            (scope, items)
        }
    }
}

fn candidate(p: &Path) -> Option<Candidate> {
    let meta = std::fs::symlink_metadata(p).ok()?;
    let is_dir = meta.is_dir() && !meta.file_type().is_symlink();
    Some(Candidate {
        path: p.to_path_buf(),
        size: if is_dir { dir_size(p) } else { meta.len() },
        is_dir,
        mtime: meta.modified().ok(),
    })
}

fn children(dir: &Path) -> Vec<Candidate> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.filter_map(Result::ok)
        .filter_map(|e| candidate(&e.path()))
        .collect()
}

/// Recursive size without following symlinks.
pub(crate) fn dir_size(dir: &Path) -> u64 {
    jwalk::WalkDir::new(dir)
        .follow_links(false)
        .skip_hidden(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| std::fs::symlink_metadata(e.path()).ok())
        .map(|m| m.len())
        .sum()
}

/// Bounded search for directories called `name`; never descends into a hit.
fn find_dirs_named(
    root: &Path,
    name: &str,
    max_depth: usize,
    skip: &[String],
    out: &mut Vec<Candidate>,
) {
    // The read_dir callback runs on worker threads and must own its data.
    let name_owned = name.to_string();
    let skip_owned: Vec<String> = skip.to_vec();
    let walker = jwalk::WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(false)
        .max_depth(max_depth)
        .process_read_dir(move |_depth, _path, _state, children| {
            for child in children.iter_mut().flatten() {
                if child.file_type().is_dir() {
                    let p = child.path();
                    let is_hit = p.file_name().is_some_and(|n| n == name_owned.as_str());
                    if is_hit || crate::rules::expand::is_skipped_pub(&p, &skip_owned) {
                        child.read_children_path = None;
                    }
                }
            }
        });
    for e in walker.into_iter().filter_map(Result::ok) {
        if e.depth() > 0 && e.file_type().is_dir() && e.file_name() == name {
            if let Some(c) = candidate(&e.path()) {
                out.push(c);
            }
        }
    }
}

/// Age of the newest file next to the artefact directory (the project's
/// own files), ignoring the artefact itself. `None` when unknown.
fn project_untouched_for(artefact: &Path) -> Option<Duration> {
    let project = artefact.parent()?;
    let mut newest: Option<SystemTime> = None;
    for e in std::fs::read_dir(project).ok()?.filter_map(Result::ok) {
        if e.path() == artefact {
            continue;
        }
        if let Ok(m) = e.metadata()
            && let Ok(t) = m.modified()
        {
            newest = Some(newest.map_or(t, |n| n.max(t)));
        }
    }
    SystemTime::now().duration_since(newest?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::parse_rules;

    fn write(p: &Path, n: usize) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(p, vec![b'x'; n]).expect("write");
    }

    fn age(p: &Path, days: u64) {
        let t = SystemTime::now() - Duration::from_secs(days * 86_400);
        std::fs::File::options()
            .write(true)
            .open(p)
            .ok()
            .map(|f| f.set_modified(t));
        if p.is_dir() {
            let _ = std::fs::File::open(p).and_then(|f| f.set_modified(t));
        }
    }

    const RULES: &str = r#"
[rule]
id = "cache"
name = { id = "Cache", en = "Cache" }
description = { id = "x", en = "x" }
risk = "green"
profile = "general"
group = "browser"
[[match]]
path = "~/Library/Caches/App"
kind = "dir_contents"
"#;

    #[test]
    fn dir_contents_lists_children_with_sizes_and_skips_protected() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join("Library/Caches/App/a.bin"), 100);
        write(&home.join("Library/Caches/App/sub/b.bin"), 50);
        write(&home.join("Library/Caches/App/secret/k"), 1);
        let rules = parse_rules([("r.toml", RULES)]).expect("rules");
        let mut protected = ProtectedPaths::os_defaults();
        protected.add(
            home.join("Library/Caches/App/secret"),
            crate::safety::Protection::Subtree,
            "test",
        );

        let report = evaluate(&rules, Profile::General, &protected, &Env::with_home(&home));
        assert_eq!(report.findings.len(), 1);
        let f = &report.findings[0];
        assert_eq!(f.scope, home.join("Library/Caches/App"));
        assert_eq!(f.items.len(), 2);
        assert_eq!(f.bytes, 150);
        assert_eq!(f.skipped_protected, 1);
        assert_eq!(report.bytes_by_risk(Risk::Green), 150);
    }

    #[test]
    fn files_glob_with_min_age() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join("Downloads/old.dmg"), 10);
        write(&home.join("Downloads/new.dmg"), 10);
        write(&home.join("Downloads/doc.pdf"), 10);
        age(&home.join("Downloads/old.dmg"), 60);
        let rules = parse_rules([(
            "r.toml",
            r#"
[rule]
id = "installers"
name = { id = "x", en = "x" }
description = { id = "x", en = "x" }
risk = "yellow"
profile = "general"
group = "installers"
[[match]]
path = "~/Downloads/*.dmg"
kind = "files"
min_age_days = 30
"#,
        )])
        .expect("rules");
        let report = evaluate(
            &rules,
            Profile::General,
            &ProtectedPaths::os_defaults(),
            &Env::with_home(&home),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].items.len(), 1);
        assert!(report.findings[0].items[0].path.ends_with("old.dmg"));
    }

    #[test]
    fn dir_name_finds_node_modules_once_and_respects_unused_days() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join("dev/old-proj/package.json"), 10);
        write(&home.join("dev/old-proj/node_modules/x/index.js"), 1000);
        write(
            &home.join("dev/old-proj/node_modules/x/node_modules/y/i.js"),
            5,
        );
        write(&home.join("dev/active/package.json"), 10);
        write(&home.join("dev/active/node_modules/z/i.js"), 500);
        write(&home.join("Library/x/node_modules/skip.js"), 9);
        age(&home.join("dev/old-proj/package.json"), 90);
        age(&home.join("dev/old-proj"), 90);

        let rules = parse_rules([(
            "r.toml",
            r#"
[rule]
id = "node-modules"
name = { id = "x", en = "x" }
description = { id = "x", en = "x" }
risk = "yellow"
profile = "developer"
group = "node_modules"
[[match]]
kind = "dir_name"
name = "node_modules"
roots = ["~"]
max_depth = 5
unused_days = 30
skip = ["Library", ".*"]
"#,
        )])
        .expect("rules");
        let general = evaluate(
            &rules,
            Profile::General,
            &ProtectedPaths::os_defaults(),
            &Env::with_home(&home),
        );
        assert!(
            general.findings.is_empty(),
            "developer rules hidden from general profile"
        );

        let report = evaluate(
            &rules,
            Profile::Developer,
            &ProtectedPaths::os_defaults(),
            &Env::with_home(&home),
        );
        assert_eq!(report.findings.len(), 1);
        let items = &report.findings[0].items;
        assert_eq!(
            items.len(),
            1,
            "only the untouched project, nested node_modules not listed twice: {items:?}"
        );
        assert!(items[0].path.ends_with("old-proj/node_modules"));
        assert_eq!(items[0].size, 1005);
    }

    #[test]
    fn unknown_variable_yields_no_finding() {
        let rules = parse_rules([(
            "r.toml",
            r#"
[rule]
id = "win"
name = { id = "x", en = "x" }
description = { id = "x", en = "x" }
risk = "green"
profile = "general"
group = "temp"
[[match]]
path = "${LOCALAPPDATA_SAPU_NOPE}/Temp"
kind = "dir_contents"
"#,
        )])
        .expect("rules");
        let report = evaluate(
            &rules,
            Profile::General,
            &ProtectedPaths::os_defaults(),
            &Env::with_home("/nonexistent"),
        );
        assert!(report.findings.is_empty());
    }
}
