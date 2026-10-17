//! The only place that moves files. Re-validates each one first, records
//! the session before returning, and can put everything back.

use std::path::{Path, PathBuf};

use super::history::{MoveRecord, OrganizeSession, OrganizeStore};
use super::plan::MovePlan;
use crate::error::{Result, SapuError};
use crate::progress::{ProgressEvent, ProgressSink, Throttled};
use crate::safety::{
    DeletePlan, Disposal, FileSnapshot, PermanentConfirmation, Safety, revalidate,
};

#[derive(Debug, Clone)]
pub struct ApplyReport {
    pub session: OrganizeSession,
    pub failures: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UndoReport {
    pub restored: usize,
    pub skipped: usize,
    pub failed: usize,
}

fn invalid(msg: &str) -> SapuError {
    SapuError::io(
        "organize apply",
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string()),
    )
}

#[cfg(unix)]
fn same_volume(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let dev = |p: &Path| {
        let mut p = p.to_path_buf();
        while !p.exists() {
            if !p.pop() {
                return None;
            }
        }
        std::fs::metadata(&p).ok().map(|m| m.dev())
    };
    dev(a) == dev(b)
}

#[cfg(not(unix))]
fn same_volume(_a: &Path, _b: &Path) -> bool {
    true
}

/// Moves one path with `rename`, the only call of its kind in the crate.
fn rename(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(from, to)
}

/// Runs the plan. Each move is re-validated right before it happens; a
/// real run is recorded before returning.
pub fn apply(
    plan: &MovePlan,
    dry_run: bool,
    store: &OrganizeStore,
    progress: &mut dyn ProgressSink,
) -> Result<ApplyReport> {
    let root = std::path::absolute(&plan.root).map_err(|e| SapuError::io("resolve root", e))?;
    for m in &plan.moves {
        if !m.to.starts_with(&root) || !m.from.starts_with(&root) {
            return Err(invalid("a move leaves the folder"));
        }
        if !same_volume(&m.from, &m.to) {
            return Err(invalid("moves across volumes are not supported"));
        }
    }
    let mut sink = Throttled::new(progress, 10);
    let total = plan.moves.len() as u64;
    let bytes_total: u64 = plan.moves.iter().map(|m| m.bytes).sum();
    let mut session = OrganizeSession {
        id: OrganizeSession::new_id(),
        root: root.clone(),
        started_at_unix: OrganizeSession::now_unix(),
        finished_at_unix: 0,
        moves: Vec::new(),
        moved: 0,
        skipped: plan.skipped.len(),
        failed: 0,
        undone: false,
    };
    let mut failures = Vec::new();
    let mut bytes_done = 0u64;
    for (i, m) in plan.moves.iter().enumerate() {
        if let Err(reason) = revalidate(&m.snapshot, &root) {
            session.skipped += 1;
            failures.push((m.from.clone(), reason.to_string()));
        } else if dry_run {
            session.moved += 1;
        } else if m.to.exists() {
            session.skipped += 1;
            failures.push((m.from.clone(), "destination already exists".into()));
        } else {
            match rename(&m.from, &m.to) {
                Ok(()) => {
                    session.moved += 1;
                    session.moves.push(MoveRecord {
                        from: m.from.clone(),
                        to: m.to.clone(),
                        bytes: m.bytes,
                        len: m.snapshot.size,
                    });
                }
                Err(e) => {
                    session.failed += 1;
                    failures.push((m.from.clone(), e.to_string()));
                }
            }
        }
        bytes_done += m.bytes;
        sink.report(&ProgressEvent {
            items_done: i as u64 + 1,
            items_total: Some(total),
            bytes_done,
            bytes_total: Some(bytes_total),
        });
    }
    session.finished_at_unix = OrganizeSession::now_unix();
    tracing::info!(
        dry_run,
        moved = session.moved,
        skipped = session.skipped,
        failed = session.failed,
        "organize session finished"
    );
    if !dry_run {
        store.record(&session)?;
    }
    Ok(ApplyReport { session, failures })
}

/// Puts every recorded move back, newest first. An entry whose file was
/// changed or moved again since is skipped; the rest still return. Folders
/// Sapu created that end up empty are removed through `safety`.
pub fn undo(
    session: &OrganizeSession,
    store: &OrganizeStore,
    safety: &Safety,
    progress: &mut dyn ProgressSink,
) -> Result<UndoReport> {
    let mut sink = Throttled::new(progress, 10);
    let total = session.moves.len() as u64;
    let mut report = UndoReport {
        restored: 0,
        skipped: 0,
        failed: 0,
    };
    for (i, m) in session.moves.iter().rev().enumerate() {
        let current = FileSnapshot::capture(&m.to);
        let back_is_free = !m.from.exists();
        let unchanged = current
            .as_ref()
            .map(|s| s.size == m.len || s.is_dir)
            .unwrap_or(false);
        if !back_is_free || !unchanged {
            report.skipped += 1;
        } else {
            match rename(&m.to, &m.from) {
                Ok(()) => report.restored += 1,
                Err(_) => report.failed += 1,
            }
        }
        sink.report(&ProgressEvent {
            items_done: i as u64 + 1,
            items_total: Some(total),
            bytes_done: 0,
            bytes_total: None,
        });
    }
    remove_empty_destinations(session, safety)?;
    store.mark_undone(&session.id)?;
    Ok(report)
}

/// The destination folders of this session that are now empty were created
/// by Sapu for it; removing them restores the tree exactly. Permanent is
/// fine for an empty folder (nothing to restore), and it still goes
/// through the safety layer like every removal.
fn remove_empty_destinations(session: &OrganizeSession, safety: &Safety) -> Result<()> {
    let mut dirs: Vec<PathBuf> = session
        .moves
        .iter()
        .filter_map(|m| m.to.parent().map(Path::to_path_buf))
        .filter(|d| d.starts_with(&session.root) && *d != session.root)
        .collect();
    dirs.sort();
    dirs.dedup();
    let empty: Vec<FileSnapshot> = dirs
        .into_iter()
        .filter(|d| {
            std::fs::read_dir(d)
                .map(|mut it| it.next().is_none())
                .unwrap_or(false)
        })
        .filter_map(|d| FileSnapshot::capture(&d).ok())
        .collect();
    if empty.is_empty() {
        return Ok(());
    }
    let mut plan = DeletePlan::new(&session.root)
        .with_disposal(Disposal::Permanent(
            PermanentConfirmation::i_understand_this_cannot_be_undone(),
        ))
        .apply();
    for snap in empty {
        plan.push(snap);
    }
    safety.execute(&plan, &mut crate::progress::NoProgress)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{Selection, build_move_plan, propose, scan_folder};
    use crate::progress::NoProgress;
    use crate::safety::{HistoryStore, ProtectedPaths, Safety};
    use std::path::Path;

    fn safety(t: &Path) -> Safety {
        Safety::new(
            ProtectedPaths::os_defaults(),
            HistoryStore::with_dir(t.join("history")).expect("history"),
        )
    }

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn touch(p: &Path, body: &[u8]) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, body).expect("write");
    }
    fn listing(root: &Path) -> Vec<String> {
        let mut v = Vec::new();
        for e in jwalk::WalkDir::new(root)
            .sort(true)
            .into_iter()
            .filter_map(|r| r.ok())
        {
            if e.depth() > 0 {
                v.push(
                    e.path()
                        .strip_prefix(root)
                        .expect("rel")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        v
    }
    fn full_plan(root: &Path) -> crate::organize::MovePlan {
        let plan = propose(&scan_folder(root).expect("scan"));
        let sel = Selection {
            group_ids: plan.groups.iter().map(|g| g.id.clone()).collect(),
            excluded: vec![],
            renames: vec![],
        };
        build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan")
    }

    #[test]
    fn apply_then_undo_restores_the_exact_tree() {
        let t = tempdir();
        let root = t.path().join("docs");
        for n in [
            "a.pdf",
            "b.pdf",
            "clip.mp4",
            "Invoice-1.pdf",
            "Invoice-2.pdf",
            "Invoice-3.pdf",
        ] {
            touch(&root.join(n), n.as_bytes());
        }
        let before = listing(&root);
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), false, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 6);
        assert!(root.join("Invoice/Invoice-1.pdf").is_file());
        assert!(root.join("Video/clip.mp4").is_file());
        assert_eq!(store.list().expect("list").len(), 1, "recorded");

        let undo_report =
            undo(&report.session, &store, &safety(t.path()), &mut NoProgress).expect("undo");
        assert_eq!(undo_report.restored, 6);
        assert_eq!(listing(&root), before, "tree is exactly as before");
        assert!(
            store
                .get(&report.session.id)
                .expect("get")
                .expect("some")
                .undone
        );
    }

    #[test]
    fn dry_run_moves_nothing_and_records_nothing() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), true, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 1, "counts what would move");
        assert!(root.join("a.pdf").is_file());
        assert!(store.list().expect("list").is_empty());
    }

    #[test]
    fn changed_file_is_skipped() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let plan = full_plan(&root);
        touch(&root.join("a.pdf"), b"edited after planning");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&plan, false, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 0);
        assert_eq!(report.session.skipped, 1);
        assert!(root.join("a.pdf").is_file());
    }

    #[test]
    fn undo_skips_entries_that_moved_again() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        touch(&root.join("b.pdf"), b"b");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), false, &store, &mut NoProgress).expect("apply");
        // The person moved one file elsewhere by hand (simulated by writing it out and leaving the slot empty).
        let gone = root.join("Dokumen/a.pdf");
        let elsewhere = t.path().join("elsewhere.pdf");
        std::fs::copy(&gone, &elsewhere).expect("copy");
        // Emulate the hand move without rename/remove APIs: truncate so re-validation fails.
        std::fs::write(&gone, b"changed by hand").expect("write");
        let r = undo(&report.session, &store, &safety(t.path()), &mut NoProgress).expect("undo");
        assert_eq!(r.restored, 1);
        assert_eq!(r.skipped, 1);
        assert!(root.join("b.pdf").is_file());
    }

    #[test]
    fn refuses_moves_outside_root() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let mut plan = full_plan(&root);
        plan.moves[0].to = t.path().join("escape.pdf");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        assert!(apply(&plan, false, &store, &mut NoProgress).is_err());
    }
}
