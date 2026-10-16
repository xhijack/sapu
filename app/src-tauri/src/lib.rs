//! Tauri commands. Each one is a thin call into `sapu-core`; no business
//! logic lives here (CLAUDE.md rule 1).

use sapu_core::platform::{
    DevTool, DiskAccess, SystemMonitor, SystemSnapshot, detect_dev_tools, full_disk_access,
};
use sapu_core::safety::{
    CleanSession, HistoryStore, ProtectedEntry, ProtectedPaths, RestoreReport,
};
use sapu_core::settings::Settings;
use tauri_plugin_opener::OpenerExt;

mod clean;
mod dedupe;
mod scan;

/// Errors cross the IPC boundary as a stable code plus a message; the
/// frontend maps the code to a translated string.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<sapu_core::SapuError> for CommandError {
    fn from(e: sapu_core::SapuError) -> Self {
        use sapu_core::SapuError as E;
        let code = match &e {
            E::Protected(_) => "protected",
            E::Validation { .. } => "validation",
            E::Disposal { .. } => "disposal",
            E::RestoreUnsupported => "restore_unsupported",
            E::Io { .. } => "io",
            E::Serde(_) => "serde",
        };
        Self {
            code,
            message: e.to_string(),
        }
    }
}

pub(crate) type CmdResult<T> = Result<T, CommandError>;

#[derive(Debug, serde::Serialize)]
pub struct AppInfo {
    version: &'static str,
    os: &'static str,
    full_disk_access: DiskAccess,
    dev_tools: Vec<DevTool>,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        full_disk_access: full_disk_access(),
        dev_tools: detect_dev_tools(),
    }
}

#[tauri::command]
fn disk_access_status() -> DiskAccess {
    full_disk_access()
}

#[tauri::command]
fn settings_get() -> CmdResult<Settings> {
    Ok(Settings::load(&Settings::default_path()?)?)
}

#[tauri::command]
fn settings_set(settings: Settings) -> CmdResult<()> {
    settings.save(&Settings::default_path()?)?;
    Ok(())
}

#[tauri::command]
fn protected_paths() -> Vec<ProtectedEntry> {
    ProtectedPaths::os_defaults().entries().to_vec()
}

#[tauri::command]
fn history_list() -> CmdResult<Vec<CleanSession>> {
    Ok(HistoryStore::open_default()?.list()?)
}

#[tauri::command]
fn history_restore(id: String) -> CmdResult<RestoreReport> {
    Ok(HistoryStore::open_default()?.restore_session(&id)?)
}

/// Live disks / CPU / memory for the home dashboard.
#[tauri::command]
fn system_snapshot(
    monitor: tauri::State<'_, std::sync::Mutex<SystemMonitor>>,
) -> CmdResult<SystemSnapshot> {
    let mut m = monitor
        .lock()
        .map_err(|_| CommandError::new("io", "system monitor poisoned"))?;
    Ok(m.snapshot())
}

/// Reveals the running binary in the file manager so the user can drag it
/// into the Full Disk Access list (dev builds are not registered apps).
#[tauri::command]
fn reveal_app_binary(app: tauri::AppHandle) -> CmdResult<()> {
    let exe = std::env::current_exe().map_err(|e| CommandError::new("io", e.to_string()))?;
    // Inside an .app bundle, point at the bundle rather than Contents/MacOS/<bin>.
    let target = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(std::path::Path::to_path_buf)
        .unwrap_or(exe);
    app.opener()
        .reveal_item_in_dir(&target)
        .map_err(|e| CommandError::new("opener", e.to_string()))
}

/// Opens the OS privacy settings where Full Disk Access is granted.
#[tauri::command]
fn open_disk_access_settings(app: tauri::AppHandle) -> CmdResult<()> {
    #[cfg(target_os = "macos")]
    let target = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";
    #[cfg(not(target_os = "macos"))]
    let target = "";
    if target.is_empty() {
        return Ok(());
    }
    app.opener()
        .open_url(target, None::<&str>)
        .map_err(|e| CommandError {
            code: "opener",
            message: e.to_string(),
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Logs from core (`sapu_core`) and this crate; override with RUST_LOG.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("sapu_core=info,sapu_app_lib=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(std::sync::Arc::new(scan::ScanSession::default()))
        .manage(std::sync::Arc::new(clean::CleanSession::default()))
        .manage(std::sync::Arc::new(dedupe::DedupeSession::default()))
        .manage(std::sync::Mutex::new(SystemMonitor::new()))
        .invoke_handler(tauri::generate_handler![
            scan::scan_targets,
            scan::pick_folder,
            scan::scan_start,
            scan::scan_pause,
            scan::scan_resume,
            scan::scan_cancel,
            scan::scan_status,
            scan::scan_summary,
            scan::scan_top_files,
            scan::scan_top_dirs,
            scan::scan_subtree,
            scan::reveal_path,
            scan::trash_paths,
            clean::clean_evaluate,
            clean::clean_execute,
            dedupe::dedupe_start,
            dedupe::dedupe_cancel,
            dedupe::dedupe_summary,
            dedupe::dedupe_groups,
            dedupe::dedupe_folder_pairs,
            dedupe::dedupe_execute,
            dedupe::file_preview,
            app_info,
            system_snapshot,
            reveal_app_binary,
            disk_access_status,
            settings_get,
            settings_set,
            protected_paths,
            history_list,
            history_restore,
            open_disk_access_settings,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        tracing::error!(error = %e, "tauri application failed");
        std::process::exit(1);
    }
}
