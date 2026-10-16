//! Totals of the previous scan per root, used as the progress denominator
//! for later scans (PRD §10.1 "scan berikutnya").

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::error::{Result, SapuError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScanStats {
    pub total_bytes: u64,
    pub file_count: u64,
    pub finished_at_unix: u64,
}

impl ScanStats {
    pub fn default_dir() -> Result<PathBuf> {
        let base = dirs::data_local_dir().ok_or_else(|| {
            SapuError::io(
                "locate local data dir",
                std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"),
            )
        })?;
        Ok(base.join("sapu").join("scans"))
    }

    fn file_for(dir: &Path, root: &Path) -> PathBuf {
        let mut h = DefaultHasher::new();
        root.hash(&mut h);
        dir.join(format!("{:016x}.json", h.finish()))
    }

    pub fn load(dir: &Path, root: &Path) -> Option<Self> {
        let bytes = std::fs::read(Self::file_for(dir, root)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self, dir: &Path, root: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(|e| SapuError::io("create scans dir", e))?;
        let json = serde_json::to_vec(self)?;
        std::fs::write(Self::file_for(dir, root), json)
            .map_err(|e| SapuError::io("write scan stats", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_per_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("scans");
        let a = ScanStats {
            total_bytes: 10,
            file_count: 2,
            finished_at_unix: 1,
        };
        a.save(&dir, Path::new("/a")).expect("save");
        assert_eq!(ScanStats::load(&dir, Path::new("/a")), Some(a));
        assert_eq!(ScanStats::load(&dir, Path::new("/b")), None);
    }
}
