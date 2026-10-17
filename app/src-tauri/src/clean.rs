//! Clean commands: evaluate rules, execute a selection through core.

use std::sync::{Arc, Mutex};

use crate::support::{notify_done, record_rate};
use sapu_core::cleaner::{
    CleanReport, CleanRunReport, EvalProgress, ProjectScanOptions, RunStep, Selection,
    evaluate_with_progress, execute_with_steps,
};
use sapu_core::eta::Estimator;
use sapu_core::eta::{Eta, OpKind};
use sapu_core::progress::{FnSink, ProgressEvent};
use sapu_core::rules::{
    Env, Localized, Profile, Risk, RuleSet, load_with_user_dir, user_rules_dir,
};
use sapu_core::safety::{HistoryStore, PermanentConfirmation, ProtectedPaths, Safety};
use tauri::{AppHandle, Emitter, State};

use crate::{CmdResult, CommandError};

#[derive(Default)]
pub struct CleanSession {
    report: Mutex<Option<CleanReport>>,
    /// Last finished check, served to the UI until the next one.
    evaluation: Mutex<Option<CleanEvaluation>>,
    running: Mutex<bool>,
    run_running: Mutex<bool>,
    last_run: Mutex<Option<CleanRunReport>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanRunProgress {
    items_done: u64,
    items_total: Option<u64>,
    bytes_done: u64,
    bytes_total: Option<u64>,
    fraction: Option<f64>,
    step: Option<RunStep>,
    elapsed_secs: u64,
    eta: Eta,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanRunStatus {
    running: bool,
    has_result: bool,
}

impl CleanSession {
    pub(crate) fn set_report(&self, r: CleanReport) {
        if let Ok(mut slot) = self.report.lock() {
            *slot = Some(r);
        }
    }

    pub(crate) fn project(&self, root: &std::path::Path) -> Option<sapu_core::cleaner::Project> {
        self.report
            .lock()
            .ok()?
            .as_ref()?
            .projects
            .iter()
            .find(|p| p.root == root)
            .cloned()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RuleInfo {
    id: String,
    name: Localized,
    description: Localized,
    risk: Risk,
    group: String,
    permanent: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanEvaluation {
    report: CleanReport,
    rules: Vec<RuleInfo>,
    preselected: Vec<String>,
    preselected_artefacts: Vec<std::path::PathBuf>,
    profile: Profile,
    finished_at_unix: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanCheckProgress {
    stage: &'static str,
    done: usize,
    total: usize,
    fraction: Option<f64>,
    elapsed_secs: u64,
    eta: Eta,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanCheckStatus {
    running: bool,
    has_result: bool,
    finished_at_unix: Option<u64>,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn rules() -> CmdResult<RuleSet> {
    load_with_user_dir(std::env::consts::OS, user_rules_dir().as_deref())
        .map_err(|e| CommandError::new("rules", e.to_string()))
}

fn build_evaluation(rules: &RuleSet, report: CleanReport, profile: Profile) -> CleanEvaluation {
    let greens = Selection::greens(&report);
    CleanEvaluation {
        preselected: greens.rule_ids.into_iter().collect(),
        preselected_artefacts: greens.artefacts.into_iter().collect(),
        rules: rules
            .rules
            .iter()
            .map(|r| RuleInfo {
                id: r.id.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                risk: r.risk,
                group: r.group.clone(),
                permanent: r.permanent,
            })
            .collect(),
        report,
        profile,
        finished_at_unix: now_unix(),
    }
}

/// Starts a check in the background. Progress arrives on
/// `clean://progress`, the result on `clean://done` (and stays available
/// through `clean_last`), failures on `clean://error`. Sizing caches and
/// project folders can take a minute, so the UI must not wait on this.
#[tauri::command]
pub fn clean_check_start(
    app: AppHandle,
    session: State<'_, Arc<CleanSession>>,
    profile: Profile,
) -> CmdResult<()> {
    {
        let mut running = session
            .running
            .lock()
            .map_err(|_| CommandError::new("io", "poisoned"))?;
        if *running {
            return Err(CommandError::new("scan_running", "check already running"));
        }
        *running = true;
    }
    let session = Arc::clone(&session);
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let result = (|| -> Result<CleanEvaluation, CommandError> {
            let rules = rules()?;
            let app2 = app.clone();
            let mut rule_started: Option<std::time::Instant> = None;
            let mut on_progress = |p: EvalProgress| {
                let elapsed = start.elapsed();
                let (stage, done, total, fraction, eta) = match p {
                    EvalProgress::Projects => ("projects", 0, 0, None, Eta::Indeterminate),
                    EvalProgress::Rules { done, total } => {
                        let since = rule_started.get_or_insert(std::time::Instant::now());
                        let fraction = (total > 0).then(|| done as f64 / total as f64);
                        let eta = if done == 0 || total == 0 {
                            Eta::Computing
                        } else {
                            let per = since.elapsed() / done as u32;
                            Eta::About(sapu_core::eta::round_human(per * (total - done) as u32))
                        };
                        ("rules", done, total, fraction, eta)
                    }
                };
                let _ = app2.emit(
                    "clean://progress",
                    CleanCheckProgress {
                        stage,
                        done,
                        total,
                        fraction,
                        elapsed_secs: elapsed.as_secs(),
                        eta,
                    },
                );
            };
            let report = evaluate_with_progress(
                &rules,
                profile,
                &ProtectedPaths::os_defaults(),
                &Env::system(),
                &ProjectScanOptions::default(),
                &mut on_progress,
            );
            session.set_report(report.clone());
            Ok(build_evaluation(&rules, report, profile))
        })();
        match result {
            Ok(ev) => {
                if let Ok(mut slot) = session.evaluation.lock() {
                    *slot = Some(ev.clone());
                }
                let _ = app.emit("clean://done", ev);
            }
            Err(e) => {
                let _ = app.emit("clean://error", e);
            }
        }
        if let Ok(mut running) = session.running.lock() {
            *running = false;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn clean_check_status(session: State<'_, Arc<CleanSession>>) -> CleanCheckStatus {
    let running = session.running.lock().map(|r| *r).unwrap_or(false);
    let last = session.evaluation.lock().ok().and_then(|e| e.clone());
    CleanCheckStatus {
        running,
        has_result: last.is_some(),
        finished_at_unix: last.map(|e| e.finished_at_unix),
    }
}

/// The last finished check, if any.
#[tauri::command]
pub fn clean_last(session: State<'_, Arc<CleanSession>>) -> Option<CleanEvaluation> {
    session.evaluation.lock().ok().and_then(|e| e.clone())
}

/// Runs the selection in the background. Progress on `clean://run_progress`
/// (counts, bytes, the rule being processed, ETA), the report on
/// `clean://run_done` (kept for `clean_run_last`), failures on
/// `clean://run_error`. The window can be closed meanwhile. `permanent_ok`
/// is the UI's second confirmation for rules that empty the Trash.
#[tauri::command]
pub fn clean_run_start(
    app: AppHandle,
    session: State<'_, Arc<CleanSession>>,
    rule_ids: Vec<String>,
    artefacts: Vec<std::path::PathBuf>,
    apply: bool,
    permanent_ok: bool,
) -> CmdResult<()> {
    let report = session
        .report
        .lock()
        .ok()
        .and_then(|r| r.clone())
        .ok_or_else(|| CommandError::new("no_evaluation", "evaluate first"))?;
    spawn_run(
        app,
        Arc::clone(&session),
        report,
        rule_ids,
        artefacts,
        apply,
        permanent_ok,
    )
}

/// Only the "empty the Trash" rules (every volume), evaluated on their own
/// so the Trash can be emptied from Home without a full check.
fn trash_rules() -> CmdResult<RuleSet> {
    let all = rules()?;
    Ok(RuleSet {
        rules: all
            .rules
            .into_iter()
            .filter(|r| r.group == "trash" && r.permanent)
            .collect(),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrashStatus {
    bytes: u64,
    items: usize,
    /// Trash folders that exist but cannot be read (no Full Disk Access).
    /// When not empty, `bytes` and `items` are not the whole truth.
    unreadable: Vec<std::path::PathBuf>,
}

/// How much the Trash holds right now (all volumes).
#[tauri::command]
pub async fn trash_status() -> CmdResult<TrashStatus> {
    tauri::async_runtime::spawn_blocking(|| {
        let rules = trash_rules()?;
        let report = sapu_core::cleaner::evaluate(
            &rules,
            Profile::General,
            &ProtectedPaths::os_defaults(),
            &Env::system(),
        );
        Ok(TrashStatus {
            bytes: report.findings.iter().map(|f| f.bytes).sum(),
            items: report.findings.iter().map(|f| f.items.len()).sum(),
            unreadable: report.unreadable.iter().map(|u| u.path.clone()).collect(),
        })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// Empties the Trash on every volume as a background run with progress
/// (same events as a clean run). The UI must have shown the permanent
/// deletion warning and taken the second confirmation first.
#[tauri::command]
pub async fn trash_empty_start(
    app: AppHandle,
    session: State<'_, Arc<CleanSession>>,
) -> CmdResult<()> {
    let session = Arc::clone(&session);
    let (report, ids) = tauri::async_runtime::spawn_blocking(|| -> CmdResult<_> {
        let rules = trash_rules()?;
        let report = sapu_core::cleaner::evaluate(
            &rules,
            Profile::General,
            &ProtectedPaths::os_defaults(),
            &Env::system(),
        );
        // Emptying a Trash Sapu cannot read would "finish" having removed
        // nothing while the Trash stays full. Refuse and say why instead.
        if let Some(u) = report.unreadable.first() {
            return Err(CommandError::new(
                "trash_unreadable",
                format!("{}: {}", u.path.display(), u.reason),
            ));
        }
        let ids: Vec<String> = report.findings.iter().map(|f| f.rule_id.clone()).collect();
        Ok((report, ids))
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))??;
    session.set_report(report.clone());
    spawn_run(app, session, report, ids, Vec::new(), true, true)
}

fn spawn_run(
    app: AppHandle,
    session: Arc<CleanSession>,
    report: CleanReport,
    rule_ids: Vec<String>,
    artefacts: Vec<std::path::PathBuf>,
    apply: bool,
    permanent_ok: bool,
) -> CmdResult<()> {
    {
        let mut running = session
            .run_running
            .lock()
            .map_err(|_| CommandError::new("io", "poisoned"))?;
        if *running {
            return Err(CommandError::new("scan_running", "clean already running"));
        }
        *running = true;
    }
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let result = (|| -> Result<CleanRunReport, CommandError> {
            let rules = rules()?;
            let selection = Selection {
                rule_ids: rule_ids.into_iter().collect(),
                artefacts: artefacts.into_iter().collect(),
            };
            let permanent =
                permanent_ok.then(PermanentConfirmation::i_understand_this_cannot_be_undone);
            let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
            let step = Arc::new(Mutex::new(None::<RunStep>));
            let step_for_sink = Arc::clone(&step);
            let mut est = Estimator::default();
            let app_sink = app.clone();
            let mut sink = FnSink(move |e: &ProgressEvent| {
                let elapsed = start.elapsed();
                est.observe(e, elapsed);
                let measurable = e.items_total.is_some();
                let _ = app_sink.emit(
                    "clean://run_progress",
                    CleanRunProgress {
                        items_done: e.items_done,
                        items_total: e.items_total,
                        bytes_done: e.bytes_done,
                        bytes_total: e.bytes_total,
                        fraction: measurable.then(|| est.fraction(e)),
                        step: step_for_sink.lock().ok().and_then(|s| s.clone()),
                        elapsed_secs: elapsed.as_secs(),
                        eta: est.eta(e, elapsed),
                    },
                );
            });
            let app_step = app.clone();
            let mut on_step = |s: RunStep| {
                if let Ok(mut slot) = step.lock() {
                    *slot = Some(s.clone());
                }
                let _ = app_step.emit(
                    "clean://run_progress",
                    CleanRunProgress {
                        items_done: 0,
                        items_total: None,
                        bytes_done: 0,
                        bytes_total: None,
                        fraction: None,
                        step: Some(s),
                        elapsed_secs: start.elapsed().as_secs(),
                        eta: Eta::Computing,
                    },
                );
            };
            let run = execute_with_steps(
                &report,
                &rules,
                &selection,
                apply,
                permanent,
                &safety,
                &mut sink,
                &mut on_step,
            )?;
            if apply {
                record_rate(OpKind::Clean, run.disposed as u64, start.elapsed());
                notify_done(
                    &app,
                    start.elapsed(),
                    "Sapu",
                    &format!("Pembersihan selesai: {} item", run.disposed),
                );
            }
            Ok(run)
        })();
        match result {
            Ok(run) => {
                if let Ok(mut slot) = session.last_run.lock() {
                    *slot = Some(run.clone());
                }
                let _ = app.emit("clean://run_done", run);
            }
            Err(e) => {
                let _ = app.emit("clean://run_error", e);
            }
        }
        if let Ok(mut running) = session.run_running.lock() {
            *running = false;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn clean_run_status(session: State<'_, Arc<CleanSession>>) -> CleanRunStatus {
    CleanRunStatus {
        running: session.run_running.lock().map(|r| *r).unwrap_or(false),
        has_result: session
            .last_run
            .lock()
            .map(|r| r.is_some())
            .unwrap_or(false),
    }
}

/// The last finished run, if any (cleared when a new one starts? No: kept
/// until the next run finishes, so the result survives a closed dialog).
#[tauri::command]
pub fn clean_run_last(session: State<'_, Arc<CleanSession>>) -> Option<CleanRunReport> {
    session.last_run.lock().ok().and_then(|r| r.clone())
}
