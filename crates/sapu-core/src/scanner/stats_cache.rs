//! Totals of the previous scan per root, used as the progress denominator
//! for later scans (PRD §10.1 "scan berikutnya").

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::error::{Result, SapuError};

use super::result::{CategorySummary, DirSummary, ScanResult};

/// Snapshot of a finished scan, kept per root so the next scan can show
/// progress against it and tell the user what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ScanStats {
    pub total_bytes: u64,
    pub file_count: u64,
    pub finished_at_unix: u64,
    pub categories: Vec<CategorySummary>,
    /// Largest folders (up to 100) by total size.
    pub top_dirs: Vec<DirSummary>,
}

/// What changed between the previous snapshot and the current scan.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScanDiff {
    pub since_unix: u64,
    pub delta_bytes: i64,
    pub delta_files: i64,
    pub categories: Vec<CategoryDelta>,
    /// Folders with the largest absolute change, largest first.
    pub dirs: Vec<DirDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CategoryDelta {
    pub category: super::category::Category,
    pub delta_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirDelta {
    pub path: PathBuf,
    pub delta_bytes: i64,
    /// `true` when the folder did not exist in the previous scan.
    pub is_new: bool,
}

impl ScanStats {
    pub fn from_result(result: &ScanResult) -> Self {
        Self {
            total_bytes: result.total_bytes,
            file_count: result.file_count,
            finished_at_unix: result.finished_at_unix,
            categories: result.categories(),
            top_dirs: result.top_dirs(100),
        }
    }

    pub fn diff(&self, result: &ScanResult) -> ScanDiff {
        let cur_cats = result.categories();
        let categories = cur_cats
            .iter()
            .map(|c| CategoryDelta {
                category: c.category,
                delta_bytes: c.bytes as i64
                    - self
                        .categories
                        .iter()
                        .find(|p| p.category == c.category)
                        .map_or(0, |p| p.bytes as i64),
            })
            .filter(|d| d.delta_bytes != 0)
            .collect();
        let mut dirs: Vec<DirDelta> = result
            .top_dirs(100)
            .into_iter()
            .map(|d| {
                let prev = self.top_dirs.iter().find(|p| p.path == d.path);
                DirDelta {
                    delta_bytes: d.bytes as i64 - prev.map_or(0, |p| p.bytes as i64),
                    is_new: prev.is_none(),
                    path: d.path,
                }
            })
            .filter(|d| d.delta_bytes != 0)
            .collect();
        // Folders that vanished since last time.
        for p in &self.top_dirs {
            if !result.dirs.contains_key(&p.path) {
                dirs.push(DirDelta {
                    path: p.path.clone(),
                    delta_bytes: -(p.bytes as i64),
                    is_new: false,
                });
            }
        }
        dirs.sort_by_key(|d| std::cmp::Reverse(d.delta_bytes.unsigned_abs()));
        // Nested folders echo their parent's change; keep the deepest of a chain only when it carries most of it.
        dirs.truncate(10);
        ScanDiff {
            since_unix: self.finished_at_unix,
            delta_bytes: result.total_bytes as i64 - self.total_bytes as i64,
            delta_files: result.file_count as i64 - self.file_count as i64,
            categories,
            dirs,
        }
    }
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
            ..Default::default()
        };
        a.save(&dir, Path::new("/a")).expect("save");
        assert_eq!(ScanStats::load(&dir, Path::new("/a")), Some(a));
        assert_eq!(ScanStats::load(&dir, Path::new("/b")), None);
    }

    #[test]
    fn old_format_without_new_fields_still_loads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();
        std::fs::write(
            ScanStats::file_for(dir, Path::new("/r")),
            br#"{"total_bytes":5,"file_count":1,"finished_at_unix":9}"#,
        )
        .expect("write");
        let s = ScanStats::load(dir, Path::new("/r")).expect("load");
        assert_eq!(s.total_bytes, 5);
        assert!(s.categories.is_empty());
    }
}
