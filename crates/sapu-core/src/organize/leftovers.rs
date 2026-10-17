//! Leftovers found without AI: Office lock files and extra exact copies.
//! Only suggestions; removal goes through `safety` like everything else.

use std::path::PathBuf;

use super::scan::{FolderListing, Unit};
use crate::dedupe::{DedupeResult, KeepStrategy, choose_keep};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LeftoverReason {
    /// `~$name.docx`: left behind by Word/Excel/PowerPoint.
    OfficeLock,
    /// Same bytes as `of`, which is the copy that stays.
    Duplicate { of: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Leftover {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
    pub reason: LeftoverReason,
}

/// Office lock files in the folder and its plain subfolders (projects and
/// bundles are never opened), then every copy but one of each duplicate
/// group, the one kept being the name that looks least like a copy.
pub fn find_leftovers(listing: &FolderListing, dupes: Option<&DedupeResult>) -> Vec<Leftover> {
    let mut out = Vec::new();
    let dirs = std::iter::once(listing.root.clone()).chain(
        listing
            .entries
            .iter()
            .filter(|e| e.unit == Unit::Folder)
            .map(|e| e.path.clone()),
    );
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut locks: Vec<Leftover> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let meta = std::fs::symlink_metadata(e.path()).ok()?;
                (name.starts_with("~$") && meta.is_file()).then(|| Leftover {
                    path: e.path(),
                    name,
                    bytes: meta.len(),
                    reason: LeftoverReason::OfficeLock,
                })
            })
            .collect();
        locks.sort_by(|a, b| a.name.cmp(&b.name));
        out.extend(locks);
    }
    for g in dupes.map(|d| d.groups.as_slice()).unwrap_or_default() {
        if g.files.len() < 2 {
            continue;
        }
        let keep = choose_keep(g, &KeepStrategy::OriginalName);
        let Some(kept) = g.files.get(keep) else {
            continue;
        };
        for (i, f) in g.files.iter().enumerate() {
            if i == keep {
                continue;
            }
            out.push(Leftover {
                path: f.path.clone(),
                name: f
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                bytes: g.size,
                reason: LeftoverReason::Duplicate {
                    of: kept.path.clone(),
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{duplicates_in, scan_folder};
    use crate::progress::NoProgress;
    use crate::scanner::ScanControl;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn write(p: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, bytes).expect("write");
    }

    #[test]
    fn office_locks_in_the_folder_and_its_subfolders_but_not_in_projects() {
        let t = tempdir();
        write(&t.path().join("~$ RAMDANI BARU.docx"), b"lock");
        write(&t.path().join("CV/~$CV.docx"), b"lock");
        write(&t.path().join("CV/CV.docx"), b"doc");
        write(&t.path().join("web/package.json"), b"{}");
        write(&t.path().join("web/~$notes.docx"), b"lock");
        let listing = scan_folder(t.path()).expect("scan");
        let mut names: Vec<_> = find_leftovers(&listing, None)
            .into_iter()
            .map(|l| {
                assert_eq!(l.reason, LeftoverReason::OfficeLock);
                l.name
            })
            .collect();
        names.sort();
        assert_eq!(names, vec!["~$ RAMDANI BARU.docx", "~$CV.docx"]);
    }

    #[test]
    fn extra_exact_copies_point_at_the_copy_that_stays() {
        let t = tempdir();
        let body = vec![7u8; 8192];
        write(&t.path().join("KTP/KTP2.jpeg"), &body);
        write(&t.path().join("KTP/KTP2 copy.jpeg"), &body);
        write(&t.path().join("KTP/lain.jpeg"), &vec![1u8; 8192]);
        let listing = scan_folder(t.path()).expect("scan");
        let dupes = duplicates_in(t.path(), &ScanControl::new(), &mut NoProgress).expect("dupes");
        let found = find_leftovers(&listing, Some(&dupes));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "KTP2 copy.jpeg");
        assert_eq!(found[0].bytes, 8192);
        assert!(
            matches!(&found[0].reason, LeftoverReason::Duplicate { of } if of.ends_with("KTP/KTP2.jpeg"))
        );
    }
}
