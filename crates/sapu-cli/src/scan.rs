//! `sapu scan`: thin presentation over `sapu_core::scanner`.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::Args;
use sapu_core::eta::{Estimator, Eta};
use sapu_core::progress::{FnSink, NoProgress, ProgressEvent};
use sapu_core::scanner::{FileFilter, ScanControl, ScanOptions, ScanStats, scan};

use crate::human_bytes;

#[derive(Args)]
pub struct ScanArgs {
    /// Directory to scan.
    path: PathBuf,
    /// Machine-readable summary.
    #[arg(long)]
    json: bool,
    /// How many largest files and folders to list.
    #[arg(long, default_value_t = 20)]
    top: usize,
}

pub fn run(args: ScanArgs) -> anyhow::Result<()> {
    let stats_dir = ScanStats::default_dir().ok();
    let opts = ScanOptions::new(&args.path, stats_dir.as_deref());
    let control = ScanControl::new();
    let interactive = std::io::stderr().is_terminal() && !args.json;

    let result = if interactive {
        let start = Instant::now();
        let mut est = Estimator::default();
        let mut sink = FnSink(|e: &ProgressEvent| {
            let elapsed = start.elapsed();
            est.observe(e, elapsed);
            let frac = est.fraction(e);
            let eta = est.eta(e, elapsed);
            let pct = if e.bytes_total.is_some() || e.items_total.is_some() {
                format!("{:>3}%", (frac * 100.0).floor() as u64)
            } else {
                "  --".to_string()
            };
            eprint!(
                "\r{pct}  {:>9}  {:>9} files  {:<28}",
                human_bytes(e.bytes_done),
                e.items_done,
                eta_text(eta)
            );
            let _ = std::io::stderr().flush();
        });
        let r = scan(&opts, &control, &mut sink);
        eprintln!();
        r
    } else {
        scan(&opts, &control, &mut NoProgress)
    }
    .context("scan")?;

    if let Some(dir) = &stats_dir {
        let _ = ScanStats {
            total_bytes: result.total_bytes,
            file_count: result.file_count,
            finished_at_unix: result.finished_at_unix,
        }
        .save(dir, &result.root);
    }

    let summary = result.summary();
    let top_files = result.top_files(&FileFilter::default(), args.top);
    let top_dirs = result.top_dirs(args.top);

    if args.json {
        let out = serde_json::json!({
            "summary": summary,
            "top_files": top_files,
            "top_dirs": top_dirs,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "{}  {} files in {} folders{}",
        human_bytes(summary.total_bytes),
        summary.file_count,
        summary.dir_count,
        if summary.denied_count > 0 {
            format!("  ({} folders could not be read)", summary.denied_count)
        } else {
            String::new()
        }
    );
    println!("\nBy category");
    for c in &summary.categories {
        if c.bytes == 0 {
            continue;
        }
        let share = c.bytes as f64 / summary.total_bytes.max(1) as f64;
        println!(
            "  {:<12} {:>9}  {:>5.1}%  {}",
            format!("{:?}", c.category).to_lowercase(),
            human_bytes(c.bytes),
            share * 100.0,
            bar(share)
        );
    }
    println!("\nLargest folders");
    for d in &top_dirs {
        println!("  {:>9}  {}", human_bytes(d.bytes), d.path.display());
    }
    println!("\nLargest files");
    for f in &top_files {
        println!("  {:>9}  {}", human_bytes(f.size), f.path.display());
    }
    Ok(())
}

fn bar(share: f64) -> String {
    let n = (share * 30.0).round() as usize;
    "█".repeat(n)
}

fn eta_text(eta: Eta) -> String {
    match eta {
        Eta::Indeterminate => "working...".into(),
        Eta::Computing => "estimating...".into(),
        Eta::AlmostDone => "almost done".into(),
        Eta::Range { low, high } => format!("{}–{} left", short(low), short(high)),
        Eta::About(d) if d < Duration::from_secs(60) => "under a minute left".into(),
        Eta::About(d) => format!("about {} left", short(d)),
    }
}

fn short(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{} h", s.div_ceil(3600))
    } else if s >= 60 {
        format!("{} min", s.div_ceil(60))
    } else {
        format!("{s} s")
    }
}
