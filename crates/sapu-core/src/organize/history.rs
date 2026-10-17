//! Every organise run on disk, so it can be undone later.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, SapuError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveRecord {
    pub from: PathBuf,
    pub to: PathBuf,
    /// Disk usage, for the summary.
    pub bytes: u64,
    /// Metadata length at move time, compared before undoing.
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrganizeSession {
    pub id: String,
    pub root: PathBuf,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    /// Successful moves only, in the order they happened.
    pub moves: Vec<MoveRecord>,
    /// Folders Sapu itself created for this run; the only ones undo may remove.
    #[serde(default)]
    pub created_dirs: Vec<PathBuf>,
    pub moved: usize,
    pub skipped: usize,
    pub failed: usize,
    #[serde(default)]
    pub undone: bool,
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl OrganizeSession {
    pub fn now_unix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Time-ordered and unique within a process: `<nanos hex>-<pid>-<n>`.
    pub fn new_id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(
            "{:x}-{}-{}",
            nanos,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }
}

/// One JSON file per session under `<local data>/sapu/organize/`.
pub struct OrganizeStore {
    dir: PathBuf,
}

impl OrganizeStore {
    pub fn open_default() -> Result<Self> {
        let base = dirs::data_local_dir().ok_or_else(|| {
            SapuError::io(
                "locate local data dir",
                std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"),
            )
        })?;
        Self::with_dir(base.join("sapu").join("organize"))
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|e| SapuError::io("create organize dir", e))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn record(&self, session: &OrganizeSession) -> Result<PathBuf> {
        let path = self.file(&session.id);
        std::fs::write(&path, serde_json::to_vec_pretty(session)?)
            .map_err(|e| SapuError::io("write session", e))?;
        Ok(path)
    }

    /// Newest first.
    pub fn list(&self) -> Result<Vec<OrganizeSession>> {
        let mut out = Vec::new();
        for entry in
            std::fs::read_dir(&self.dir).map_err(|e| SapuError::io("read organize dir", e))?
        {
            let Ok(entry) = entry else { continue };
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json")
                && let Ok(text) = std::fs::read_to_string(&p)
                && let Ok(s) = serde_json::from_str::<OrganizeSession>(&text)
            {
                out.push(s);
            }
        }
        out.sort_by(|a, b| {
            b.started_at_unix
                .cmp(&a.started_at_unix)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<Option<OrganizeSession>> {
        match std::fs::read_to_string(self.file(id)) {
            Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SapuError::io("read session", e)),
        }
    }

    pub fn mark_undone(&self, id: &str) -> Result<()> {
        if let Some(mut s) = self.get(id)? {
            s.undone = true;
            self.record(&s)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }

    fn session(id: &str, at: u64) -> OrganizeSession {
        OrganizeSession {
            id: id.into(),
            root: "/r".into(),
            started_at_unix: at,
            finished_at_unix: at + 1,
            moves: vec![MoveRecord {
                from: "/r/a".into(),
                to: "/r/G/a".into(),
                bytes: 5,
                len: 1,
            }],
            created_dirs: vec!["/r/G".into()],
            moved: 1,
            skipped: 0,
            failed: 0,
            undone: false,
        }
    }

    #[test]
    fn records_lists_newest_first_and_marks_undone() {
        let t = tempdir();
        let store = OrganizeStore::with_dir(t.path().join("organize")).expect("store");
        store.record(&session("one", 100)).expect("record");
        store.record(&session("two", 200)).expect("record");
        let list = store.list().expect("list");
        assert_eq!(
            list.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            vec!["two", "one"]
        );
        store.mark_undone("one").expect("mark");
        assert!(store.get("one").expect("get").expect("some").undone);
        assert!(store.get("zzz").expect("get").is_none());
    }

    #[test]
    fn ids_are_unique() {
        let a = OrganizeSession::new_id();
        let b = OrganizeSession::new_id();
        assert_ne!(a, b);
    }
}
