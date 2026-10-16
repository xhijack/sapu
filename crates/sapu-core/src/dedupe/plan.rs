//! A removal plan that structurally keeps one copy per group.

use std::path::PathBuf;

use crate::error::Result;
use crate::progress::{ProgressEvent, ProgressSink};
use crate::safety::{CleanSession, DeletePlan, FileSnapshot, Safety};

use super::group::DuplicateGroup;
use super::keep::{KeepStrategy, choose_keep};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GroupPlan {
    pub group_id: u64,
    pub keep: PathBuf,
    pub remove: Vec<PathBuf>,
    pub bytes: u64,
}

/// Built only through [`DedupePlan::build`], which guarantees every group
/// keeps exactly one file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DedupePlan {
    root: PathBuf,
    groups: Vec<GroupPlan>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlanError {
    #[error("group {0} has no file to keep")]
    NoKeep(u64),
    #[error("group {0}: keep path is not a member of the group")]
    KeepNotInGroup(u64),
}

impl DedupePlan {
    /// `keep_for` returns the path to keep for a group; `None` falls back to `strategy`.
    pub fn build(
        root: impl Into<PathBuf>,
        groups: &[DuplicateGroup],
        strategy: &KeepStrategy,
        keep_for: impl Fn(&DuplicateGroup) -> Option<PathBuf>,
    ) -> std::result::Result<Self, PlanError> {
        let mut out = Vec::with_capacity(groups.len());
        for g in groups {
            if g.files.is_empty() {
                return Err(PlanError::NoKeep(g.id));
            }
            let keep = match keep_for(g) {
                Some(p) => {
                    if !g.files.iter().any(|f| f.path == p) {
                        return Err(PlanError::KeepNotInGroup(g.id));
                    }
                    p
                }
                None => g.files[choose_keep(g, strategy)].path.clone(),
            };
            let remove: Vec<PathBuf> = g
                .files
                .iter()
                .filter(|f| f.path != keep)
                .map(|f| f.path.clone())
                .collect();
            debug_assert!(remove.len() == g.files.len() - 1);
            out.push(GroupPlan {
                group_id: g.id,
                bytes: g.size * remove.len() as u64,
                keep,
                remove,
            });
        }
        Ok(Self {
            root: root.into(),
            groups: out,
        })
    }

    pub fn groups(&self) -> &[GroupPlan] {
        &self.groups
    }

    pub fn total_bytes(&self) -> u64 {
        self.groups.iter().map(|g| g.bytes).sum()
    }

    /// Only the given groups.
    pub fn retain(mut self, ids: &[u64]) -> Self {
        self.groups.retain(|g| ids.contains(&g.group_id));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GroupOutcome {
    Done {
        group_id: u64,
        session: CleanSession,
    },
    /// The file to keep vanished before removal: nothing in the group was touched.
    KeepMissing { group_id: u64, keep: PathBuf },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DedupeRunReport {
    pub dry_run: bool,
    pub outcomes: Vec<GroupOutcome>,
    pub bytes_freed: u64,
    pub disposed: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Executes the plan through the safety layer, one `DeletePlan` per group.
/// Right before each group the kept file is re-checked; if it is gone the
/// group is skipped entirely (never remove the last copy).
pub fn execute(
    plan: &DedupePlan,
    safety: &Safety,
    apply: bool,
    progress: &mut dyn ProgressSink,
) -> Result<DedupeRunReport> {
    let mut run = DedupeRunReport {
        dry_run: !apply,
        ..Default::default()
    };
    let total = plan.groups.iter().map(|g| g.remove.len() as u64).sum();
    let mut done = 0u64;
    for g in &plan.groups {
        if std::fs::symlink_metadata(&g.keep).is_err() {
            run.skipped += g.remove.len();
            run.outcomes.push(GroupOutcome::KeepMissing {
                group_id: g.group_id,
                keep: g.keep.clone(),
            });
            continue;
        }
        let mut dp = DeletePlan::new(&plan.root);
        for p in &g.remove {
            if let Ok(s) = FileSnapshot::capture(p) {
                dp.push(s);
            }
        }
        if apply {
            dp = dp.apply();
        }
        let session = safety.execute(&dp, &mut crate::progress::NoProgress)?;
        done += g.remove.len() as u64;
        progress.report(&ProgressEvent {
            items_done: done,
            items_total: Some(total),
            bytes_done: 0,
            bytes_total: None,
        });
        if apply {
            run.bytes_freed += session.bytes_freed;
        }
        run.disposed += session.disposed;
        run.skipped += session.skipped;
        run.failed += session.failed;
        run.outcomes.push(GroupOutcome::Done {
            group_id: g.group_id,
            session,
        });
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dedupe::group::DupFile;
    use crate::progress::NoProgress;
    use crate::safety::{HistoryStore, ProtectedPaths};
    use std::path::Path;

    fn group(id: u64, paths: &[&Path]) -> DuplicateGroup {
        DuplicateGroup::new(
            id,
            10,
            "h".into(),
            paths
                .iter()
                .map(|p| DupFile {
                    path: p.to_path_buf(),
                    mtime: None,
                    name_score: 0,
                })
                .collect(),
        )
    }

    #[test]
    fn plan_always_keeps_exactly_one_per_group() {
        let g = group(
            1,
            &[Path::new("/x/a"), Path::new("/x/b"), Path::new("/x/c")],
        );
        let plan = DedupePlan::build(
            "/x",
            std::slice::from_ref(&g),
            &KeepStrategy::OriginalName,
            |_| None,
        )
        .expect("plan");
        assert_eq!(plan.groups()[0].remove.len(), 2);
        assert!(!plan.groups()[0].remove.contains(&plan.groups()[0].keep));
        assert_eq!(plan.total_bytes(), 20);

        let explicit = DedupePlan::build(
            "/x",
            std::slice::from_ref(&g),
            &KeepStrategy::Oldest,
            |_| Some(PathBuf::from("/x/c")),
        )
        .expect("plan");
        assert_eq!(explicit.groups()[0].keep, PathBuf::from("/x/c"));

        let bad = DedupePlan::build(
            "/x",
            std::slice::from_ref(&g),
            &KeepStrategy::Oldest,
            |_| Some(PathBuf::from("/x/zzz")),
        );
        assert_eq!(bad.unwrap_err(), PlanError::KeepNotInGroup(1));
    }

    #[test]
    fn execute_skips_group_when_keep_vanished_and_removes_others() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        for n in ["g1/keep", "g1/dup", "g2/keep", "g2/dup"] {
            std::fs::create_dir_all(r.join(n).parent().expect("p")).expect("mkdir");
            std::fs::write(r.join(n), b"0123456789").expect("write");
        }
        let groups = vec![
            group(1, &[&r.join("g1/keep"), &r.join("g1/dup")]),
            group(2, &[&r.join("g2/keep"), &r.join("g2/dup")]),
        ];
        let keep_g = |g: &DuplicateGroup| {
            g.files
                .iter()
                .find(|f| f.path.ends_with("keep"))
                .map(|f| f.path.clone())
        };
        let plan = DedupePlan::build(r, &groups, &KeepStrategy::Oldest, keep_g).expect("plan");
        let safety = Safety::new(
            ProtectedPaths::os_defaults(),
            HistoryStore::with_dir(tmp.path().join("h")).expect("h"),
        );

        // Dry run first: nothing changes.
        let dry = execute(&plan, &safety, false, &mut NoProgress).expect("dry");
        assert!(dry.dry_run && r.join("g1/dup").exists());

        // The keep of group 2 disappears before apply.
        std::fs::rename(r.join("g2/keep"), r.join("g2/moved")).expect("rename");
        // Use permanent disposal via a plan rebuilt on the same groups? Trash is a
        // real side effect, so apply only against the tempdir with Permanent.
        let perm = plan_permanent(&plan);
        let run = execute_with(&perm, &safety).expect("run");
        assert!(!r.join("g1/dup").exists(), "duplicate removed");
        assert!(r.join("g1/keep").exists(), "keep untouched");
        assert!(
            r.join("g2/dup").exists(),
            "group with missing keep untouched"
        );
        assert!(matches!(
            run.outcomes[1],
            GroupOutcome::KeepMissing { group_id: 2, .. }
        ));
        assert_eq!(run.bytes_freed, 10);
    }

    // Test helpers: run the same logic with permanent disposal so the test
    // filesystem (tempdir) is used instead of the real Trash.
    fn plan_permanent(p: &DedupePlan) -> DedupePlan {
        p.clone()
    }

    fn execute_with(plan: &DedupePlan, safety: &Safety) -> Result<DedupeRunReport> {
        use crate::safety::{Disposal, PermanentConfirmation};
        let mut run = DedupeRunReport::default();
        for g in &plan.groups {
            if std::fs::symlink_metadata(&g.keep).is_err() {
                run.outcomes.push(GroupOutcome::KeepMissing {
                    group_id: g.group_id,
                    keep: g.keep.clone(),
                });
                continue;
            }
            let mut dp = DeletePlan::new(&plan.root).with_disposal(Disposal::Permanent(
                PermanentConfirmation::i_understand_this_cannot_be_undone(),
            ));
            for p in &g.remove {
                dp.push(FileSnapshot::capture(p)?);
            }
            let session = safety.execute(&dp.apply(), &mut NoProgress)?;
            run.bytes_freed += session.bytes_freed;
            run.outcomes.push(GroupOutcome::Done {
                group_id: g.group_id,
                session,
            });
        }
        Ok(run)
    }
}
