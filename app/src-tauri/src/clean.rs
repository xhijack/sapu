//! Clean commands: evaluate rules, execute a selection through core.

use std::sync::{Arc, Mutex};

use crate::support::{notify_done, record_rate};
use sapu_core::cleaner::{
    CleanReport, CleanRunReport, EvalProgress, ProjectScanOptions, Selection,
    evaluate_with_progress, execute,
};
use sapu_core::eta::{Eta, OpKind};
use sapu_core::progress::NoProgress;
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

/// Execute `rule_ids` from the last evaluation. `permanent_ok` is the UI's
/// second confirmation for rules that empty the Trash.
#[tauri::command]
pub async fn clean_execute(
    app: AppHandle,
    session: State<'_, Arc<CleanSession>>,
    rule_ids: Vec<String>,
    artefacts: Vec<std::path::PathBuf>,
    apply: bool,
    permanent_ok: bool,
) -> CmdResult<CleanRunReport> {
    let report = session
        .report
        .lock()
        .ok()
        .and_then(|r| r.clone())
        .ok_or_else(|| CommandError::new("no_evaluation", "evaluate first"))?;
    tauri::async_runtime::spawn_blocking(move || {
        let rules = rules()?;
        let selection = Selection {
            rule_ids: rule_ids.into_iter().collect(),
            artefacts: artefacts.into_iter().collect(),
        };
        let permanent =
            permanent_ok.then(PermanentConfirmation::i_understand_this_cannot_be_undone);
        let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
        let start = std::time::Instant::now();
        let run = execute(
            &report,
            &rules,
            &selection,
            apply,
            permanent,
            &safety,
            &mut NoProgress,
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
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}
