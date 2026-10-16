//! `sapu` command line interface: a thin layer over `sapu-core`.
//! Dry-run by default; `--apply` executes (S4).

mod scan;

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
use sapu_core::progress::NoProgress;
use sapu_core::safety::{
    CleanSession, DeletePlan, Disposal, FileSnapshot, HistoryStore, Outcome, PermanentConfirmation,
    ProtectedPaths, Safety,
};

#[derive(Parser)]
#[command(name = "sapu", version, about = "Smart Analyzer & Pruning Utility")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List paths Sapu will never remove.
    Protected(JsonFlag),
    /// Show or restore past clean sessions.
    History(HistoryArgs),
    /// Move files to the trash (dry-run unless --apply).
    Trash(TrashArgs),
    /// Scan a folder and report what takes up space.
    Scan(scan::ScanArgs),
}

#[derive(Args)]
struct JsonFlag {
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct HistoryArgs {
    #[command(subcommand)]
    action: Option<HistoryAction>,
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum HistoryAction {
    /// Restore every item of a session that is still in the trash.
    Restore { id: String },
}

#[derive(Args)]
struct TrashArgs {
    /// Files or directories to remove.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    /// Actually do it. Without this flag nothing is touched.
    #[arg(long)]
    apply: bool,
    /// Delete permanently instead of moving to trash. Asks for confirmation.
    #[arg(long)]
    permanent: bool,
    /// Restrict the operation to this directory (default: common ancestor).
    #[arg(long)]
    scope: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Protected(f) => protected(f.json),
        Command::History(a) => history(a),
        Command::Trash(a) => trash(a),
        Command::Scan(a) => scan::run(a),
    }
}

fn safety() -> anyhow::Result<Safety> {
    let history = HistoryStore::open_default().context("open history store")?;
    Ok(Safety::new(ProtectedPaths::os_defaults(), history))
}

fn protected(json: bool) -> anyhow::Result<()> {
    let set = ProtectedPaths::os_defaults();
    if json {
        println!("{}", serde_json::to_string_pretty(set.entries())?);
        return Ok(());
    }
    for e in set.entries() {
        println!(
            "{:<8} {:<18} {}",
            format!("{:?}", e.protection).to_lowercase(),
            e.reason,
            e.path.display()
        );
    }
    Ok(())
}

fn history(args: HistoryArgs) -> anyhow::Result<()> {
    let store = HistoryStore::open_default().context("open history store")?;
    match args.action {
        Some(HistoryAction::Restore { id }) => {
            let report = store.restore_session(&id).context("restore session")?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("restored: {}", report.restored.len());
                for p in &report.not_found_in_trash {
                    println!("not in trash: {}", p.display());
                }
            }
        }
        None => {
            let sessions = store.list().context("list history")?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else if sessions.is_empty() {
                println!("no clean sessions yet");
            } else {
                for s in &sessions {
                    println!(
                        "{}  {:<9} {:>4} removed {:>4} skipped {:>4} failed  {}  {}",
                        s.id,
                        format!("{:?}", s.disposal).to_lowercase(),
                        s.disposed,
                        s.skipped,
                        s.failed,
                        human_bytes(s.bytes_freed),
                        s.scope.display()
                    );
                }
            }
        }
    }
    Ok(())
}

fn trash(args: TrashArgs) -> anyhow::Result<()> {
    let abs: Vec<PathBuf> = args
        .paths
        .iter()
        .map(|p| std::path::absolute(p).with_context(|| format!("resolve {}", p.display())))
        .collect::<anyhow::Result<_>>()?;
    let scope = match args.scope {
        Some(s) => std::path::absolute(s)?,
        None => DeletePlan::common_scope(abs.iter().map(PathBuf::as_path))
            .context("cannot derive a common scope; pass --scope")?,
    };

    let mut plan = DeletePlan::new(&scope);
    for p in &abs {
        plan.push(FileSnapshot::capture(p).with_context(|| format!("inspect {}", p.display()))?);
    }
    if args.permanent {
        let confirmation = confirm_permanent(plan.items().len())?;
        plan = plan.with_disposal(Disposal::Permanent(confirmation));
    }
    if args.apply {
        plan = plan.apply();
    }

    let session = safety()?
        .execute(&plan, &mut NoProgress)
        .context("execute plan")?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&session)?);
    } else {
        print_session(&session);
    }
    if session.failed > 0 {
        std::process::exit(2);
    }
    Ok(())
}

/// Second confirmation for permanent deletion (S1). Refused without a TTY.
fn confirm_permanent(count: usize) -> anyhow::Result<PermanentConfirmation> {
    if !std::io::stdin().is_terminal() {
        bail!("--permanent needs an interactive terminal to confirm");
    }
    eprintln!(
        "Permanently delete {count} item(s)? This cannot be undone. Type DELETE to continue:"
    );
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim() != "DELETE" {
        bail!("aborted");
    }
    Ok(PermanentConfirmation::i_understand_this_cannot_be_undone())
}

fn print_session(s: &CleanSession) {
    for e in &s.entries {
        let (tag, note) = match &e.outcome {
            Outcome::Disposed => ("removed", String::new()),
            Outcome::WouldDispose => ("would remove", String::new()),
            Outcome::Protected { reason } => ("protected", reason.clone()),
            Outcome::Skipped { reason } => ("skipped", reason.to_string()),
            Outcome::Failed { message } => ("FAILED", message.clone()),
        };
        println!(
            "{tag:<13} {:>9}  {}  {note}",
            human_bytes(e.size),
            e.path.display()
        );
    }
    let verb = if s.dry_run { "would free" } else { "freed" };
    println!(
        "{} {} from {} item(s); {} skipped, {} failed{}",
        verb,
        human_bytes(if s.dry_run {
            s.entries
                .iter()
                .filter(|e| e.outcome == Outcome::WouldDispose)
                .map(|e| e.size)
                .sum()
        } else {
            s.bytes_freed
        }),
        s.disposed,
        s.skipped,
        s.failed,
        if s.dry_run {
            "  (dry run: pass --apply to execute)"
        } else {
            ""
        }
    );
}

pub(crate) fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}
