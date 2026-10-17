//! Dead development projects (PRD Tahap 2 P1, the deterministic half of
//! §12.6): find project roots, their build artefacts, whether those are
//! ignored by git and untracked, how long the project has been idle, and
//! how to rebuild. Folders that contain git-tracked files are never
//! proposed.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::rules::{Env, Risk};
use crate::safety::ProtectedPaths;

use super::evaluate::dir_size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Node,
    Rust,
    Python,
    Go,
    Dart,
    Gradle,
    Maven,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Artefact {
    pub path: PathBuf,
    /// Folder name, e.g. `node_modules`, `.next`, `target`.
    pub kind: String,
    pub bytes: u64,
    pub gitignored: bool,
    /// `Some(true)` when git tracks files inside; `None` when unknown (no git).
    pub tracked: Option<bool>,
    pub risk: Risk,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
    pub ecosystem: Ecosystem,
    pub last_activity: Option<SystemTime>,
    pub idle_days: Option<u64>,
    pub artefacts: Vec<Artefact>,
    pub bytes: u64,
    /// Commands that recreate the artefacts, in order.
    pub rebuild: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectScanOptions {
    pub roots: Vec<String>,
    pub max_depth: usize,
    pub skip: Vec<String>,
    /// Idle for at least this long → green.
    pub stale_days: u64,
    /// Artefacts smaller than this are not worth listing.
    pub min_bytes: u64,
}

impl Default for ProjectScanOptions {
    fn default() -> Self {
        Self {
            roots: vec!["~".into()],
            max_depth: 6,
            skip: vec![
                "Library".into(),
                ".*".into(),
                "Applications".into(),
                "Music".into(),
                "Movies".into(),
                "Pictures".into(),
                "AppData".into(),
                "node_modules".into(),
                "target".into(),
                "vendor".into(),
            ],
            stale_days: 30,
            min_bytes: 1024 * 1024,
        }
    }
}

const MARKERS: &[(&str, Ecosystem)] = &[
    ("package.json", Ecosystem::Node),
    ("Cargo.toml", Ecosystem::Rust),
    ("pyproject.toml", Ecosystem::Python),
    ("requirements.txt", Ecosystem::Python),
    ("go.mod", Ecosystem::Go),
    ("pubspec.yaml", Ecosystem::Dart),
    ("build.gradle", Ecosystem::Gradle),
    ("build.gradle.kts", Ecosystem::Gradle),
    ("pom.xml", Ecosystem::Maven),
];

/// Does `dir` hold a project marker (`package.json`, `Cargo.toml`, …) or a
/// git checkout? Such folders are treated as one unit by other features.
pub fn is_project_root(dir: &Path) -> bool {
    dir.join(".git").exists() || MARKERS.iter().any(|(m, _)| dir.join(m).is_file())
}

/// Every folder name that can be a build artefact, used to avoid descending
/// into them and when measuring project activity.
const ALL_ARTEFACTS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    ".cache",
    "coverage",
    "storybook-static",
    ".angular",
    ".astro",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".gradle",
    ".dart_tool",
    "bin",
    "obj",
];

pub fn find_projects(
    opts: &ProjectScanOptions,
    env: &Env,
    protected: &ProtectedPaths,
) -> Vec<Project> {
    let mut out = Vec::new();
    let git = git_available();
    for root in &opts.roots {
        let Some(root) = env.expand(root) else {
            continue;
        };
        for dir in project_dirs(&root, opts) {
            if let Some(p) = inspect_project(&dir, opts, git, protected) {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.root.cmp(&b.root)));
    out
}

fn project_dirs(root: &Path, opts: &ProjectScanOptions) -> Vec<PathBuf> {
    let skip = opts.skip.clone();
    let walker = jwalk::WalkDir::new(root)
        .follow_links(false)
        .parallelism(crate::platform::walk_parallelism())
        .skip_hidden(false)
        .max_depth(opts.max_depth)
        .process_read_dir(move |_d, _p, _s, children| {
            for child in children.iter_mut().flatten() {
                if child.file_type().is_dir() {
                    let p = child.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if ALL_ARTEFACTS.contains(&name)
                        || crate::rules::expand::is_skipped_pub(&p, &skip)
                    {
                        child.read_children_path = None;
                    }
                }
            }
        });
    let mut dirs = Vec::new();
    for e in walker.into_iter().filter_map(Result::ok) {
        if e.file_type().is_file()
            && let Some(name) = e.file_name().to_str()
            && MARKERS.iter().any(|(m, _)| *m == name)
            && let Some(parent) = e.path().parent()
        {
            let parent = parent.to_path_buf();
            if !dirs.contains(&parent) {
                dirs.push(parent);
            }
        }
    }
    dirs
}

fn inspect_project(
    dir: &Path,
    opts: &ProjectScanOptions,
    git: bool,
    protected: &ProtectedPaths,
) -> Option<Project> {
    let ecosystem = MARKERS
        .iter()
        .find(|(m, _)| dir.join(m).exists())
        .map(|(_, e)| *e)?;
    let package_json = (ecosystem == Ecosystem::Node)
        .then(|| read_package_json(dir))
        .flatten();
    let expected = expected_artefacts(dir, ecosystem, package_json.as_ref());
    let ignore = GitIgnore::load(dir);
    let in_git = dir.join(".git").exists();
    let last_activity = last_activity(dir, in_git);
    let idle_days = last_activity
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() / 86_400);

    let mut artefacts = Vec::new();
    for kind in expected {
        let path = dir.join(kind);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_dir() || meta.file_type().is_symlink() || protected.is_protected(&path) {
            continue;
        }
        let tracked = if in_git && git {
            Some(git_tracks(dir, kind))
        } else {
            None
        };
        if tracked == Some(true) {
            continue; // PRD: never propose folders with tracked files
        }
        let bytes = dir_size(&path);
        if bytes < opts.min_bytes {
            continue;
        }
        let gitignored = ignore.matches(kind);
        let risk = if gitignored
            && tracked == Some(false)
            && idle_days.is_some_and(|d| d >= opts.stale_days)
        {
            Risk::Green
        } else {
            Risk::Yellow
        };
        artefacts.push(Artefact {
            path,
            kind: kind.to_string(),
            bytes,
            gitignored,
            tracked,
            risk,
        });
    }
    if artefacts.is_empty() {
        return None;
    }
    artefacts.sort_by_key(|a| std::cmp::Reverse(a.bytes));
    let bytes = artefacts.iter().map(|a| a.bytes).sum();
    Some(Project {
        name: package_json
            .as_ref()
            .and_then(|p| p.name.clone())
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            }),
        rebuild: rebuild_commands(dir, ecosystem, package_json.as_ref()),
        root: dir.to_path_buf(),
        ecosystem,
        last_activity,
        idle_days,
        artefacts,
        bytes,
    })
}

#[derive(Debug, Default)]
struct PackageJson {
    name: Option<String>,
    deps: HashSet<String>,
    scripts: HashSet<String>,
}

fn read_package_json(dir: &Path) -> Option<PackageJson> {
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("package.json")).ok()?).ok()?;
    let mut p = PackageJson {
        name: v.get("name").and_then(|n| n.as_str()).map(str::to_string),
        ..Default::default()
    };
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = v.get(key).and_then(|d| d.as_object()) {
            p.deps.extend(obj.keys().cloned());
        }
    }
    if let Some(obj) = v.get("scripts").and_then(|d| d.as_object()) {
        p.scripts.extend(obj.keys().cloned());
    }
    Some(p)
}

/// Artefact folders this ecosystem/framework is known to generate.
fn expected_artefacts(dir: &Path, eco: Ecosystem, pkg: Option<&PackageJson>) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = match eco {
        Ecosystem::Node => {
            let mut v = vec![
                "node_modules",
                "coverage",
                ".turbo",
                ".parcel-cache",
                ".cache",
            ];
            let has = |d: &str| pkg.is_some_and(|p| p.deps.contains(d));
            if has("next") {
                v.push(".next");
            }
            if has("nuxt") {
                v.extend([".nuxt", ".output"]);
            }
            if has("@sveltejs/kit") {
                v.push(".svelte-kit");
            }
            if has("astro") || has("vite") || has("@angular/core") || has("parcel") {
                v.push("dist");
            }
            if has("react-scripts") || has("gatsby") {
                v.push("build");
            }
            if has("@angular/core") {
                v.push(".angular");
            }
            if has("storybook") || has("@storybook/react") {
                v.push("storybook-static");
            }
            if has("typescript") && !v.contains(&"dist") {
                // tsc outDir is commonly dist/ or build/; only when gitignored (checked later).
                v.extend(["dist", "build", "out"]);
            }
            v
        }
        Ecosystem::Rust => vec!["target"],
        Ecosystem::Python => vec![
            ".venv",
            "venv",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".tox",
            "build",
            "dist",
        ],
        Ecosystem::Go => vec!["bin"],
        Ecosystem::Dart => vec!["build", ".dart_tool"],
        Ecosystem::Gradle => vec!["build", ".gradle"],
        Ecosystem::Maven => vec!["target"],
    };
    v.dedup();
    let _ = dir;
    v
}

fn rebuild_commands(dir: &Path, eco: Ecosystem, pkg: Option<&PackageJson>) -> Vec<String> {
    match eco {
        Ecosystem::Node => {
            let pm = if dir.join("pnpm-lock.yaml").exists() {
                "pnpm"
            } else if dir.join("yarn.lock").exists() {
                "yarn"
            } else if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
                "bun"
            } else {
                "npm"
            };
            let mut v = vec![if pm == "yarn" {
                "yarn".to_string()
            } else {
                format!("{pm} install")
            }];
            if pkg.is_some_and(|p| p.scripts.contains("build")) {
                v.push(if pm == "npm" {
                    "npm run build".into()
                } else {
                    format!("{pm} run build")
                });
            }
            v
        }
        Ecosystem::Rust => vec!["cargo build".into()],
        Ecosystem::Python => {
            if dir.join("poetry.lock").exists() {
                vec!["poetry install".into()]
            } else if dir.join("uv.lock").exists() {
                vec!["uv sync".into()]
            } else if dir.join("requirements.txt").exists() {
                vec![
                    "python -m venv .venv".into(),
                    "pip install -r requirements.txt".into(),
                ]
            } else {
                vec!["pip install -e .".into()]
            }
        }
        Ecosystem::Go => vec!["go build ./...".into()],
        Ecosystem::Dart => vec![if dir.join("pubspec.yaml").exists()
            && std::fs::read_to_string(dir.join("pubspec.yaml"))
                .is_ok_and(|s| s.contains("flutter"))
        {
            "flutter pub get".into()
        } else {
            "dart pub get".into()
        }],
        Ecosystem::Gradle => vec![if dir.join("gradlew").exists() {
            "./gradlew build".into()
        } else {
            "gradle build".into()
        }],
        Ecosystem::Maven => vec!["mvn package".into()],
    }
}

/// Newest modification time among the project's own files (artefact and
/// hidden folders excluded), plus `.git/index` when present. Bounded.
fn last_activity(dir: &Path, in_git: bool) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    let mut bump = |t: SystemTime| newest = Some(newest.map_or(t, |n| n.max(t)));
    if in_git
        && let Ok(m) = std::fs::metadata(dir.join(".git/index"))
        && let Ok(t) = m.modified()
    {
        bump(t);
    }
    let walker = jwalk::WalkDir::new(dir)
        .follow_links(false)
        .parallelism(crate::platform::walk_parallelism())
        .skip_hidden(true)
        .max_depth(3)
        .process_read_dir(|_d, _p, _s, children| {
            for child in children.iter_mut().flatten() {
                if child.file_type().is_dir() {
                    let name = child.file_name().to_str().unwrap_or("");
                    if ALL_ARTEFACTS.contains(&name) {
                        child.read_children_path = None;
                    }
                }
            }
        });
    for e in walker
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .take(2000)
    {
        if let Ok(m) = e.metadata()
            && let Ok(t) = m.modified()
        {
            bump(t);
        }
    }
    newest
}

/// Minimal `.gitignore` reader: enough to answer "is this top-level folder ignored?".
struct GitIgnore {
    patterns: Vec<String>,
}

impl GitIgnore {
    fn load(dir: &Path) -> Self {
        let patterns = std::fs::read_to_string(dir.join(".gitignore"))
            .map(|s| {
                s.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!'))
                    .map(|l| l.trim_start_matches('/').trim_end_matches('/').to_string())
                    .collect()
            })
            .unwrap_or_default();
        Self { patterns }
    }

    fn matches(&self, name: &str) -> bool {
        self.patterns.iter().any(|p| {
            p == name
                || p == "*"
                || (p.starts_with("**/") && &p[3..] == name)
                || glob::Pattern::new(p).is_ok_and(|g| g.matches(name))
        })
    }
}

fn git_available() -> bool {
    std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p).any(|d| d.join("git").is_file() || d.join("git.exe").is_file())
    })
}

/// True when git tracks at least one file under `name` in `dir`.
fn git_tracks(dir: &Path, name: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "--", name])
        .stdin(std::process::Stdio::null())
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn write(p: &Path, bytes: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(p, bytes).expect("write");
    }

    fn age(p: &Path, days: u64) {
        let t = SystemTime::now() - Duration::from_secs(days * 86_400);
        let _ = std::fs::File::options()
            .write(true)
            .open(p)
            .and_then(|f| f.set_modified(t));
    }

    fn opts() -> ProjectScanOptions {
        ProjectScanOptions {
            min_bytes: 1,
            ..Default::default()
        }
    }

    #[test]
    fn finds_stale_next_project_with_ignored_artefacts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let proj = home.join("dev/landing");
        write(&proj.join("package.json"), br#"{"name":"landing-page","dependencies":{"next":"14"},"scripts":{"build":"next build"}}"#);
        write(&proj.join("pnpm-lock.yaml"), b"lock");
        write(&proj.join(".gitignore"), b"# deps\nnode_modules/\n/.next\n");
        write(&proj.join("src/index.ts"), b"export {}");
        write(&proj.join("node_modules/x/index.js"), &[1u8; 5000]);
        write(&proj.join(".next/cache/a"), &[1u8; 3000]);
        write(&proj.join("dist/tracked.js"), &[1u8; 100]); // not ignored, not a known output
        for f in [
            "package.json",
            "pnpm-lock.yaml",
            ".gitignore",
            "src/index.ts",
        ] {
            age(&proj.join(f), 200);
        }

        let projects = find_projects(
            &opts(),
            &Env::with_home(&home),
            &ProtectedPaths::os_defaults(),
        );
        assert_eq!(projects.len(), 1, "{projects:?}");
        let p = &projects[0];
        assert_eq!(p.name, "landing-page");
        assert_eq!(p.ecosystem, Ecosystem::Node);
        assert!(p.idle_days.is_some_and(|d| d >= 199), "{:?}", p.idle_days);
        assert_eq!(p.rebuild, vec!["pnpm install", "pnpm run build"]);
        let kinds: Vec<&str> = p.artefacts.iter().map(|a| a.kind.as_str()).collect();
        assert!(
            kinds.contains(&"node_modules") && kinds.contains(&".next"),
            "{kinds:?}"
        );
        let nm = p
            .artefacts
            .iter()
            .find(|a| a.kind == "node_modules")
            .expect("nm");
        assert!(nm.gitignored);
        assert_eq!(nm.tracked, None, "no .git → unknown");
        assert_eq!(nm.risk, Risk::Yellow, "unknown tracking stays yellow");
        // `dist` is not a known Next.js output, so it is never proposed.
        assert!(p.artefacts.iter().all(|a| a.kind != "dist"));
    }

    #[test]
    fn git_tracked_folder_is_never_proposed_and_ignored_stale_is_green() {
        if !git_available() {
            return;
        }
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let proj = home.join("code/tool");
        write(
            &proj.join("Cargo.toml"),
            b"[package]\nname = \"tool\"\nversion = \"0.1.0\"\n",
        );
        write(&proj.join("src/main.rs"), b"fn main() {}");
        write(&proj.join(".gitignore"), b"/target\n");
        write(&proj.join("target/debug/tool"), &[1u8; 4000]);
        write(&proj.join("build/keep.sh"), b"#!/bin/sh\n"); // hand-written, tracked below
        let git = |args: &[&str]| {
            let st = std::process::Command::new("git")
                .arg("-C")
                .arg(&proj)
                .args(args)
                .output()
                .expect("git");
            assert!(
                st.status.success(),
                "{}",
                String::from_utf8_lossy(&st.stderr)
            );
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "add",
            "Cargo.toml",
            "src",
            ".gitignore",
            "build/keep.sh",
        ]);
        git(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "init",
        ]);
        for f in ["Cargo.toml", "src/main.rs", ".gitignore", ".git/index"] {
            age(&proj.join(f), 90);
        }

        let projects = find_projects(
            &opts(),
            &Env::with_home(&home),
            &ProtectedPaths::os_defaults(),
        );
        assert_eq!(projects.len(), 1, "{projects:?}");
        let p = &projects[0];
        assert_eq!(p.ecosystem, Ecosystem::Rust);
        assert_eq!(
            p.artefacts.len(),
            1,
            "build/ has tracked files and must not appear: {:?}",
            p.artefacts
        );
        let t = &p.artefacts[0];
        assert_eq!(t.kind, "target");
        assert_eq!(t.tracked, Some(false));
        assert!(t.gitignored);
        assert_eq!(t.risk, Risk::Green);
        assert_eq!(p.rebuild, vec!["cargo build"]);
    }

    #[test]
    fn active_project_is_yellow() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let proj = home.join("p");
        write(&proj.join("pyproject.toml"), b"[project]\nname='p'\n");
        write(&proj.join(".gitignore"), b".venv\n");
        write(&proj.join(".venv/lib/x.py"), &[1u8; 2000]);
        write(&proj.join("main.py"), b"print(1)"); // fresh
        let projects = find_projects(
            &opts(),
            &Env::with_home(&home),
            &ProtectedPaths::os_defaults(),
        );
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].artefacts[0].risk, Risk::Yellow);
        assert!(projects[0].idle_days.is_some_and(|d| d < 1));
    }

    #[test]
    fn gitignore_matching() {
        let g = GitIgnore {
            patterns: vec![
                "node_modules".into(),
                "dist".into(),
                "*.log".into(),
                "**/coverage".into(),
            ],
        };
        assert!(g.matches("node_modules"));
        assert!(g.matches("dist"));
        assert!(g.matches("coverage"));
        assert!(!g.matches("build"));
    }
}
