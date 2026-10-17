//! Files an app leaves in the user's Library, matched conservatively by
//! bundle identifier (exact) or app name (exact folder name).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::list::InstalledApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeftoverKind {
    Support,
    Cache,
    Preferences,
    Logs,
    Container,
    GroupContainer,
    SavedState,
    WebKit,
    LaunchAgent,
    Cookies,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Leftover {
    pub path: PathBuf,
    pub kind: LeftoverKind,
    pub bytes: u64,
}

/// `(relative Library folder, kind, match on bundle id prefix?)`.
const LOCATIONS: &[(&str, LeftoverKind)] = &[
    ("Application Support", LeftoverKind::Support),
    ("Caches", LeftoverKind::Cache),
    ("Preferences", LeftoverKind::Preferences),
    ("Logs", LeftoverKind::Logs),
    ("Containers", LeftoverKind::Container),
    ("Group Containers", LeftoverKind::GroupContainer),
    ("Saved Application State", LeftoverKind::SavedState),
    ("WebKit", LeftoverKind::WebKit),
    ("HTTPStorages", LeftoverKind::Cookies),
    ("Cookies", LeftoverKind::Cookies),
    ("LaunchAgents", LeftoverKind::LaunchAgent),
];

fn library_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(home) = dirs::home_dir() {
        v.push(home.join("Library"));
    }
    v
}

/// Does this Library entry belong to the app? Exact bundle id (or bundle id
/// followed by a separator) or the app's exact name.
fn matches(entry_name: &str, app: &InstalledApp) -> bool {
    let lower = entry_name.to_lowercase();
    if let Some(id) = &app.bundle_id {
        let id = id.to_lowercase();
        if lower == id
            || lower.starts_with(&format!("{id}."))
            || lower.starts_with(&format!("{id}-"))
            || lower.starts_with(&format!("{id}_"))
        {
            return true;
        }
        // Saved state and some caches use "<bundle>.savedState" / "<bundle>.plist".
        if lower.strip_suffix(".savedstate") == Some(id.as_str())
            || lower.strip_suffix(".plist") == Some(id.as_str())
        {
            return true;
        }
    }
    let name = app.name.to_lowercase();
    name.len() >= 3 && lower == name
}

/// Leftovers for an installed (or just removed) app.
pub fn leftovers_for(app: &InstalledApp) -> Vec<Leftover> {
    let mut out = Vec::new();
    for lib in library_dirs() {
        for (sub, kind) in LOCATIONS {
            let dir = lib.join(sub);
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.filter_map(Result::ok) {
                let p = e.path();
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if matches(name, app) {
                    out.push(Leftover {
                        bytes: size_of(&p),
                        path: p,
                        kind: *kind,
                    });
                }
            }
        }
    }
    out.sort_by_key(|l| std::cmp::Reverse(l.bytes));
    out
}

/// Library entries that look like an app's data (reverse-DNS names) whose
/// app is not installed any more. Conservative: only Application Support,
/// Caches and Containers, and only names with at least two dots.
pub fn orphan_leftovers(installed: &[InstalledApp]) -> Vec<Leftover> {
    let ids: HashSet<String> = installed
        .iter()
        .filter_map(|a| a.bundle_id.as_ref())
        .map(|s| s.to_lowercase())
        .collect();
    let names: HashSet<String> = installed.iter().map(|a| a.name.to_lowercase()).collect();
    let mut out = Vec::new();
    for lib in library_dirs() {
        for (sub, kind) in [
            ("Application Support", LeftoverKind::Support),
            ("Caches", LeftoverKind::Cache),
            ("Containers", LeftoverKind::Container),
        ] {
            let Ok(rd) = std::fs::read_dir(lib.join(sub)) else {
                continue;
            };
            for e in rd.filter_map(Result::ok) {
                let p = e.path();
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let lower = name.to_lowercase();
                if lower.matches('.').count() < 2 || lower.starts_with("com.apple.") {
                    continue;
                }
                let owned = ids
                    .iter()
                    .any(|id| lower == *id || lower.starts_with(&format!("{id}.")))
                    || names.contains(&lower);
                if !owned {
                    out.push(Leftover {
                        bytes: size_of(&p),
                        path: p,
                        kind,
                    });
                }
            }
        }
    }
    out.sort_by_key(|l| std::cmp::Reverse(l.bytes));
    out
}

fn size_of(p: &Path) -> u64 {
    match std::fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => crate::cleaner::dir_size_pub(p),
        Ok(m) => crate::platform::disk_usage_of(&m),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, id: Option<&str>) -> InstalledApp {
        InstalledApp {
            name: name.into(),
            bundle_id: id.map(str::to_string),
            version: None,
            path: PathBuf::from(format!("/Applications/{name}.app")),
            bytes: 0,
            last_used: None,
            is_system: false,
            support: crate::apps::AppSupport::Unknown,
        }
    }

    #[test]
    fn matching_is_exact_not_fuzzy() {
        let spotify = app("Spotify", Some("com.spotify.client"));
        assert!(matches("com.spotify.client", &spotify));
        assert!(matches("com.spotify.client.helper", &spotify));
        assert!(matches("com.spotify.client.savedState", &spotify));
        assert!(matches("com.spotify.client.plist", &spotify));
        assert!(matches("Spotify", &spotify));
        assert!(!matches("com.spotifyclone.app", &spotify));
        assert!(!matches("Spotify Helper", &spotify));
        let short = app("Go", None);
        assert!(!matches("go", &short), "very short names are not trusted");
    }
}
