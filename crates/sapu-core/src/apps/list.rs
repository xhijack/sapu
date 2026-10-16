//! Enumerates installed apps. macOS reads `.app` bundles; other platforms
//! return an empty list for now (documented in the UI).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub path: PathBuf,
    pub bytes: u64,
    pub last_used: Option<SystemTime>,
    /// Apple / OS-owned: shown, never removable.
    pub is_system: bool,
}

/// Apps in the standard locations, largest first.
pub fn list_apps() -> Vec<InstalledApp> {
    let mut out = platform::list();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    out
}

/// Whether a process from this bundle is running (macOS). Elsewhere `false`.
pub fn is_running(app_path: &Path) -> bool {
    platform::is_running(app_path)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub(super) fn list() -> Vec<InstalledApp> {
        let mut roots = vec![PathBuf::from("/Applications")];
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Applications"));
        }
        let mut out = Vec::new();
        for root in roots {
            let Ok(rd) = std::fs::read_dir(&root) else {
                continue;
            };
            for entry in rd.filter_map(Result::ok) {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "app") {
                    if let Some(app) = read_bundle(&p) {
                        out.push(app);
                    }
                } else if p.is_dir() {
                    // One level of grouping folders (e.g. /Applications/Utilities).
                    if let Ok(sub) = std::fs::read_dir(&p) {
                        for e in sub.filter_map(Result::ok) {
                            let q = e.path();
                            if q.extension().is_some_and(|x| x == "app")
                                && let Some(app) = read_bundle(&q)
                            {
                                out.push(app);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn read_bundle(path: &Path) -> Option<InstalledApp> {
        let info = path.join("Contents/Info.plist");
        let plist: plist::Dictionary = plist::from_file(&info).ok()?;
        let get = |k: &str| plist.get(k).and_then(|v| v.as_string()).map(str::to_string);
        let name = get("CFBundleDisplayName")
            .or_else(|| get("CFBundleName"))
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        let bundle_id = get("CFBundleIdentifier");
        let is_system = bundle_id
            .as_deref()
            .is_some_and(|b| b.starts_with("com.apple."));
        let meta = std::fs::metadata(path).ok();
        Some(InstalledApp {
            name,
            version: get("CFBundleShortVersionString").or_else(|| get("CFBundleVersion")),
            bytes: crate::cleaner::dir_size_pub(path),
            last_used: meta.and_then(|m| m.accessed().ok()),
            path: path.to_path_buf(),
            bundle_id,
            is_system,
        })
    }

    pub(super) fn is_running(app_path: &Path) -> bool {
        let needle = app_path
            .join("Contents/MacOS")
            .to_string_lossy()
            .into_owned();
        std::process::Command::new("pgrep")
            .args(["-f", &needle])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub(super) fn list() -> Vec<InstalledApp> {
        Vec::new()
    }

    pub(super) fn is_running(_app_path: &Path) -> bool {
        false
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "walks and sizes every bundle in /Applications; run manually"]
    fn lists_apps_with_names_and_sizes() {
        let apps = list_apps();
        // Every Mac has at least Safari or Finder-adjacent apps in /Applications.
        if apps.is_empty() {
            return;
        }
        assert!(apps.iter().all(|a| !a.name.is_empty()));
        assert!(
            apps.windows(2).all(|w| w[0].bytes >= w[1].bytes),
            "largest first"
        );
        assert!(apps.iter().any(|a| a.is_system) || apps.iter().all(|a| !a.is_system));
    }
}
