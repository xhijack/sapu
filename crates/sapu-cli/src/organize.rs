//! `sapu organize`: propose groups for a folder, apply them, undo a run.

use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use sapu_core::organize::{
    FolderListing, ItemAdvice, OrganizePlan, OrganizeStore, Unit, apply, build_move_plan,
    default_selection, plan_from_advice, propose, scan_folder, undo,
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
        advise_with_ai(&listing, &mut plan);
    }
    // Only sure groups that move files alone; folders, projects and apps
    // are never moved without the person ticking them. Renames are printed.
    let selection = default_selection(&plan);
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
    let suggestions = plan
        .groups
        .iter()
        .flat_map(|g| g.entries.iter())
        .chain(plan.unplaced.iter());
    for e in suggestions {
        if let Some(n) = &e.suggested_name {
            println!("  rename? {} -> {}", e.name, n);
        }
        if let Some(why) = &e.junk {
            println!("  junk?   {} ({why})", e.name);
        }
    }
    if !plan.unplaced.is_empty() {
        println!("  {} item(s) left where they are", plan.unplaced.len());
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

/// Asks the configured AI where each item belongs. Any failure keeps the
/// heuristic plan; the AI only ever sees names, sizes and folder listings.
fn advise_with_ai(listing: &FolderListing, plan: &mut OrganizePlan) {
    use sapu_ai::tasks::tidy_items::{TidyItem, TidyItems, TidyItemsInput};
    let client = match crate::ai::client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("AI not configured, using heuristic groups: {e}");
            return;
        }
    };
    let language = sapu_core::settings::Settings::default_path()
        .and_then(|p| sapu_core::settings::Settings::load(&p))
        .ok()
        .and_then(|s| s.locale)
        .unwrap_or_else(|| "id".into());
    let input = TidyItemsInput {
        language,
        folder_name: listing
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        items: listing
            .entries
            .iter()
            .filter(|e| e.unit != Unit::Link)
            .take(250)
            .map(|e| TidyItem {
                name: e.name.clone(),
                unit: label(&e.unit),
                kind: label(&e.kind),
                year: e
                    .mtime
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| 1970 + (d.as_secs() / 31_557_600) as i32),
                size_kb: e.bytes / 1024,
                children: e.children.clone(),
            })
            .collect(),
    };
    match client.run::<TidyItems>(&input) {
        Ok(answer) => {
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
                Some(p) => *plan = p,
                None => eprintln!("AI answer matched no item, using heuristic groups"),
            }
        }
        Err(e) => eprintln!("AI unavailable, using heuristic groups: {e}"),
    }
}

/// The serde name of a unit or kind ("file", "document", ...).
fn label<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}
