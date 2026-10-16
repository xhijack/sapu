//! Clean-session history (S2): what was removed, when, how, and restore.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, SapuError};

use super::dispose::DisposalKind;
use super::validate::ValidationFailure;

/// Per-item result of a session.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    /// Removed from its original location.
    Disposed,
    /// Dry run: would have been removed.
    WouldDispose,
    /// Refused: inside a protected path.
    Protected { reason: String },
    /// Skipped: failed re-validation.
    Skipped { reason: ValidationFailure },
    /// Disposal was attempted and failed (e.g. locked file).
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub outcome: Outcome,
}

/// One execution of a [`super::DeletePlan`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CleanSession {
    pub id: String,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub dry_run: bool,
    pub disposal: DisposalKind,
    pub scope: PathBuf,
    pub entries: Vec<SessionEntry>,
    pub bytes_freed: u64,
    pub disposed: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl CleanSession {
    pub(super) fn new_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{nanos:x}-{}", std::process::id())
    }

    pub(super) fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Result of a restore attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestoreReport {
    pub restored: Vec<PathBuf>,
    pub not_found_in_trash: Vec<PathBuf>,
}

/// On-disk store: one JSON file per session under `dir`.
#[derive(Debug, Clone)]
pub struct HistoryStore {
    dir: PathBuf,
}

impl HistoryStore {
    /// `<local data dir>/sapu/history`.
    pub fn open_default() -> Result<Self> {
        let base = dirs::data_local_dir().ok_or_else(|| {
            SapuError::io(
                "locate local data dir",
                std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"),
            )
        })?;
        Self::with_dir(base.join("sapu").join("history"))
    }

    /// Use an explicit directory (tests, portable mode).
    pub fn with_dir(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|e| SapuError::io("create history dir", e))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn record(&self, session: &CleanSession) -> Result<PathBuf> {
        let path = self.file_for(&session.id);
        let json = serde_json::to_vec_pretty(session)?;
        std::fs::write(&path, json).map_err(|e| SapuError::io("write history", e))?;
        Ok(path)
    }

    /// All sessions, newest first.
    pub fn list(&self) -> Result<Vec<CleanSession>> {
        let mut out = Vec::new();
        let rd = std::fs::read_dir(&self.dir).map_err(|e| SapuError::io("read history dir", e))?;
        for entry in rd {
            let entry = entry.map_err(|e| SapuError::io("read history entry", e))?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                match std::fs::read(&path) {
                    Ok(bytes) => match serde_json::from_slice::<CleanSession>(&bytes) {
                        Ok(s) => out.push(s),
                        Err(e) => tracing::warn!(error = %e, "skipping unreadable history file"),
                    },
                    Err(e) => tracing::warn!(error = %e, "skipping unreadable history file"),
                }
            }
        }
        out.sort_by(|a, b| {
            b.started_at_unix
                .cmp(&a.started_at_unix)
                .then(b.id.cmp(&a.id))
        });
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<Option<CleanSession>> {
        match std::fs::read(self.file_for(id)) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SapuError::io("read history", e)),
        }
    }

    /// Restore every item of a trashed session that is still in the trash.
    /// Only Windows and Linux expose a trash listing API; macOS returns
    /// [`SapuError::RestoreUnsupported`] so the UI can point to Finder.
    pub fn restore_session(&self, id: &str) -> Result<RestoreReport> {
        let session = self.get(id)?.ok_or_else(|| {
            SapuError::io(
                "restore",
                std::io::Error::new(std::io::ErrorKind::NotFound, "unknown session id"),
            )
        })?;
        let wanted: Vec<PathBuf> = session
            .entries
            .iter()
            .filter(|e| e.outcome == Outcome::Disposed)
            .map(|e| e.path.clone())
            .collect();
        if session.disposal == DisposalKind::Permanent {
            return Ok(RestoreReport {
                restored: Vec::new(),
                not_found_in_trash: wanted,
            });
        }
        restore::restore_paths(wanted)
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod restore {
    use super::*;

    pub(super) fn restore_paths(wanted: Vec<PathBuf>) -> Result<RestoreReport> {
        let items = trash::os_limited::list()
            .map_err(|e| SapuError::io("list trash", std::io::Error::other(e)))?;
        let mut report = RestoreReport::default();
        let mut to_restore = Vec::new();
        for path in wanted {
            match items.iter().find(|i| i.original_path() == path) {
                Some(item) => {
                    to_restore.push(item.clone());
                    report.restored.push(path);
                }
                None => report.not_found_in_trash.push(path),
            }
        }
        if !to_restore.is_empty() {
            trash::os_limited::restore_all(to_restore)
                .map_err(|e| SapuError::io("restore from trash", std::io::Error::other(e)))?;
        }
        Ok(report)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod restore {
    use super::*;

    pub(super) fn restore_paths(_wanted: Vec<PathBuf>) -> Result<RestoreReport> {
        Err(SapuError::RestoreUnsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, started: u64) -> CleanSession {
        CleanSession {
            id: id.into(),
            started_at_unix: started,
            finished_at_unix: started + 1,
            dry_run: false,
            disposal: DisposalKind::Trash,
            scope: PathBuf::from("/tmp/scope"),
            entries: vec![SessionEntry {
                path: PathBuf::from("/tmp/scope/a"),
                size: 10,
                is_dir: false,
                outcome: Outcome::Disposed,
            }],
            bytes_freed: 10,
            disposed: 1,
            skipped: 0,
            failed: 0,
        }
    }

    #[test]
    fn record_list_get_roundtrip_newest_first() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::with_dir(tmp.path().join("h")).expect("store");
        store.record(&session("old", 100)).expect("record");
        store.record(&session("new", 200)).expect("record");

        let all = store.list().expect("list");
        assert_eq!(
            all.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["new", "old"]
        );
        assert_eq!(store.get("old").expect("get"), Some(session("old", 100)));
        assert_eq!(store.get("nope").expect("get"), None);
    }

    #[test]
    fn corrupt_file_is_skipped_not_fatal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::with_dir(tmp.path()).expect("store");
        std::fs::write(tmp.path().join("bad.json"), b"{not json").expect("write");
        store.record(&session("ok", 1)).expect("record");
        assert_eq!(store.list().expect("list").len(), 1);
    }

    #[test]
    fn restore_permanent_session_reports_nothing_restored() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = HistoryStore::with_dir(tmp.path()).expect("store");
        let mut s = session("perm", 1);
        s.disposal = DisposalKind::Permanent;
        store.record(&s).expect("record");
        let r = store.restore_session("perm").expect("restore");
        assert!(r.restored.is_empty());
        assert_eq!(r.not_found_in_trash, vec![PathBuf::from("/tmp/scope/a")]);
    }

    #[test]
    fn ids_are_unique() {
        let a = CleanSession::new_id();
        let b = CleanSession::new_id();
        assert_ne!(a, b);
    }
}
