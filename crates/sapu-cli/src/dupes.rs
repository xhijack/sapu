//! `sapu dupes`: thin presentation over `sapu_core::dedupe`.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use clap::{Args, ValueEnum};
use sapu_core::dedupe::{
    DedupeOptions, DedupePlan, KeepStrategy, ReplaceMode, Stage, execute, find_duplicates,
    folder_pairs,
};
use sapu_core::progress::{FnSink, NoProgress, ProgressEvent};
use sapu_core::safety::{HistoryStore, ProtectedPaths, Safety};
use sapu_core::scanner::ScanControl;

use crate::human_bytes;

#[derive(Clone, Copy, ValueEnum)]
enum KeepArg {
    Oldest,
    Newest,
    Original,
}

#[derive(Args)]
pub struct DupesArgs {
    /// Directory to search.
    path: PathBuf,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
    /// Ignore files smaller than this many bytes.
    #[arg(long, default_value_t = 4096)]
    min_size: u64,
    /// Move duplicates to the trash (one copy per group is always kept).
    #[arg(long)]
    apply: bool,
    /// Which copy to keep.
    #[arg(long, value_enum, default_value_t = KeepArg::Original)]
    keep: KeepArg,
    /// How many groups to print.
    #[arg(long, default_value_t = 30)]
    top: usize,
    /// After trashing a duplicate, put a hardlink to the kept file at its path.
    #[arg(long)]
    hardlink: bool,
}

pub fn run(args: DupesArgs) -> anyhow::Result<()> {
    let mut opts = DedupeOptions::new(&args.path);
    opts.min_size = args.min_size;
    let control = ScanControl::new();
    let interactive = std::io::stderr().is_terminal() && !args.json;

    let result = if interactive {
        let start = Instant::now();
        let stage = std::cell::Cell::new(Stage {
            current: 1,
            total: 3,
        });
        let mut sink = FnSink(|e: &ProgressEvent| {
            let pct = match (e.bytes_total, e.items_total) {
                (Some(t), _) if t > 0 => format!("{:>3}%", e.bytes_done * 100 / t),
                (_, Some(t)) if t > 0 => format!("{:>3}%", e.items_done * 100 / t),
                _ => "  --".into(),
            };
            let st = stage.get();
            eprint!(
                "\rstep {}/{}  {pct}  {:>9} items  {:>5}s   ",
                st.current,
                st.total,
                e.items_done,
                start.elapsed().as_secs()
            );
            let _ = std::io::stderr().flush();
        });
        let r = find_duplicates(&opts, &control, &mut sink, |s| stage.set(s));
        eprintln!();
        r
    } else {
        find_duplicates(&opts, &control, &mut NoProgress, |_| {})
    }
    .context("dedupe")?;

    let strategy = match args.keep {
        KeepArg::Oldest => KeepStrategy::Oldest,
        KeepArg::Newest => KeepStrategy::Newest,
        KeepArg::Original => KeepStrategy::OriginalName,
    };
    let plan = DedupePlan::build(&result.summary.root, &result.groups, &strategy, |_| None)
        .context("plan")?;
    let pairs = folder_pairs(&result.groups);

    let run = if args.apply {
        let safety = Safety::new(
            ProtectedPaths::os_defaults(),
            HistoryStore::open_default().context("history")?,
        );
        Some(
            execute(
                &plan,
                &safety,
                true,
                if args.hardlink {
                    ReplaceMode::Hardlink
                } else {
                    ReplaceMode::Trash
                },
                &mut NoProgress,
            )
            .context("execute")?,
        )
    } else {
        None
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "summary": result.summary,
                "groups": result.groups.iter().take(args.top).collect::<Vec<_>>(),
                "folder_pairs": pairs,
                "plan": plan,
                "run": run,
            }))?
        );
        return Ok(());
    }

    let s = &result.summary;
    println!(
        "{} recoverable in {} groups ({} duplicate files of {} seen){}",
        human_bytes(s.wasted_bytes),
        s.groups,
        s.duplicate_files,
        s.files_seen,
        if s.cancelled {
            "  (cancelled, partial)"
        } else {
            ""
        }
    );
    if !pairs.is_empty() {
        println!("\nFolders sharing files");
        for p in pairs.iter().take(10) {
            println!(
                "  {:>9}  {} files  {}  <->  {}",
                human_bytes(p.bytes),
                p.files,
                p.a.display(),
                p.b.display()
            );
        }
    }
    println!("\nGroups (largest first)");
    for (g, gp) in result.groups.iter().zip(plan.groups()).take(args.top) {
        println!(
            "  {:>9} x{}  keep {}",
            human_bytes(g.size),
            g.files.len(),
            gp.keep.display()
        );
        for p in &gp.remove {
            println!(
                "             {} {}",
                if args.apply {
                    "removed"
                } else {
                    "would remove"
                },
                p.display()
            );
        }
    }
    match run {
        Some(r) => println!(
            "\nfreed {}; {} removed, {} skipped, {} failed",
            human_bytes(r.bytes_freed),
            r.disposed,
            r.skipped,
            r.failed
        ),
        None => println!(
            "\nwould free {}  (dry run: pass --apply to execute)",
            human_bytes(plan.total_bytes())
        ),
    }
    Ok(())
}
