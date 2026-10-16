//! Scan commands: run `sapu_core::scanner` on a worker thread, stream
//! progress as events, answer queries over the last result.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sapu_core::eta::{Estimator, Eta};
use sapu_core::progress::{FnSink, ProgressEvent};
use sapu_core::safety::{DeletePlan, FileSnapshot, HistoryStore, ProtectedPaths, Safety};
use sapu_core::scanner::{
    DirSummary, FileEntry, FileFilter, ScanControl, ScanOptions, ScanResult, ScanState, ScanStats,
    ScanSummary, TreeNode, scan,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::{CmdResult, CommandError};

#[derive(Default)]
pub struct ScanSession {
    result: Mutex<Option<Arc<ScanResult>>>,
    control: Mutex<Option<ScanControl>>,
    running: AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressPayload {
    fraction: Option<f64>,
    items_done: u64,
    items_total: Option<u64>,
    bytes_done: u64,
    bytes_total: Option<u64>,
    elapsed_secs: u64,
    eta: Eta,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStatus {
    running: bool,
    state: Option<ScanState>,
    has_result: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanTargets {
    home: Option<PathBuf>,
    disk: PathBuf,
}

fn disk_root() -> PathBuf {
    #[cfg(windows)]
    {
        let mut d = std::env::var_os("SystemDrive")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:"));
        d.push("\\");
        d
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/")
    }
}

#[tauri::command]
pub fn scan_targets() -> ScanTargets {
    ScanTargets {
        home: dirs::home_dir(),
        disk: disk_root(),
    }
}

/// Native folder picker. Runs off the main thread (async command).
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<PathBuf> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
}

#[tauri::command]
pub fn scan_start(
    app: AppHandle,
    session: State<'_, Arc<ScanSession>>,
    root: PathBuf,
) -> CmdResult<()> {
    if session.running.swap(true, Ordering::AcqRel) {
        return Err(CommandError::new(
            "scan_running",
            "a scan is already running",
        ));
    }
    let control = ScanControl::new();
    if let Ok(mut c) = session.control.lock() {
        *c = Some(control.clone());
    }
    let session = Arc::clone(&session);
    std::thread::spawn(move || {
        let stats_dir = ScanStats::default_dir().ok();
        let opts = ScanOptions::new(&root, stats_dir.as_deref());
        let start = Instant::now();
        let mut est = Estimator::default();
        let app2 = app.clone();
        let mut sink = FnSink(move |e: &ProgressEvent| {
            let elapsed = start.elapsed();
            est.observe(e, elapsed);
            let fraction = if e.bytes_total.is_some() || e.items_total.is_some() {
                Some(est.fraction(e))
            } else {
                None
            };
            let payload = ProgressPayload {
                fraction,
                items_done: e.items_done,
                items_total: e.items_total,
                bytes_done: e.bytes_done,
                bytes_total: e.bytes_total,
                elapsed_secs: elapsed.as_secs(),
                eta: est.eta(e, elapsed),
            };
            let _ = app2.emit("scan://progress", payload);
        });

        match scan(&opts, &control, &mut sink) {
            Ok(result) => {
                if let Some(dir) = &stats_dir
                    && !result.cancelled
                {
                    let _ = ScanStats {
                        total_bytes: result.total_bytes,
                        file_count: result.file_count,
                        finished_at_unix: result.finished_at_unix,
                    }
                    .save(dir, &result.root);
                }
                let summary = result.summary();
                if let Ok(mut r) = session.result.lock() {
                    *r = Some(Arc::new(result));
                }
                let _ = app.emit("scan://done", summary);
            }
            Err(e) => {
                let _ = app.emit("scan://error", CommandError::from(e));
            }
        }
        session.running.store(false, Ordering::Release);
    });
    Ok(())
}

fn with_control(session: &ScanSession, f: impl FnOnce(&ScanControl)) {
    if let Ok(c) = session.control.lock()
        && let Some(c) = c.as_ref()
    {
        f(c);
    }
}

#[tauri::command]
pub fn scan_pause(session: State<'_, Arc<ScanSession>>) {
    with_control(&session, ScanControl::pause);
}

#[tauri::command]
pub fn scan_resume(session: State<'_, Arc<ScanSession>>) {
    with_control(&session, ScanControl::resume);
}

#[tauri::command]
pub fn scan_cancel(session: State<'_, Arc<ScanSession>>) {
    with_control(&session, ScanControl::cancel);
}

#[tauri::command]
pub fn scan_status(session: State<'_, Arc<ScanSession>>) -> ScanStatus {
    let state = session
        .control
        .lock()
        .ok()
        .and_then(|c| c.as_ref().map(ScanControl::state));
    ScanStatus {
        running: session.running.load(Ordering::Acquire),
        state,
        has_result: session.result.lock().is_ok_and(|r| r.is_some()),
    }
}

fn result(session: &ScanSession) -> CmdResult<Arc<ScanResult>> {
    session
        .result
        .lock()
        .ok()
        .and_then(|r| r.clone())
        .ok_or_else(|| CommandError::new("no_scan", "no scan result yet"))
}

#[tauri::command]
pub fn scan_summary(session: State<'_, Arc<ScanSession>>) -> CmdResult<ScanSummary> {
    Ok(result(&session)?.summary())
}

#[tauri::command]
pub fn scan_top_files(
    session: State<'_, Arc<ScanSession>>,
    filter: FileFilter,
    limit: usize,
) -> CmdResult<Vec<FileEntry>> {
    Ok(result(&session)?.top_files(&filter, limit.min(1000)))
}

#[tauri::command]
pub fn scan_top_dirs(
    session: State<'_, Arc<ScanSession>>,
    limit: usize,
) -> CmdResult<Vec<DirSummary>> {
    Ok(result(&session)?.top_dirs(limit.min(1000)))
}

#[tauri::command]
pub fn scan_subtree(
    session: State<'_, Arc<ScanSession>>,
    path: Option<PathBuf>,
    depth: usize,
    limit: usize,
) -> CmdResult<Option<TreeNode>> {
    let r = result(&session)?;
    let path = path.unwrap_or_else(|| r.root.clone());
    Ok(r.subtree(&path, depth.min(3), limit.min(200)))
}

#[tauri::command]
pub fn reveal_path(app: AppHandle, path: PathBuf) -> CmdResult<()> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| CommandError::new("opener", e.to_string()))
}

/// Moves the given items to the trash through the safety layer. Dry run
/// unless `apply`; the returned session is the preview or the record.
#[tauri::command]
pub fn trash_paths(paths: Vec<PathBuf>, apply: bool) -> CmdResult<sapu_core::safety::CleanSession> {
    let scope = DeletePlan::common_scope(paths.iter().map(PathBuf::as_path))
        .ok_or_else(|| CommandError::new("io", "cannot derive a common scope"))?;
    let mut plan = DeletePlan::new(scope);
    for p in &paths {
        plan.push(FileSnapshot::capture(p)?);
    }
    if apply {
        plan = plan.apply();
    }
    let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
    Ok(safety.execute(&plan, &mut sapu_core::progress::NoProgress)?)
}
