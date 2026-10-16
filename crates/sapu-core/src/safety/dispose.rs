//! The ONLY file in the workspace that may call a removal API.
//!
//! `clippy.toml` disallows `std::fs::remove_*` and `trash::delete*`
//! everywhere; this file opts out, and `scripts/check-forbidden-fs.sh`
//! fails CI if that opt-out or those calls appear anywhere else.
#![allow(clippy::disallowed_methods)]

use std::io;
use std::path::Path;

/// Proof that the caller obtained an explicit, second confirmation from the
/// user for permanent deletion (S1). Cannot be constructed implicitly.
#[derive(Debug, Clone, Copy)]
pub struct PermanentConfirmation(());

impl PermanentConfirmation {
    /// Construct only after the user confirmed twice in the UI/CLI.
    pub fn i_understand_this_cannot_be_undone() -> Self {
        Self(())
    }
}

/// How items in a plan leave the disk.
#[derive(Debug, Clone, Copy, Default)]
pub enum Disposal {
    /// Move to the OS trash / recycle bin (default).
    #[default]
    Trash,
    /// Delete permanently. Requires a [`PermanentConfirmation`].
    Permanent(PermanentConfirmation),
}

impl Disposal {
    pub fn kind(&self) -> DisposalKind {
        match self {
            Self::Trash => DisposalKind::Trash,
            Self::Permanent(_) => DisposalKind::Permanent,
        }
    }
}

/// Serializable flavour of [`Disposal`] for history records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisposalKind {
    Trash,
    Permanent,
}

/// Remove `path` from the disk using `disposal`. Callers must have passed
/// the protected-path check and re-validation first; this function does
/// not repeat them.
pub(super) fn dispose(path: &Path, disposal: &Disposal) -> io::Result<()> {
    match disposal {
        Disposal::Trash => trash::delete(path).map_err(io::Error::other),
        Disposal::Permanent(_) => {
            let meta = std::fs::symlink_metadata(path)?;
            if meta.is_dir() {
                // Does not follow symlinks inside the tree.
                std::fs::remove_dir_all(path)
            } else {
                // Regular files and symlinks (removes the link, not its target).
                std::fs::remove_file(path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permanent() -> Disposal {
        Disposal::Permanent(PermanentConfirmation::i_understand_this_cannot_be_undone())
    }

    #[test]
    fn default_is_trash() {
        assert_eq!(Disposal::default().kind(), DisposalKind::Trash);
    }

    #[test]
    fn permanent_removes_file_and_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let f = tmp.path().join("f.txt");
        std::fs::write(&f, b"x").expect("write");
        let d = tmp.path().join("d");
        std::fs::create_dir_all(d.join("sub")).expect("mkdir");
        std::fs::write(d.join("sub/x"), b"x").expect("write");

        dispose(&f, &permanent()).expect("file");
        dispose(&d, &permanent()).expect("dir");
        assert!(!f.exists());
        assert!(!d.exists());
        assert!(tmp.path().exists(), "tempdir itself is untouched");
    }

    #[cfg(unix)]
    #[test]
    fn permanent_on_symlink_removes_link_not_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("target");
        std::fs::create_dir_all(target.join("keep")).expect("mkdir");
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        dispose(&link, &permanent()).expect("link");
        assert!(std::fs::symlink_metadata(&link).is_err());
        assert!(target.join("keep").exists());
    }

    /// Touches the real system trash; opt in with `SAPU_TEST_TRASH=1`.
    #[test]
    fn trash_moves_file_out_of_place() {
        if std::env::var_os("SAPU_TEST_TRASH").is_none() {
            return;
        }
        let tmp = tempfile::tempdir().expect("tempdir");
        let f = tmp.path().join("sapu-trash-test.txt");
        std::fs::write(&f, b"x").expect("write");
        dispose(&f, &Disposal::Trash).expect("trash");
        assert!(!f.exists());
    }
}
