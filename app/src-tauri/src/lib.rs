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

mod ai;
mod apps;
mod care;
mod clean;
mod dedupe;
mod memory;
mod organize;
mod scan;
mod support;
mod tray;

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

/// Runs blocking work (disk, keychain, large scan data) off the main thread,
/// so the window never freezes. See `scripts/check-sync-commands.sh`.
pub(crate) async fn off_main<T: Send + 'static>(
    f: impl FnOnce() -> CmdResult<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))?
}

#[derive(Debug, serde::Serialize)]
pub struct AppInfo {
    version: &'static str,
    os: &'static str,
    full_disk_access: DiskAccess,
    dev_tools: Vec<DevTool>,
}

#[tauri::command]
async fn app_info() -> CmdResult<AppInfo> {
    crate::off_main(move || {
        Ok({
            AppInfo {
                version: env!("CARGO_PKG_VERSION"),
                os: std::env::consts::OS,
                full_disk_access: full_disk_access(),
                dev_tools: detect_dev_tools(),
            }
        })
    })
    .await
}

#[tauri::command]
async fn disk_access_status() -> CmdResult<DiskAccess> {
    crate::off_main(move || Ok(full_disk_access())).await
}

#[tauri::command]
async fn settings_get() -> CmdResult<Settings> {
    crate::off_main(move || Ok(Settings::load(&Settings::default_path()?)?)).await
}

#[tauri::command]
async fn settings_set(settings: Settings) -> CmdResult<()> {
    crate::off_main(move || {
        settings.save(&Settings::default_path()?)?;
        Ok(())
    })
    .await
}

#[tauri::command]
fn protected_paths() -> Vec<ProtectedEntry> {
    ProtectedPaths::os_defaults().entries().to_vec()
}

#[tauri::command]
async fn history_list() -> CmdResult<Vec<CleanSession>> {
    crate::off_main(move || Ok(HistoryStore::open_default()?.list()?)).await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryTotals {
    sessions: usize,
    bytes_freed: u64,
    items_disposed: usize,
    last_session_unix: Option<u64>,
    first_session_unix: Option<u64>,
}

/// Lifetime totals of real (non dry-run) clean sessions.
#[tauri::command]
async fn history_totals() -> CmdResult<HistoryTotals> {
    crate::off_main(move || {
        let sessions = HistoryStore::open_default()?.list()?;
        let real: Vec<_> = sessions.iter().filter(|s| !s.dry_run).collect();
        Ok(HistoryTotals {
            sessions: real.len(),
            bytes_freed: real.iter().map(|s| s.bytes_freed).sum(),
            items_disposed: real.iter().map(|s| s.disposed).sum(),
            last_session_unix: real.iter().map(|s| s.finished_at_unix).max(),
            first_session_unix: real.iter().map(|s| s.finished_at_unix).min(),
        })
    })
    .await
}

#[tauri::command]
async fn history_restore(id: String) -> CmdResult<RestoreReport> {
    crate::off_main(move || Ok(HistoryStore::open_default()?.restore_session(&id)?)).await
}

/// Live disks / CPU / memory for the home dashboard.
#[tauri::command]
async fn system_snapshot(
    monitor: tauri::State<'_, std::sync::Arc<std::sync::Mutex<SystemMonitor>>>,
) -> CmdResult<SystemSnapshot> {
    let monitor = monitor.inner().clone();
    crate::off_main(move || {
        let mut m = monitor
            .lock()
            .map_err(|_| CommandError::new("io", "system monitor poisoned"))?;
        Ok(m.snapshot())
    })
    .await
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

/// Windows: start a second, elevated instance of Sapu and quit this one.
/// A no-op elsewhere.
#[tauri::command]
fn relaunch_as_admin(app: tauri::AppHandle) -> CmdResult<()> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().map_err(|e| CommandError::new("io", e.to_string()))?;
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Start-Process",
                "-Verb",
                "RunAs",
                "-FilePath",
            ])
            .arg(format!("'{}'", exe.display()))
            .spawn()
            .map_err(|e| CommandError::new("io", e.to_string()))?;
        app.exit(0);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Ok(())
    }
}

/// Opens an https link in the default browser (About screen links).
#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) -> CmdResult<()> {
    if !url.starts_with("https://") {
        return Err(CommandError::new(
            "validation",
            "only https links can be opened",
        ));
    }
    app.opener()
        .open_url(&url, None::<&str>)
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
        // Closing the window hides it so a running scan, Care or dedupe keeps
        // going; the app stays in the Dock / tray. Quit (Cmd+Q, tray "Keluar")
        // still exits. On Windows/Linux this only applies with the tray on,
        // otherwise there would be no way back to the window.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let keep_alive = cfg!(target_os = "macos") || tray::is_installed();
                if keep_alive {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(std::sync::Arc::new(scan::ScanSession::default()))
        .manage(std::sync::Arc::new(clean::CleanSession::default()))
        .manage(std::sync::Arc::new(care::CareSession::default()))
        .manage(std::sync::Arc::new(dedupe::DedupeSession::default()))
        .manage(std::sync::Arc::new(organize::OrganizeState::default()))
        .manage(std::sync::Arc::new(std::sync::Mutex::new(
            SystemMonitor::new(),
        )))
        .setup(|app| {
            // Native material behind the whole window: the sidebar reads as
            // part of macOS / Windows 11 instead of a web page.
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
                    if let Err(e) =
                        apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
                    {
                        tracing::warn!(error = %e, "vibrancy unavailable");
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    if let Err(e) = window_vibrancy::apply_mica(&window, None) {
                        tracing::warn!(error = %e, "mica unavailable");
                    }
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                let _ = window;
            }
            let tray_enabled = Settings::default_path()
                .and_then(|p| Settings::load(&p))
                .map(|s| s.menu_bar_monitor)
                .unwrap_or(true);
            if tray_enabled && let Err(e) = tray::install(app.handle()) {
                tracing::warn!(error = %e, "tray unavailable");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan::scan_targets,
            scan::scan_previous,
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
            scan::scan_browse,
            scan::downloads_report,
            scan::reveal_path,
            scan::trash_paths,
            apps::apps_list,
            apps::apps_leftovers,
            apps::apps_orphans,
            apps::apps_is_running,
            apps::apps_uninstall,
            apps::app_icon,
            care::care_run,
            care::care_last,
            care::care_history,
            clean::clean_check_start,
            clean::clean_check_status,
            clean::clean_last,
            clean::clean_run_start,
            clean::clean_run_status,
            clean::clean_run_last,
            clean::trash_status,
            organize::organize_targets,
            organize::organize_analyze_start,
            organize::organize_plan,
            organize::organize_status,
            organize::organize_apply_start,
            organize::organize_undo,
            organize::organize_history,
            clean::trash_empty_start,
            dedupe::dedupe_start,
            dedupe::dedupe_cancel,
            dedupe::dedupe_summary,
            dedupe::dedupe_groups,
            dedupe::dedupe_folder_pairs,
            dedupe::dedupe_execute,
            dedupe::file_preview,
            app_info,
            system_snapshot,
            memory::memory_snapshot,
            memory::memory_quit_apps,
            memory::memory_purge,
            support::estimate_duration,
            ai::ai_status,
            ai::ai_config_set,
            ai::ai_set_api_key,
            ai::ai_models,
            ai::ai_test,
            ai::ai_whatis,
            ai::ai_artefact,
            ai::ai_search,
            ai::ai_report,
            ai::ai_save_rule,
            reveal_app_binary,
            relaunch_as_admin,
            dedupe::dedupe_primary_folders,
            disk_access_status,
            settings_get,
            settings_set,
            protected_paths,
            history_list,
            history_totals,
            scan::scan_history,
            scan::quick_look,
            history_restore,
            open_disk_access_settings,
            open_url,
        ])
        .build(tauri::generate_context!())
        .map(|app| {
            app.run(|app, event| {
                // macOS: clicking the Dock icon while the window is hidden.
                #[cfg(target_os = "macos")]
                if let tauri::RunEvent::Reopen { .. } = event {
                    tray::show_main(app);
                }
                #[cfg(not(target_os = "macos"))]
                let _ = (app, event);
            });
        });
    if let Err(e) = result {
        tracing::error!(error = %e, "tauri application failed");
        std::process::exit(1);
    }
}
