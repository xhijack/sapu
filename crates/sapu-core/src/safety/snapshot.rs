//! A file's identity as observed at scan time, used to re-validate (S5)
//! right before disposal.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::{Result, SapuError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    pub is_symlink: bool,
    pub is_dir: bool,
}

impl FileSnapshot {
    /// Capture metadata without following symlinks (S6).
    pub fn capture(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let meta = std::fs::symlink_metadata(path)
            .map_err(|e| SapuError::io(format!("stat {}", path.display()), e))?;
        Ok(Self {
            path: path.to_path_buf(),
            size: meta.len(),
            mtime: meta.modified().ok(),
            is_symlink: meta.file_type().is_symlink(),
            is_dir: meta.is_dir(),
        })
    }
}
