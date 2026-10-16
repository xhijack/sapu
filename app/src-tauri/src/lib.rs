//! Tauri commands. Each one is a thin call into `sapu-core`; no business
//! logic lives here (CLAUDE.md rule 1).

use sapu_core::platform::{DevTool, DiskAccess, detect_dev_tools, full_disk_access};
use sapu_core::safety::{
    CleanSession, HistoryStore, ProtectedEntry, ProtectedPaths, RestoreReport,
};
use sapu_core::settings::Settings;
use tauri_plugin_opener::OpenerExt;

/// Errors cross the IPC boundary as a stable code plus a message; the
/// frontend maps the code to a translated string.
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
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

type CmdResult<T> = Result<T, CommandError>;

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
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
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
