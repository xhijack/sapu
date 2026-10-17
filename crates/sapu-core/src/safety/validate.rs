//! Re-validation right before disposal (S5).

use std::path::Path;

use super::protected::{best_effort_canonical, lexical_absolute};
use super::snapshot::FileSnapshot;

/// Why an item failed re-validation. Rendered by the UI layer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValidationFailure {
    Missing,
    OutsideScope,
    TypeChanged,
    SizeChanged {
        expected: u64,
        actual: u64,
    },
    MtimeChanged,
    SymlinkTargetOutsideScope,
    Unreadable {
        message: String,
    },
    /// The current user cannot remove it (owned by another user or root,
    /// or inside a folder that is not writable). Skipped, never prompted for.
    NoPermission {
        message: String,
    },
}

impl std::fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "file no longer exists"),
            Self::OutsideScope => write!(f, "path is outside the plan scope"),
            Self::TypeChanged => write!(f, "file type changed since scan"),
            Self::SizeChanged { expected, actual } => {
                write!(f, "size changed since scan ({expected} -> {actual})")
            }
            Self::MtimeChanged => write!(f, "modification time changed since scan"),
            Self::SymlinkTargetOutsideScope => write!(f, "symlink points outside the plan scope"),
            Self::Unreadable { message } => write!(f, "could not read metadata: {message}"),
            Self::NoPermission { message } => write!(f, "no permission to remove: {message}"),
        }
    }
}

/// Checks that `snapshot` still describes what is on disk and that the item
/// lies inside `scope`. Never follows the symlink when reading metadata.
pub fn revalidate(snapshot: &FileSnapshot, scope: &Path) -> Result<(), ValidationFailure> {
    let scope_abs = lexical_absolute(scope);
    let path_abs = lexical_absolute(&snapshot.path);
    if !path_abs.starts_with(&scope_abs) {
        return Err(ValidationFailure::OutsideScope);
    }

    let meta = match std::fs::symlink_metadata(&snapshot.path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ValidationFailure::Missing);
        }
        Err(e) => {
            return Err(ValidationFailure::Unreadable {
                message: e.to_string(),
            });
        }
    };

    let ft = meta.file_type();
    if ft.is_symlink() != snapshot.is_symlink
        || (!ft.is_symlink() && meta.is_dir() != snapshot.is_dir)
    {
        return Err(ValidationFailure::TypeChanged);
    }
    if meta.len() != snapshot.size {
        return Err(ValidationFailure::SizeChanged {
            expected: snapshot.size,
            actual: meta.len(),
        });
    }
    if snapshot.mtime.is_some() && meta.modified().ok() != snapshot.mtime {
        return Err(ValidationFailure::MtimeChanged);
    }

    if ft.is_symlink() {
        let scope_canon = best_effort_canonical(&scope_abs);
        let raw =
            std::fs::read_link(&snapshot.path).map_err(|e| ValidationFailure::Unreadable {
                message: e.to_string(),
            })?;
        // `join` keeps absolute targets as-is and resolves relative ones
        // against the link's parent directory.
        let link_dir = path_abs.parent().unwrap_or(&path_abs);
        let target = best_effort_canonical(&lexical_absolute(&link_dir.join(raw)));
        if !target.starts_with(&scope_canon) && !target.starts_with(&scope_abs) {
            return Err(ValidationFailure::SymlinkTargetOutsideScope);
        }
    }
    Ok(())
}

/// Can this process remove `path` without elevated rights? Removing means
/// renaming out of the parent folder, so the parent must be writable; a
/// directory must also be writable itself so its entries can move. On
/// Unix this is decided from ownership and mode bits; elsewhere the
/// operation itself reports the error.
pub fn check_removable(path: &Path) -> Result<(), ValidationFailure> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // The process's own ids, read from the home folder it owns; no
        // unsafe libc call. Unknown ids mean "let the operation decide".
        let Some((uid, gid)) = dirs::home_dir()
            .and_then(|h| std::fs::metadata(h).ok())
            .map(|m| (m.uid(), m.gid()))
        else {
            return Ok(());
        };
        if uid == 0 {
            return Ok(());
        }
        let writable = |m: &std::fs::Metadata| {
            let mode = m.mode();
            if m.uid() == uid {
                mode & 0o200 != 0
            } else if m.gid() == gid {
                mode & 0o020 != 0
            } else {
                mode & 0o002 != 0
            }
        };
        if let Some(parent) = path.parent() {
            let pm =
                std::fs::symlink_metadata(parent).map_err(|e| ValidationFailure::Unreadable {
                    message: e.to_string(),
                })?;
            if !writable(&pm) {
                return Err(ValidationFailure::NoPermission {
                    message: format!("parent folder {} is not writable", parent.display()),
                });
            }
        }
        let m = std::fs::symlink_metadata(path).map_err(|e| ValidationFailure::Unreadable {
            message: e.to_string(),
        })?;
        if m.is_dir() && !writable(&m) {
            return Err(ValidationFailure::NoPermission {
                message: "folder is not writable".into(),
            });
        }
        if m.uid() != uid && !m.file_type().is_symlink() && !writable(&m) {
            return Err(ValidationFailure::NoPermission {
                message: "owned by another user".into(),
            });
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(path: &Path, bytes: &[u8]) {
        let mut f = std::fs::File::create(path).expect("create");
        f.write_all(bytes).expect("write");
        f.sync_all().expect("sync");
    }

    #[test]
    fn unchanged_file_passes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.txt");
        write(&p, b"hello");
        let snap = FileSnapshot::capture(&p).expect("capture");
        assert_eq!(revalidate(&snap, tmp.path()), Ok(()));
    }

    #[test]
    fn missing_file_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.txt");
        write(&p, b"hello");
        let snap = FileSnapshot::capture(&p).expect("capture");
        std::fs::rename(&p, tmp.path().join("b.txt")).expect("rename");
        assert_eq!(
            revalidate(&snap, tmp.path()),
            Err(ValidationFailure::Missing)
        );
    }

    #[test]
    fn size_change_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.txt");
        write(&p, b"hello");
        let snap = FileSnapshot::capture(&p).expect("capture");
        write(&p, b"hello world");
        assert!(matches!(
            revalidate(&snap, tmp.path()),
            Err(ValidationFailure::SizeChanged {
                expected: 5,
                actual: 11
            })
        ));
    }

    #[test]
    fn mtime_change_with_same_size_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.txt");
        write(&p, b"hello");
        let snap = FileSnapshot::capture(&p).expect("capture");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        std::fs::File::options()
            .write(true)
            .open(&p)
            .expect("open")
            .set_modified(later)
            .expect("set mtime");
        assert_eq!(
            revalidate(&snap, tmp.path()),
            Err(ValidationFailure::MtimeChanged)
        );
    }

    #[test]
    fn outside_scope_fails_before_touching_disk() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.txt");
        write(&p, b"x");
        let snap = FileSnapshot::capture(&p).expect("capture");
        let scope = tmp.path().join("sub");
        assert_eq!(
            revalidate(&snap, &scope),
            Err(ValidationFailure::OutsideScope)
        );
    }

    #[test]
    fn file_replaced_by_directory_fails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a");
        write(&p, b"x");
        let snap = FileSnapshot::capture(&p).expect("capture");
        std::fs::rename(&p, tmp.path().join("gone")).expect("rename");
        std::fs::create_dir(&p).expect("mkdir");
        assert_eq!(
            revalidate(&snap, tmp.path()),
            Err(ValidationFailure::TypeChanged)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_outside_scope_fails_inside_scope_passes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("tempdir2");
        let target_out = outside.path().join("secret");
        write(&target_out, b"x");
        let target_in = tmp.path().join("inside");
        write(&target_in, b"x");

        let bad = tmp.path().join("bad-link");
        std::os::unix::fs::symlink(&target_out, &bad).expect("symlink");
        let good = tmp.path().join("good-link");
        std::os::unix::fs::symlink("inside", &good).expect("symlink");

        let bad_snap = FileSnapshot::capture(&bad).expect("capture");
        let good_snap = FileSnapshot::capture(&good).expect("capture");
        assert!(bad_snap.is_symlink);
        assert_eq!(
            revalidate(&bad_snap, tmp.path()),
            Err(ValidationFailure::SymlinkTargetOutsideScope)
        );
        assert_eq!(revalidate(&good_snap, tmp.path()), Ok(()));
    }
}
