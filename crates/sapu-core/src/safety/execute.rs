//! Executes a [`DeletePlan`]: protected check → re-validation → disposal →
//! history, item by item, never aborting the whole session on one failure.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::progress::{ProgressEvent, ProgressSink, Throttled};

use super::dispose::{Disposal, dispose};
use super::history::{CleanSession, HistoryStore, Outcome, SessionEntry};
use super::protected::ProtectedPaths;
use super::snapshot::FileSnapshot;
use super::validate::revalidate;

/// What to remove. Dry-run by default (S4).
#[derive(Debug, Clone)]
pub struct DeletePlan {
    scope: PathBuf,
    items: Vec<FileSnapshot>,
    disposal: Disposal,
    dry_run: bool,
}

impl DeletePlan {
    /// Every item must lie inside `scope`; symlink targets must too.
    pub fn new(scope: impl Into<PathBuf>) -> Self {
        Self {
            scope: scope.into(),
            items: Vec::new(),
            disposal: Disposal::Trash,
            dry_run: true,
        }
    }

    pub fn push(&mut self, item: FileSnapshot) {
        self.items.push(item);
    }

    pub fn with_item(mut self, item: FileSnapshot) -> Self {
        self.push(item);
        self
    }

    /// Turn the dry run into a real run.
    pub fn apply(mut self) -> Self {
        self.dry_run = false;
        self
    }

    pub fn with_disposal(mut self, disposal: Disposal) -> Self {
        self.disposal = disposal;
        self
    }

    pub fn scope(&self) -> &Path {
        &self.scope
    }

    pub fn items(&self) -> &[FileSnapshot] {
        &self.items
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn disposal(&self) -> &Disposal {
        &self.disposal
    }

    pub fn total_bytes(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    /// Deepest directory that contains every path (after lexical
    /// normalisation). Useful default for [`DeletePlan::new`] when the
    /// caller only has a list of files.
    pub fn common_scope<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Option<PathBuf> {
        let mut iter = paths.into_iter();
        let first = super::protected::lexical_absolute(iter.next()?);
        let mut prefix: Vec<_> = first.components().collect();
        for p in iter {
            let p = super::protected::lexical_absolute(p);
            let n = prefix
                .iter()
                .zip(p.components())
                .take_while(|(a, b)| *a == b)
                .count();
            prefix.truncate(n);
        }
        // A single file's scope is its parent directory.
        if prefix.len() == first.components().count() {
            prefix.pop();
        }
        if prefix.is_empty() {
            return None;
        }
        Some(prefix.iter().collect())
    }
}

/// The safety gate. Holds the protected set and the history store.
#[derive(Debug, Clone)]
pub struct Safety {
    protected: ProtectedPaths,
    history: HistoryStore,
}

impl Safety {
    pub fn new(protected: ProtectedPaths, history: HistoryStore) -> Self {
        Self { protected, history }
    }

    pub fn protected(&self) -> &ProtectedPaths {
        &self.protected
    }

    pub fn history(&self) -> &HistoryStore {
        &self.history
    }

    /// Run the plan. Progress events are throttled to at most 10 per second.
    /// A real run is recorded in history before returning.
    pub fn execute(
        &self,
        plan: &DeletePlan,
        progress: &mut dyn ProgressSink,
    ) -> Result<CleanSession> {
        let mut sink = Throttled::new(progress, 10);
        let total = plan.items.len() as u64;
        let bytes_total = plan.total_bytes();
        let mut session = CleanSession {
            id: CleanSession::new_id(),
            started_at_unix: CleanSession::now_unix(),
            finished_at_unix: 0,
            dry_run: plan.dry_run,
            disposal: plan.disposal.kind(),
            scope: plan.scope.clone(),
            entries: Vec::with_capacity(plan.items.len()),
            bytes_freed: 0,
            disposed: 0,
            skipped: 0,
            failed: 0,
        };
        let mut bytes_done = 0u64;

        for (i, item) in plan.items.iter().enumerate() {
            let outcome = self.process(item, plan);
            match &outcome {
                Outcome::Disposed => {
                    session.disposed += 1;
                    session.bytes_freed += item.size;
                }
                Outcome::WouldDispose => session.disposed += 1,
                Outcome::Protected { .. } | Outcome::Skipped { .. } => session.skipped += 1,
                Outcome::Failed { .. } => session.failed += 1,
            }
            session.entries.push(SessionEntry {
                path: item.path.clone(),
                size: item.size,
                is_dir: item.is_dir,
                outcome,
            });
            bytes_done += item.size;
            sink.report(&ProgressEvent {
                items_done: i as u64 + 1,
                items_total: Some(total),
                bytes_done,
                bytes_total: Some(bytes_total),
            });
        }

        session.finished_at_unix = CleanSession::now_unix();
        tracing::info!(
            dry_run = plan.dry_run,
            disposed = session.disposed,
            skipped = session.skipped,
            failed = session.failed,
            bytes_freed = session.bytes_freed,
            "clean session finished"
        );
        if !plan.dry_run {
            self.history.record(&session)?;
        }
        Ok(session)
    }

    fn process(&self, item: &FileSnapshot, plan: &DeletePlan) -> Outcome {
        if let Some(entry) = self.protected.check(&item.path) {
            return Outcome::Protected {
                reason: entry.reason.to_string(),
            };
        }
        if let Err(reason) = revalidate(item, &plan.scope) {
            return Outcome::Skipped { reason };
        }
        if plan.dry_run {
            return Outcome::WouldDispose;
        }
        match dispose(&item.path, &plan.disposal) {
            Ok(()) => Outcome::Disposed,
            Err(e) => Outcome::Failed {
                message: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{FnSink, NoProgress};
    use crate::safety::dispose::PermanentConfirmation;
    use crate::safety::protected::Protection;
    use crate::safety::validate::ValidationFailure;

    struct Fixture {
        _tmp: tempfile::TempDir,
        scope: PathBuf,
        safety: Safety,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().expect("tempdir");
        let scope = tmp.path().join("scope");
        std::fs::create_dir_all(&scope).expect("mkdir");
        let history = HistoryStore::with_dir(tmp.path().join("history")).expect("history");
        let safety = Safety::new(ProtectedPaths::os_defaults(), history);
        Fixture {
            _tmp: tmp,
            scope,
            safety,
        }
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> FileSnapshot {
        let p = dir.join(name);
        std::fs::write(&p, bytes).expect("write");
        FileSnapshot::capture(&p).expect("capture")
    }

    fn permanent() -> Disposal {
        Disposal::Permanent(PermanentConfirmation::i_understand_this_cannot_be_undone())
    }

    #[test]
    fn dry_run_is_default_and_touches_nothing() {
        let fx = fixture();
        let a = file(&fx.scope, "a", b"aaa");
        let plan = DeletePlan::new(&fx.scope).with_item(a.clone());
        assert!(plan.is_dry_run());

        let s = fx.safety.execute(&plan, &mut NoProgress).expect("execute");
        assert!(a.path.exists());
        assert_eq!(s.entries[0].outcome, Outcome::WouldDispose);
        assert_eq!(s.disposed, 1);
        assert_eq!(s.bytes_freed, 0);
        assert!(
            fx.safety.history().list().expect("list").is_empty(),
            "dry run is not recorded"
        );
    }

    #[test]
    fn apply_permanent_removes_and_records_history() {
        let fx = fixture();
        let a = file(&fx.scope, "a", b"aaa");
        let plan = DeletePlan::new(&fx.scope)
            .with_item(a.clone())
            .with_disposal(permanent())
            .apply();

        let s = fx.safety.execute(&plan, &mut NoProgress).expect("execute");
        assert!(!a.path.exists());
        assert_eq!(s.entries[0].outcome, Outcome::Disposed);
        assert_eq!(s.bytes_freed, 3);

        let recorded = fx.safety.history().list().expect("list");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].id, s.id);
    }

    #[test]
    fn protected_item_is_refused_even_when_applied() {
        let fx = fixture();
        let secret_dir = fx.scope.join("secret");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        let key = file(&secret_dir, "key", b"k");
        let mut protected = ProtectedPaths::os_defaults();
        protected.add(&secret_dir, Protection::Subtree, "test");
        let safety = Safety::new(protected, fx.safety.history().clone());

        let plan = DeletePlan::new(&fx.scope)
            .with_item(key.clone())
            .with_disposal(permanent())
            .apply();
        let s = safety.execute(&plan, &mut NoProgress).expect("execute");
        assert!(key.path.exists());
        assert!(matches!(s.entries[0].outcome, Outcome::Protected { .. }));
        assert_eq!(s.skipped, 1);
    }

    #[test]
    fn changed_file_is_skipped_but_others_proceed() {
        let fx = fixture();
        let a = file(&fx.scope, "a", b"aaa");
        let b = file(&fx.scope, "b", b"bbb");
        std::fs::write(&a.path, b"changed!").expect("modify");

        let plan = DeletePlan::new(&fx.scope)
            .with_item(a.clone())
            .with_item(b.clone())
            .with_disposal(permanent())
            .apply();
        let s = fx.safety.execute(&plan, &mut NoProgress).expect("execute");
        assert!(a.path.exists(), "modified file must survive");
        assert!(!b.path.exists());
        assert!(matches!(
            s.entries[0].outcome,
            Outcome::Skipped {
                reason: ValidationFailure::SizeChanged { .. }
            }
        ));
        assert_eq!(s.entries[1].outcome, Outcome::Disposed);
        assert_eq!((s.disposed, s.skipped, s.failed), (1, 1, 0));
    }

    #[test]
    fn item_outside_scope_is_skipped() {
        let fx = fixture();
        let outside = tempfile::tempdir().expect("tempdir");
        let x = file(outside.path(), "x", b"x");
        let plan = DeletePlan::new(&fx.scope)
            .with_item(x.clone())
            .with_disposal(permanent())
            .apply();
        let s = fx.safety.execute(&plan, &mut NoProgress).expect("execute");
        assert!(x.path.exists());
        assert_eq!(
            s.entries[0].outcome,
            Outcome::Skipped {
                reason: ValidationFailure::OutsideScope
            }
        );
    }

    #[test]
    fn common_scope_is_deepest_shared_dir() {
        let a = PathBuf::from("/x/y/a.txt");
        let b = PathBuf::from("/x/y/z/b.txt");
        assert_eq!(
            DeletePlan::common_scope([a.as_path(), b.as_path()]),
            Some(PathBuf::from("/x/y"))
        );
        assert_eq!(
            DeletePlan::common_scope([a.as_path()]),
            Some(PathBuf::from("/x/y"))
        );
        assert_eq!(DeletePlan::common_scope(std::iter::empty()), None);
    }

    #[test]
    fn progress_reaches_final_event() {
        let fx = fixture();
        let mut plan = DeletePlan::new(&fx.scope);
        for i in 0..5 {
            plan.push(file(&fx.scope, &format!("f{i}"), b"12"));
        }
        let mut last = None;
        fx.safety
            .execute(
                &plan,
                &mut FnSink(|e: &ProgressEvent| last = Some(e.clone())),
            )
            .expect("execute");
        assert_eq!(
            last,
            Some(ProgressEvent {
                items_done: 5,
                items_total: Some(5),
                bytes_done: 10,
                bytes_total: Some(10)
            })
        );
    }
}
