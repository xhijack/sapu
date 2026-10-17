//! `sapu organize`: propose groups for a folder, apply them, undo a run.

use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use sapu_core::organize::{
    Naming, OrganizeStore, Selection, apply, apply_naming, build_move_plan, propose, scan_folder,
    undo,
};
use sapu_core::progress::NoProgress;
use sapu_core::safety::{HistoryStore, ProtectedPaths, Safety};

use crate::human_bytes;

#[derive(Args)]
pub struct OrganizeArgs {
    /// Folder to tidy (its top level only).
    path: Option<PathBuf>,
    /// Move the files. Without it, only the plan is printed.
    #[arg(long)]
    apply: bool,
    /// Ask the configured AI to name and merge groups (needs `sapu ai` set up).
    #[arg(long)]
    ai: bool,
    /// Undo a previous run by session id (see `sapu organize --list`).
    #[arg(long, conflicts_with_all = ["apply", "ai"])]
    undo: Option<String>,
    /// List previous runs.
    #[arg(long, conflicts_with_all = ["apply", "ai", "undo"])]
    list: bool,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

pub fn run(a: OrganizeArgs) -> anyhow::Result<()> {
    let store = OrganizeStore::open_default()?;
    if a.list {
        for s in store.list()? {
            println!(
                "{}  {}  {} moved, {} skipped, {} failed{}  {}",
                s.id,
                s.started_at_unix,
                s.moved,
                s.skipped,
                s.failed,
                if s.undone { ", undone" } else { "" },
                s.root.display()
            );
        }
        return Ok(());
    }
    if let Some(id) = a.undo {
        let session = store
            .get(&id)?
            .with_context(|| format!("no organize session {id}"))?;
        let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
        let r = undo(&session, &store, &safety, &mut NoProgress)?;
        println!(
            "restored {}, skipped {}, failed {}",
            r.restored, r.skipped, r.failed
        );
        return Ok(());
    }
    let root = a.path.context("a folder is required")?;
    let listing = scan_folder(&root)?;
    let mut plan = propose(&listing);
    if a.ai {
        match crate::ai::client() {
            Ok(client) => {
                let input = sapu_ai::tasks::tidy::TidyInput {
                    language: "en".into(),
                    folder_name: root
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    candidates: plan
                        .groups
                        .iter()
                        .map(|g| sapu_ai::tasks::tidy::TidyCandidate {
                            id: g.id.clone(),
                            name: g.name.clone(),
                            source: format!("{:?}", g.source).to_lowercase(),
                            count: g.entries.len(),
                            examples: g.entries.iter().take(12).map(|e| e.name.clone()).collect(),
                        })
                        .collect(),
                };
                match client.run::<sapu_ai::tasks::tidy::Tidy>(&input) {
                    Ok(answer) => {
                        let names: Vec<Naming> = answer
                            .groups
                            .into_iter()
                            .map(|n| Naming {
                                id: n.id,
                                name: n.name,
                                confidence: n.confidence,
                            })
                            .collect();
                        apply_naming(&mut plan, &names, &answer.merge);
                    }
                    Err(e) => eprintln!("AI unavailable, using heuristic names: {e}"),
                }
            }
            Err(e) => eprintln!("AI not configured, using heuristic names: {e}"),
        }
    }
    let selection = Selection {
        group_ids: plan.groups.iter().map(|g| g.id.clone()).collect(),
        ..Default::default()
    };
    let moves = build_move_plan(&plan, &selection, &ProtectedPaths::os_defaults())?;
    if a.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "plan": plan, "moves": moves }))?
        );
        return Ok(());
    }
    println!("{}", root.display());
    for g in &plan.groups {
        println!(
            "  {:<28} {:>3} item  {:>9}  [{:?}, {:?}]",
            g.name,
            g.entries.len(),
            human_bytes(g.bytes),
            g.source,
            g.confidence
        );
    }
    println!(
        "{} move(s), {} skipped",
        moves.moves.len(),
        moves.skipped.len()
    );
    let report = apply(&moves, !a.apply, &store, &mut NoProgress)?;
    if a.apply {
        println!(
            "moved {}, skipped {}, failed {}; session {} (undo with --undo)",
            report.session.moved, report.session.skipped, report.session.failed, report.session.id
        );
        for (p, why) in report.failures {
            println!("  ! {}: {why}", p.display());
        }
    } else {
        println!(
            "dry run: would move {} item(s). Add --apply to move them.",
            report.session.moved
        );
    }
    Ok(())
}
