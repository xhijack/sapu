//! "Rawat": one command that scans the home folder, evaluates the clean
//! rules and scores the result. Nothing is removed here; the UI calls
//! `clean_execute` afterwards with the pre-selected green rules.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use sapu_core::care::{SpaceScore, score};
use sapu_core::cleaner::{Selection, evaluate};
use sapu_core::eta::{Estimator, Eta, OpKind};
use sapu_core::progress::{FnSink, ProgressEvent};
use sapu_core::rules::{Env, Profile, load_with_user_dir, user_rules_dir};
use sapu_core::safety::ProtectedPaths;
use sapu_core::scanner::{ScanControl, ScanOptions, ScanStats, ScanSummary, scan};
use sapu_core::settings::Settings;
use tauri::{AppHandle, Emitter, State};

use crate::support::{notify_done, record_rate};
use crate::{CmdResult, CommandError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CareReport {
    pub score: SpaceScore,
    pub scan: ScanSummary,
    pub preselected: Vec<String>,
    pub preselected_artefacts: Vec<std::path::PathBuf>,
    pub finished_at_unix: u64,
}

#[derive(Default)]
pub struct CareSession {
    last: Mutex<Option<CareReport>>,
    running: Mutex<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CareProgress {
    stage: u8,
    total: u8,
    fraction: Option<f64>,
    items_done: u64,
    bytes_done: u64,
    elapsed_secs: u64,
    eta: Eta,
}

#[tauri::command]
pub fn care_last(session: State<'_, Arc<CareSession>>) -> Option<CareReport> {
    session.last.lock().ok().and_then(|l| l.clone())
}

#[tauri::command]
pub fn care_run(
    app: AppHandle,
    session: State<'_, Arc<CareSession>>,
    scans: State<'_, Arc<crate::scan::ScanSession>>,
    cleans: State<'_, Arc<crate::clean::CleanSession>>,
) -> CmdResult<()> {
    {
        let mut running = session
            .running
            .lock()
            .map_err(|_| CommandError::new("io", "poisoned"))?;
        if *running {
            return Err(CommandError::new("scan_running", "care already running"));
        }
        *running = true;
    }
    let root = dirs::home_dir().ok_or_else(|| CommandError::new("io", "no home dir"))?;
    let session = Arc::clone(&session);
    let scans = Arc::clone(&scans);
    let cleans = Arc::clone(&cleans);
    std::thread::spawn(move || {
        let start = Instant::now();
        let result = (|| -> Result<CareReport, CommandError> {
            // Stage 1: scan.
            let stats_dir = ScanStats::default_dir().ok();
            let opts = ScanOptions::new(&root, stats_dir.as_deref());
            let control = ScanControl::new();
            let mut est = Estimator::default();
            let app2 = app.clone();
            let mut sink = FnSink(move |e: &ProgressEvent| {
                let elapsed = start.elapsed();
                est.observe(e, elapsed);
                let measurable = e.bytes_total.is_some() || e.items_total.is_some();
                let _ = app2.emit(
                    "care://progress",
                    CareProgress {
                        stage: 1,
                        total: 2,
                        fraction: measurable.then(|| est.fraction(e)),
                        items_done: e.items_done,
                        bytes_done: e.bytes_done,
                        elapsed_secs: elapsed.as_secs(),
                        eta: est.eta(e, elapsed),
                    },
                );
            });
            let scanned = scan(&opts, &control, &mut sink)?;
            if let Some(dir) = &stats_dir {
                let _ = ScanStats::from_result(&scanned).save(dir, &scanned.root);
            }
            record_rate(OpKind::Scan, scanned.total_bytes, start.elapsed());
            let summary = scanned.summary();
            scans.set_result(scanned.clone());

            // Stage 2: evaluate rules for the user's profile.
            let _ = app.emit(
                "care://progress",
                CareProgress {
                    stage: 2,
                    total: 2,
                    fraction: None,
                    items_done: 0,
                    bytes_done: 0,
                    elapsed_secs: start.elapsed().as_secs(),
                    eta: Eta::Indeterminate,
                },
            );
            let profile = Settings::default_path()
                .and_then(|p| Settings::load(&p))
                .map(|s| match s.profile {
                    sapu_core::settings::Profile::Developer => Profile::Developer,
                    sapu_core::settings::Profile::General => Profile::General,
                })
                .unwrap_or(Profile::General);
            let rules = load_with_user_dir(std::env::consts::OS, user_rules_dir().as_deref())
                .map_err(|e| CommandError::new("rules", e.to_string()))?;
            let report = evaluate(
                &rules,
                profile,
                &ProtectedPaths::os_defaults(),
                &Env::system(),
            );
            let greens = Selection::greens(&report);
            cleans.set_report(report.clone());

            Ok(CareReport {
                score: score(&scanned, &report),
                scan: summary,
                preselected: greens.rule_ids.into_iter().collect(),
                preselected_artefacts: greens.artefacts.into_iter().collect(),
                finished_at_unix: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            })
        })();
        match result {
            Ok(report) => {
                if let Ok(mut l) = session.last.lock() {
                    *l = Some(report.clone());
                }
                notify_done(
                    &app,
                    start.elapsed(),
                    "Sapu",
                    &format!("Rawat selesai: Skor Ruang {}", report.score.score),
                );
                let _ = app.emit("care://done", report);
            }
            Err(e) => {
                let _ = app.emit("care://error", e);
            }
        }
        if let Ok(mut running) = session.running.lock() {
            *running = false;
        }
    });
    Ok(())
}
