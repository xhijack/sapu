//! Organize ("Rapikan") commands. The plan stays here, in the session: the
//! UI sends back only what the person ticked (`Selection`), and the move
//! plan is built from the stored plan, so nothing that crossed IPC decides
//! which paths move.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sapu_core::eta::{Estimator, Eta};
use sapu_core::organize::{
    FolderListing, ItemAdvice, Leftover, OrganizePlan, OrganizeSession as RunRecord, OrganizeStore,
    Selection, UndoReport, Unit, apply, build_move_plan, duplicates_in, find_leftovers,
    plan_from_advice, propose, scan_folder, undo,
};
use sapu_core::progress::{FnSink, NoProgress, ProgressEvent};
use sapu_core::safety::{HistoryStore, ProtectedPaths, Safety};
use sapu_core::scanner::ScanControl;
use tauri::{AppHandle, Emitter, State};

use crate::support::notify_done;
use crate::{CmdResult, CommandError};

#[derive(Default)]
pub struct OrganizeState {
    plan: Mutex<Option<OrganizeAnalysis>>,
    analysing: Mutex<bool>,
    applying: Mutex<bool>,
}

/// What the plan screen shows: the groups, and duplicates found on the way.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganizeAnalysis {
    pub plan: OrganizePlan,
    pub unreadable: Vec<PathBuf>,
    pub duplicate_groups: u64,
    pub duplicate_files: u64,
    pub duplicate_bytes: u64,
    /// Office lock files and extra exact copies, offered for the Trash.
    pub leftovers: Vec<Leftover>,
    /// The AI renamed or merged groups (false when off or it failed).
    pub ai_used: bool,
    /// Why AI was not used, when it was asked for.
    pub ai_note: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AnalyseProgress {
    /// reading | grouping | ai | duplicates
    stage: &'static str,
    elapsed_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RunProgress {
    items_done: u64,
    items_total: Option<u64>,
    fraction: Option<f64>,
    elapsed_secs: u64,
    eta: Eta,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunResult {
    session: RunRecord,
    failures: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganizeTarget {
    key: &'static str,
    path: PathBuf,
}

/// Documents, Desktop and Downloads when they exist.
#[tauri::command]
pub fn organize_targets() -> Vec<OrganizeTarget> {
    [
        ("documents", dirs::document_dir()),
        ("desktop", dirs::desktop_dir()),
        ("downloads", dirs::download_dir()),
    ]
    .into_iter()
    .filter_map(|(key, p)| {
        p.filter(|p| p.is_dir())
            .map(|path| OrganizeTarget { key, path })
    })
    .collect()
}

fn take_flag(flag: &Mutex<bool>) -> CmdResult<()> {
    let mut f = flag
        .lock()
        .map_err(|_| CommandError::new("io", "poisoned"))?;
    if *f {
        return Err(CommandError::new("scan_running", "already running"));
    }
    *f = true;
    Ok(())
}

fn release(flag: &Mutex<bool>) {
    if let Ok(mut f) = flag.lock() {
        *f = false;
    }
}

/// Reads the folder and proposes groups in the background. Events:
/// `organize://progress`, `organize://plan` (an `OrganizeAnalysis`),
/// `organize://error`.
#[tauri::command]
pub fn organize_analyze_start(
    app: AppHandle,
    state: State<'_, Arc<OrganizeState>>,
    root: PathBuf,
    use_ai: bool,
) -> CmdResult<()> {
    take_flag(&state.analysing)?;
    let state = Arc::clone(&state);
    std::thread::spawn(move || {
        let start = Instant::now();
        let stage = |s: &'static str| {
            let _ = app.emit(
                "organize://progress",
                AnalyseProgress {
                    stage: s,
                    elapsed_secs: start.elapsed().as_secs(),
                },
            );
        };
        let result = (|| -> Result<OrganizeAnalysis, CommandError> {
            stage("reading");
            let listing = scan_folder(&root)?;
            stage("grouping");
            let mut plan = propose(&listing);
            let (ai_used, ai_note) = if use_ai {
                stage("ai");
                advise_with_ai(&listing, &mut plan)
            } else {
                (false, None)
            };
            stage("duplicates");
            let dupes = duplicates_in(&root, &ScanControl::new(), &mut NoProgress).ok();
            let leftovers = find_leftovers(&listing, dupes.as_ref());
            Ok(OrganizeAnalysis {
                plan,
                unreadable: listing.unreadable,
                duplicate_groups: dupes.as_ref().map(|d| d.groups.len() as u64).unwrap_or(0),
                duplicate_files: dupes
                    .as_ref()
                    .map(|d| d.groups.iter().map(|g| g.files.len() as u64).sum())
                    .unwrap_or(0),
                duplicate_bytes: dupes
                    .as_ref()
                    .map(|d| d.groups.iter().map(|g| g.wasted_bytes).sum())
                    .unwrap_or(0),
                leftovers,
                ai_used,
                ai_note,
            })
        })();
        match result {
            Ok(analysis) => {
                if let Ok(mut slot) = state.plan.lock() {
                    *slot = Some(analysis.clone());
                }
                let _ = app.emit("organize://plan", analysis);
            }
            Err(e) => {
                let _ = app.emit("organize://error", e);
            }
        }
        release(&state.analysing);
    });
    Ok(())
}

/// The serde name of a unit or kind ("file", "document", ...).
fn label<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Most items sent to the AI in one request. Items past it get no advice
/// and stay where they are (listed as unplaced).
const AI_MAX_ITEMS: usize = 250;

/// Asks the configured AI where each item belongs, plus renames and junk.
/// Never fails the run: an error or an unusable answer keeps the heuristic
/// plan and is reported as a note.
fn advise_with_ai(listing: &FolderListing, plan: &mut OrganizePlan) -> (bool, Option<String>) {
    use sapu_ai::tasks::tidy_items::{TidyItem, TidyItems, TidyItemsInput};
    let client = match crate::ai::client() {
        Ok(c) => c,
        Err(e) => return (false, Some(e.code.to_string())),
    };
    let year = |t: Option<std::time::SystemTime>| {
        t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| 1970 + (d.as_secs() / 31_557_600) as i32)
    };
    let input = TidyItemsInput {
        language: crate::ai::language(),
        folder_name: listing
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        items: listing
            .entries
            .iter()
            .filter(|e| e.unit != Unit::Link)
            .take(AI_MAX_ITEMS)
            .map(|e| TidyItem {
                name: e.name.clone(),
                unit: label(&e.unit),
                kind: label(&e.kind),
                year: year(e.mtime),
                size_kb: e.bytes / 1024,
                children: e.children.clone(),
            })
            .collect(),
    };
    let sent: Vec<String> = input.items.iter().map(|i| i.name.clone()).collect();
    match client.run::<TidyItems>(&input) {
        Ok(mut answer) => {
            // Names went out anonymised; match the answer to the real files.
            answer.restore_names(&sent, client.anonymizer());
            let advice: Vec<ItemAdvice> = answer
                .items
                .into_iter()
                .map(|a| ItemAdvice {
                    name: a.name,
                    category: a.category,
                    rename: a.rename,
                    junk: a.junk,
                    reason: a.reason,
                    confidence: a.confidence,
                })
                .collect();
            match plan_from_advice(listing, &advice) {
                Some(p) => {
                    *plan = p;
                    (true, None)
                }
                None => (false, Some("ai_bad_answer".into())),
            }
        }
        Err(e) => (false, Some(e.to_string())),
    }
}

/// The last analysis, so the plan survives switching screens.
#[tauri::command]
pub fn organize_plan(state: State<'_, Arc<OrganizeState>>) -> Option<OrganizeAnalysis> {
    state.plan.lock().ok().and_then(|p| p.clone())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganizeStatus {
    analysing: bool,
    applying: bool,
    has_plan: bool,
}

#[tauri::command]
pub fn organize_status(state: State<'_, Arc<OrganizeState>>) -> OrganizeStatus {
    OrganizeStatus {
        analysing: state.analysing.lock().map(|f| *f).unwrap_or(false),
        applying: state.applying.lock().map(|f| *f).unwrap_or(false),
        has_plan: state.plan.lock().map(|p| p.is_some()).unwrap_or(false),
    }
}

/// Moves what the person ticked, in the background. Events:
/// `organize://run_progress`, `organize://run_done` (a `RunResult`),
/// `organize://run_error`.
#[tauri::command]
pub fn organize_apply_start(
    app: AppHandle,
    state: State<'_, Arc<OrganizeState>>,
    selection: Selection,
) -> CmdResult<()> {
    let plan = state
        .plan
        .lock()
        .ok()
        .and_then(|p| p.as_ref().map(|a| a.plan.clone()))
        .ok_or_else(|| CommandError::new("no_evaluation", "analyse first"))?;
    let moves = build_move_plan(&plan, &selection, &ProtectedPaths::os_defaults())?;
    take_flag(&state.applying)?;
    let state = Arc::clone(&state);
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut est = Estimator::default();
        let app_sink = app.clone();
        let mut sink = FnSink(move |e: &ProgressEvent| {
            let elapsed = start.elapsed();
            est.observe(e, elapsed);
            let _ = app_sink.emit(
                "organize://run_progress",
                RunProgress {
                    items_done: e.items_done,
                    items_total: e.items_total,
                    fraction: e.items_total.map(|_| est.fraction(e)),
                    elapsed_secs: elapsed.as_secs(),
                    eta: est.eta(e, elapsed),
                },
            );
        });
        let result = OrganizeStore::open_default()
            .and_then(|store| apply(&moves, false, &store, &mut sink))
            .map_err(CommandError::from);
        match result {
            Ok(report) => {
                notify_done(
                    &app,
                    start.elapsed(),
                    "Sapu",
                    &format!("Rapikan selesai: {} item dipindahkan", report.session.moved),
                );
                // The plan described the folder before the move; it is stale now.
                if let Ok(mut slot) = state.plan.lock() {
                    *slot = None;
                }
                let _ = app.emit(
                    "organize://run_done",
                    RunResult {
                        session: report.session,
                        failures: report.failures,
                    },
                );
            }
            Err(e) => {
                let _ = app.emit("organize://run_error", e);
            }
        }
        release(&state.applying);
    });
    Ok(())
}

/// Puts a run back. Folders Sapu created for it and that are empty again
/// are removed through `safety`.
#[tauri::command]
pub async fn organize_undo(id: String) -> CmdResult<UndoReport> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = OrganizeStore::open_default()?;
        let session = store
            .get(&id)?
            .ok_or_else(|| CommandError::new("io", "no such organize session"))?;
        let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
        Ok(undo(&session, &store, &safety, &mut NoProgress)?)
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// Previous runs, newest first.
#[tauri::command]
pub async fn organize_history(limit: usize) -> CmdResult<Vec<RunRecord>> {
    crate::off_main(move || {
        let mut list = OrganizeStore::open_default()?.list()?;
        list.truncate(limit.min(100));
        Ok(list)
    })
    .await
}
