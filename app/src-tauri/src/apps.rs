//! Installed apps, their leftovers, and uninstall through the safety layer.

use std::path::PathBuf;

use sapu_core::apps::{
    InstalledApp, Leftover, is_running, leftovers_for, list_apps, orphan_leftovers,
};
use sapu_core::progress::NoProgress;
use sapu_core::safety::{
    CleanSession, DeletePlan, FileSnapshot, HistoryStore, ProtectedPaths, Safety,
};
use tauri::AppHandle;

use crate::{CmdResult, CommandError};

#[tauri::command]
pub async fn apps_list() -> CmdResult<Vec<InstalledApp>> {
    tauri::async_runtime::spawn_blocking(list_apps)
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

#[tauri::command]
pub async fn apps_leftovers(app: InstalledApp) -> CmdResult<Vec<Leftover>> {
    tauri::async_runtime::spawn_blocking(move || leftovers_for(&app))
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

#[tauri::command]
pub async fn apps_orphans() -> CmdResult<Vec<Leftover>> {
    tauri::async_runtime::spawn_blocking(|| orphan_leftovers(&list_apps()))
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

#[tauri::command]
pub fn apps_is_running(path: PathBuf) -> bool {
    is_running(&path)
}

/// Move the bundle (optional) and the chosen leftovers to the Trash.
/// Refused while the app is running or when it is OS-owned.
#[tauri::command]
pub fn apps_uninstall(
    app: Option<InstalledApp>,
    leftovers: Vec<PathBuf>,
    apply: bool,
) -> CmdResult<CleanSession> {
    if let Some(a) = &app {
        if a.is_system {
            return Err(CommandError::new("protected", "system app"));
        }
        if apply && is_running(&a.path) {
            return Err(CommandError::new("app_running", "quit the app first"));
        }
    }
    let mut paths: Vec<PathBuf> = leftovers;
    if let Some(a) = &app {
        paths.insert(0, a.path.clone());
    }
    if paths.is_empty() {
        return Err(CommandError::new("io", "nothing selected"));
    }
    let scope = DeletePlan::common_scope(paths.iter().map(PathBuf::as_path))
        .ok_or_else(|| CommandError::new("io", "no common scope"))?;
    let mut plan = DeletePlan::new(scope);
    for p in &paths {
        plan.push(FileSnapshot::capture(p)?);
    }
    if apply {
        plan = plan.apply();
    }
    let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
    Ok(safety.execute(&plan, &mut NoProgress)?)
}

/// App icon as a PNG data URL (macOS: converted from the bundle's .icns
/// with `sips` into a cache). Empty string when unavailable.
#[tauri::command]
pub async fn app_icon(_app: AppHandle, path: PathBuf) -> String {
    tauri::async_runtime::spawn_blocking(move || icon_data_url(&path).unwrap_or_default())
        .await
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn icon_data_url(bundle: &std::path::Path) -> Option<String> {
    let info: plist::Dictionary = plist::from_file(bundle.join("Contents/Info.plist")).ok()?;
    let mut icon = info
        .get("CFBundleIconFile")
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .or_else(|| {
            info.get("CFBundleIcons")
                .and_then(|v| v.as_dictionary())
                .and_then(|d| d.get("CFBundlePrimaryIcon"))
                .and_then(|v| v.as_dictionary())
                .and_then(|d| d.get("CFBundleIconName"))
                .and_then(|v| v.as_string())
                .map(str::to_string)
        })?;
    if !icon.ends_with(".icns") {
        icon.push_str(".icns");
    }
    let icns = bundle.join("Contents/Resources").join(&icon);
    if !icns.exists() {
        return None;
    }
    let cache = dirs::cache_dir()?.join("sapu").join("icons");
    std::fs::create_dir_all(&cache).ok()?;
    let key = blake3::hash(bundle.to_string_lossy().as_bytes()).to_hex();
    let png = cache.join(format!("{key}.png"));
    if !png.exists() {
        let ok = std::process::Command::new("sips")
            .args(["-s", "format", "png", "-Z", "128"])
            .arg(&icns)
            .arg("--out")
            .arg(&png)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return None;
        }
    }
    let bytes = std::fs::read(&png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        crate::dedupe::base64_pub(&bytes)
    ))
}

#[cfg(not(target_os = "macos"))]
fn icon_data_url(_bundle: &std::path::Path) -> Option<String> {
    None
}
