//! Clean commands: evaluate rules, execute a selection through core.

use std::sync::{Arc, Mutex};

use crate::support::{notify_done, record_rate};
use sapu_core::cleaner::{CleanReport, CleanRunReport, Selection, evaluate, execute};
use sapu_core::eta::OpKind;
use sapu_core::progress::NoProgress;
use sapu_core::rules::{Env, Localized, Profile, Risk, RuleSet, load_embedded};
use sapu_core::safety::{HistoryStore, PermanentConfirmation, ProtectedPaths, Safety};
use tauri::{AppHandle, State};

use crate::{CmdResult, CommandError};

#[derive(Default)]
pub struct CleanSession {
    report: Mutex<Option<CleanReport>>,
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
}

fn rules() -> CmdResult<RuleSet> {
    load_embedded(std::env::consts::OS).map_err(|e| CommandError::new("rules", e.to_string()))
}

/// Evaluate rules for `profile`. Runs on a worker thread (async command)
/// because sizing caches can take seconds.
#[tauri::command]
pub async fn clean_evaluate(
    session: State<'_, Arc<CleanSession>>,
    profile: Profile,
) -> CmdResult<CleanEvaluation> {
    let session = Arc::clone(&session);
    tauri::async_runtime::spawn_blocking(move || {
        let rules = rules()?;
        let report = evaluate(
            &rules,
            profile,
            &ProtectedPaths::os_defaults(),
            &Env::system(),
        );
        let greens = Selection::greens(&report);
        let preselected = greens.rule_ids.into_iter().collect();
        let preselected_artefacts = greens.artefacts.into_iter().collect();
        let infos = rules
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
            .collect();
        if let Ok(mut r) = session.report.lock() {
            *r = Some(report.clone());
        }
        Ok(CleanEvaluation {
            report,
            rules: infos,
            preselected,
            preselected_artefacts,
        })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
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
