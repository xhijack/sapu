//! `sapu clean`: thin presentation over `sapu_core::cleaner`.

use std::io::IsTerminal;

use anyhow::{Context, bail};
use clap::{Args, ValueEnum};
use sapu_core::cleaner::{CleanReport, RuleOutcome, Selection, evaluate, execute};
use sapu_core::progress::NoProgress;
use sapu_core::rules::{Env, Profile, Risk, load_embedded};
use sapu_core::safety::{HistoryStore, PermanentConfirmation, ProtectedPaths, Safety};

use crate::human_bytes;

#[derive(Clone, Copy, ValueEnum)]
enum ProfileArg {
    General,
    Dev,
}

#[derive(Args)]
pub struct CleanArgs {
    /// Which rules to consider.
    #[arg(long, value_enum, default_value_t = ProfileArg::General)]
    profile: ProfileArg,
    /// Actually clean. Without this flag nothing is touched.
    #[arg(long)]
    apply: bool,
    /// Also run yellow ("check first") rules, not only green ones.
    #[arg(long)]
    include_yellow: bool,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

pub fn run(args: CleanArgs) -> anyhow::Result<()> {
    let profile = match args.profile {
        ProfileArg::General => Profile::General,
        ProfileArg::Dev => Profile::Developer,
    };
    let rules = load_embedded(std::env::consts::OS).context("load rules")?;
    let protected = ProtectedPaths::os_defaults();
    let report = evaluate(&rules, profile, &protected, &Env::system());

    let mut selection = Selection::greens(&report);
    if args.include_yellow {
        for f in &report.findings {
            selection.rule_ids.insert(f.rule_id.clone());
        }
        for c in report.commands.iter().filter(|c| c.available) {
            selection.rule_ids.insert(c.rule_id.clone());
        }
        for a in report.projects.iter().flat_map(|p| p.artefacts.iter()) {
            selection.artefacts.insert(a.path.clone());
        }
    }
    let needs_permanent = report
        .findings
        .iter()
        .any(|f| f.permanent && selection.contains(&f.rule_id));
    let permanent = if args.apply && needs_permanent {
        Some(confirm_permanent()?)
    } else {
        None
    };

    let safety = Safety::new(protected, HistoryStore::open_default().context("history")?);
    let run = execute(
        &report,
        &rules,
        &selection,
        args.apply,
        permanent,
        &safety,
        &mut NoProgress,
    )
    .context("clean")?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({ "report": report, "selection": selection, "run": run })
            )?
        );
        return Ok(());
    }
    print_report(&report, &selection, &rules);
    println!();
    for o in &run.outcomes {
        match o {
            RuleOutcome::Files { rule_id, session } => println!(
                "  {:<28} {} {}, {} skipped, {} failed",
                rule_id,
                if session.dry_run {
                    "would remove"
                } else {
                    "removed"
                },
                session.disposed,
                session.skipped,
                session.failed
            ),
            RuleOutcome::Command { rule_id, outcome } => println!(
                "  {:<28} {} `{}`",
                rule_id,
                if outcome.success { "ran" } else { "FAILED" },
                outcome.command_line
            ),
            RuleOutcome::Skipped { rule_id, reason } => {
                println!("  {rule_id:<28} skipped ({reason})")
            }
        }
    }
    println!(
        "{} {}{}",
        if run.dry_run { "would free" } else { "freed" },
        human_bytes(if run.dry_run {
            selected_bytes(&report, &selection)
        } else {
            run.bytes_freed
        }),
        if run.dry_run {
            "  (dry run: pass --apply to execute)"
        } else {
            ""
        }
    );
    if run.failed > 0 {
        std::process::exit(2);
    }
    Ok(())
}

fn selected_bytes(report: &CleanReport, sel: &Selection) -> u64 {
    report
        .findings
        .iter()
        .filter(|f| sel.contains(&f.rule_id))
        .map(|f| f.bytes)
        .sum::<u64>()
        + report
            .commands
            .iter()
            .filter(|c| sel.contains(&c.rule_id))
            .filter_map(|c| c.estimate_bytes)
            .sum::<u64>()
}

fn print_report(report: &CleanReport, sel: &Selection, rules: &sapu_core::rules::RuleSet) {
    let name = |id: &str| {
        rules
            .get(id)
            .map(|r| r.name.en.clone())
            .unwrap_or_else(|| id.to_string())
    };
    // One line per rule: findings are per match in core.
    let mut seen = Vec::new();
    for f in &report.findings {
        if seen.contains(&&f.rule_id) {
            continue;
        }
        seen.push(&f.rule_id);
        let same: Vec<_> = report
            .findings
            .iter()
            .filter(|x| x.rule_id == f.rule_id)
            .collect();
        let bytes: u64 = same.iter().map(|x| x.bytes).sum();
        let items: usize = same.iter().map(|x| x.items.len()).sum();
        let prot: u64 = same.iter().map(|x| x.skipped_protected).sum();
        println!(
            "{} {:<7} {:>9}  {:<34} {} items{}",
            if sel.contains(&f.rule_id) {
                "[x]"
            } else {
                "[ ]"
            },
            risk(f.risk),
            human_bytes(bytes),
            name(&f.rule_id),
            items,
            if prot > 0 {
                format!(", {prot} protected skipped")
            } else {
                String::new()
            }
        );
    }
    for c in &report.commands {
        println!(
            "{} {:<7} {:>9}  {:<34} {}{}",
            if sel.contains(&c.rule_id) {
                "[x]"
            } else {
                "[ ]"
            },
            risk(c.risk),
            c.estimate_bytes
                .map(human_bytes)
                .unwrap_or_else(|| "?".into()),
            name(&c.rule_id),
            c.command_line,
            if c.available { "" } else { "  (not installed)" }
        );
    }
    if !report.projects.is_empty() {
        println!("\nDevelopment projects");
        for p in &report.projects {
            println!(
                "  {:<34} {:<7} idle {:>4} d  {:>9}  rebuild: {}",
                p.name,
                format!("{:?}", p.ecosystem).to_lowercase(),
                p.idle_days
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "?".into()),
                human_bytes(p.bytes),
                p.rebuild.join(" && ")
            );
            for a in &p.artefacts {
                println!(
                    "    {} {:<7} {:>9}  {}{}",
                    if sel.artefacts.contains(&a.path) {
                        "[x]"
                    } else {
                        "[ ]"
                    },
                    risk(a.risk),
                    human_bytes(a.bytes),
                    a.kind,
                    match (a.gitignored, a.tracked) {
                        (true, Some(false)) => "  (gitignored, untracked)",
                        (true, None) => "  (gitignored)",
                        (false, _) => "  (NOT gitignored)",
                        _ => "",
                    }
                );
            }
        }
    }
    println!(
        "\ngreen total {}   yellow total {}",
        human_bytes(report.bytes_by_risk(Risk::Green)),
        human_bytes(report.bytes_by_risk(Risk::Yellow))
    );
}

fn risk(r: Risk) -> &'static str {
    match r {
        Risk::Green => "green",
        Risk::Yellow => "yellow",
    }
}

fn confirm_permanent() -> anyhow::Result<PermanentConfirmation> {
    if !std::io::stdin().is_terminal() {
        bail!("emptying the Trash needs an interactive terminal to confirm");
    }
    eprintln!("Some selected rules delete permanently (Trash contents). Type DELETE to continue:");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim() != "DELETE" {
        bail!("aborted");
    }
    Ok(PermanentConfirmation::i_understand_this_cannot_be_undone())
}
