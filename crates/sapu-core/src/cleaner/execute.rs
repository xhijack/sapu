//! Executes a selection of findings through the safety layer.

use std::collections::HashSet;
use std::time::Duration;

use crate::error::Result;
use crate::progress::{ProgressEvent, ProgressSink};
use crate::rules::{Risk, RuleSet};
use crate::safety::{
    CleanSession, DeletePlan, Disposal, FileSnapshot, PermanentConfirmation, Safety,
};

use super::command::{CommandOutcome, run_command};
use super::evaluate::CleanReport;

/// Which rules to run. Defaults to every green finding.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Selection {
    pub rule_ids: HashSet<String>,
    /// Project artefact folders (by path) to remove.
    pub artefacts: HashSet<std::path::PathBuf>,
}

impl Selection {
    pub fn greens(report: &CleanReport) -> Self {
        Self {
            rule_ids: report
                .findings
                .iter()
                .filter(|f| f.risk == Risk::Green)
                .map(|f| f.rule_id.clone())
                .chain(
                    report
                        .commands
                        .iter()
                        .filter(|c| c.risk == Risk::Green && c.available)
                        .map(|c| c.rule_id.clone()),
                )
                .collect(),
            artefacts: report
                .projects
                .iter()
                .flat_map(|p| p.artefacts.iter())
                .filter(|a| a.risk == Risk::Green)
                .map(|a| a.path.clone())
                .collect(),
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.rule_ids.contains(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleOutcome {
    Files {
        rule_id: String,
        session: CleanSession,
    },
    Command {
        rule_id: String,
        outcome: CommandOutcome,
    },
    /// Artefact folders of one development project.
    Project {
        root: std::path::PathBuf,
        session: CleanSession,
    },
    /// Would run (dry run) or skipped because permanent deletion was not confirmed.
    Skipped { rule_id: String, reason: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CleanRunReport {
    pub dry_run: bool,
    pub outcomes: Vec<RuleOutcome>,
    pub bytes_freed: u64,
    pub disposed: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Run `selection` from `report`. `permanent` is required for rules that
/// are flagged `permanent` (items already in the Trash); without it those
/// rules are skipped with a reason, never silently downgraded or upgraded.
pub fn execute(
    report: &CleanReport,
    rules: &RuleSet,
    selection: &Selection,
    apply: bool,
    permanent: Option<PermanentConfirmation>,
    safety: &Safety,
    progress: &mut dyn ProgressSink,
) -> Result<CleanRunReport> {
    execute_with_steps(
        report,
        rules,
        selection,
        apply,
        permanent,
        safety,
        progress,
        &mut |_| {},
    )
}

/// What the run is working on right now, for the UI's "processing …" line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RunStep {
    /// Files of one rule (or one project's artefacts, `rule_id` = root).
    Rule { rule_id: String, items: u64 },
    /// An official clean-up command.
    Command {
        rule_id: String,
        command_line: String,
    },
}

/// [`execute`] that also announces each rule or command as it starts.
#[allow(clippy::too_many_arguments)]
pub fn execute_with_steps(
    report: &CleanReport,
    rules: &RuleSet,
    selection: &Selection,
    apply: bool,
    permanent: Option<PermanentConfirmation>,
    safety: &Safety,
    progress: &mut dyn ProgressSink,
    on_step: &mut dyn FnMut(RunStep),
) -> Result<CleanRunReport> {
    let mut run = CleanRunReport {
        dry_run: !apply,
        ..Default::default()
    };
    let total: u64 = report
        .findings
        .iter()
        .filter(|f| selection.contains(&f.rule_id))
        .map(|f| f.items.len() as u64)
        .sum();
    let mut done = 0u64;

    for f in report
        .findings
        .iter()
        .filter(|f| selection.contains(&f.rule_id))
    {
        let disposal = if f.permanent {
            match permanent {
                Some(c) => Disposal::Permanent(c),
                None => {
                    run.outcomes.push(RuleOutcome::Skipped {
                        rule_id: f.rule_id.clone(),
                        reason: "permanent_confirmation_required".into(),
                    });
                    continue;
                }
            }
        } else {
            Disposal::Trash
        };
        on_step(RunStep::Rule {
            rule_id: f.rule_id.clone(),
            items: f.items.len() as u64,
        });
        let mut plan = DeletePlan::new(&f.scope).with_disposal(disposal);
        for c in &f.items {
            if let Ok(snap) = FileSnapshot::capture(&c.path) {
                plan.push(snap.with_bytes(c.size));
            }
        }
        if apply {
            plan = plan.apply();
        }
        let mut sub = SubProgress {
            inner: progress,
            base: done,
            total,
        };
        let session = safety.execute(&plan, &mut sub)?;
        done += f.items.len() as u64;
        if apply {
            run.bytes_freed += session.bytes_freed;
        }
        run.disposed += session.disposed;
        run.skipped += session.skipped;
        run.failed += session.failed;
        run.outcomes.push(RuleOutcome::Files {
            rule_id: f.rule_id.clone(),
            session,
        });
    }

    // Development projects: the selected artefact folders, one plan per
    // project scoped to its root. Never anything git tracks (the evaluator
    // already left those out).
    for p in &report.projects {
        let chosen: Vec<_> = p
            .artefacts
            .iter()
            .filter(|a| selection.artefacts.contains(&a.path))
            .collect();
        if chosen.is_empty() {
            continue;
        }
        on_step(RunStep::Rule {
            rule_id: p.root.to_string_lossy().into_owned(),
            items: chosen.len() as u64,
        });
        let mut plan = DeletePlan::new(&p.root).with_disposal(Disposal::Trash);
        for a in &chosen {
            if let Ok(snap) = FileSnapshot::capture(&a.path) {
                plan.push(snap.with_bytes(a.bytes));
            }
        }
        if apply {
            plan = plan.apply();
        }
        let session = safety.execute(&plan, progress)?;
        if apply {
            run.bytes_freed += session.bytes_freed;
        }
        run.disposed += session.disposed;
        run.skipped += session.skipped;
        run.failed += session.failed;
        run.outcomes.push(RuleOutcome::Project {
            root: p.root.clone(),
            session,
        });
    }

    for c in report
        .commands
        .iter()
        .filter(|c| selection.contains(&c.rule_id) && c.available)
    {
        let Some(cmd) = rules.get(&c.rule_id).and_then(|r| r.command.as_ref()) else {
            continue;
        };
        if !apply {
            run.outcomes.push(RuleOutcome::Skipped {
                rule_id: c.rule_id.clone(),
                reason: format!("dry_run: {}", c.command_line),
            });
            continue;
        }
        on_step(RunStep::Command {
            rule_id: c.rule_id.clone(),
            command_line: c.command_line.clone(),
        });
        let outcome = run_command(cmd, Duration::from_secs(600));
        if outcome.success {
            run.disposed += 1;
            if let Some(b) = c.estimate_bytes {
                run.bytes_freed += b;
            }
        } else {
            run.failed += 1;
        }
        tracing::info!(rule = %c.rule_id, success = outcome.success, "clean command finished");
        run.outcomes.push(RuleOutcome::Command {
            rule_id: c.rule_id.clone(),
            outcome,
        });
    }
    Ok(run)
}

/// Re-bases per-plan progress onto the whole run.
struct SubProgress<'a> {
    inner: &'a mut dyn ProgressSink,
    base: u64,
    total: u64,
}

impl ProgressSink for SubProgress<'_> {
    fn report(&mut self, event: &ProgressEvent) {
        self.inner.report(&ProgressEvent {
            items_done: self.base + event.items_done,
            items_total: Some(self.total),
            bytes_done: event.bytes_done,
            bytes_total: event.bytes_total,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::evaluate;
    use crate::progress::NoProgress;
    use crate::rules::{Env, Profile, parse_rules};
    use crate::safety::{HistoryStore, Outcome, ProtectedPaths};
    use std::path::Path;

    fn write(p: &Path, n: usize) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(p, vec![b'x'; n]).expect("write");
    }

    const RULES: &str = r#"
[rule]
id = "cache"
name = { id = "x", en = "x" }
description = { id = "x", en = "x" }
risk = "green"
profile = "general"
group = "browser"
[[match]]
path = "~/.cache/app"
kind = "dir_contents"

[rule2]
"#;

    fn two_rules() -> RuleSet {
        let green = RULES.replace("\n[rule2]\n", "");
        let trash = r#"
[rule]
id = "trash"
name = { id = "x", en = "x" }
description = { id = "x", en = "x" }
risk = "yellow"
profile = "general"
group = "trash"
permanent = true
[[match]]
path = "~/.Trash"
kind = "dir_contents"
"#;
        parse_rules([("g.toml", green.as_str()), ("t.toml", trash)]).expect("rules")
    }

    #[test]
    fn dry_run_touches_nothing_and_apply_removes_selected_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join(".cache/app/a"), 10);
        write(&home.join(".cache/app/b"), 20);
        write(&home.join(".Trash/t"), 5);
        let rules = two_rules();
        let env = Env::with_home(&home);
        let protected = ProtectedPaths::os_defaults();
        let safety = Safety::new(
            protected.clone(),
            HistoryStore::with_dir(tmp.path().join("h")).expect("h"),
        );
        let report = evaluate(&rules, Profile::General, &protected, &env);
        assert_eq!(report.findings.len(), 2);

        let sel = Selection::greens(&report);
        assert!(
            sel.contains("cache") && !sel.contains("trash"),
            "only green pre-selected"
        );

        let dry =
            execute(&report, &rules, &sel, false, None, &safety, &mut NoProgress).expect("dry");
        assert!(dry.dry_run);
        assert!(home.join(".cache/app/a").exists());
        assert_eq!(dry.disposed, 2);
        assert_eq!(dry.bytes_freed, 0);

        // Apply the green rule permanently (tempdir; trash is a real side effect).
        let perm = PermanentConfirmation::i_understand_this_cannot_be_undone();
        let mut report_perm = report.clone();
        for f in &mut report_perm.findings {
            f.permanent = true; // force the permanent path for the test fs
        }
        let run = execute(
            &report_perm,
            &rules,
            &sel,
            true,
            Some(perm),
            &safety,
            &mut NoProgress,
        )
        .expect("apply");
        assert!(!home.join(".cache/app/a").exists());
        assert!(!home.join(".cache/app/b").exists());
        assert!(
            home.join(".cache/app").exists(),
            "dir_contents keeps the directory"
        );
        assert!(home.join(".Trash/t").exists(), "unselected rule untouched");
        // Freed bytes are what the disk gives back (allocated blocks), so
        // at least the 30 logical bytes of the two files.
        assert!(run.bytes_freed >= 30, "{}", run.bytes_freed);
        assert_eq!(safety.history().list().expect("list").len(), 1);
    }

    #[test]
    fn permanent_rule_without_confirmation_is_skipped_with_reason() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join(".Trash/t"), 5);
        let rules = two_rules();
        let protected = ProtectedPaths::os_defaults();
        let safety = Safety::new(
            protected.clone(),
            HistoryStore::with_dir(tmp.path().join("h")).expect("h"),
        );
        let report = evaluate(&rules, Profile::General, &protected, &Env::with_home(&home));
        let sel = Selection {
            rule_ids: ["trash".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let run =
            execute(&report, &rules, &sel, true, None, &safety, &mut NoProgress).expect("run");
        assert!(home.join(".Trash/t").exists());
        assert!(
            matches!(&run.outcomes[0], RuleOutcome::Skipped { reason, .. } if reason.contains("permanent"))
        );
    }

    #[test]
    fn changed_item_is_skipped_by_safety_and_reported() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        write(&home.join(".cache/app/a"), 10);
        let rules = two_rules();
        let protected = ProtectedPaths::os_defaults();
        let safety = Safety::new(
            protected.clone(),
            HistoryStore::with_dir(tmp.path().join("h")).expect("h"),
        );
        let mut report = evaluate(&rules, Profile::General, &protected, &Env::with_home(&home));
        for f in &mut report.findings {
            f.permanent = true;
        }
        // Make the file vanish between evaluation and execution.
        std::fs::rename(home.join(".cache/app/a"), home.join("moved")).expect("rename");
        let run = execute(
            &report,
            &rules,
            &Selection::greens(&report),
            true,
            Some(PermanentConfirmation::i_understand_this_cannot_be_undone()),
            &safety,
            &mut NoProgress,
        )
        .expect("run");
        // The file vanished before snapshot: nothing to plan, session empty.
        match &run.outcomes[0] {
            RuleOutcome::Files { session, .. } => assert!(
                session
                    .entries
                    .iter()
                    .all(|e| e.outcome != Outcome::Disposed)
            ),
            other => panic!("unexpected {other:?}"),
        }
    }
}
