//! Turns ticked groups into concrete, validated moves.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::group::{GroupSource, OrganizePlan, sanitize_name};
use super::scan::{Unit, plain_folder_or_absent};
use crate::error::{Result, SapuError};
use crate::safety::{FileSnapshot, ProtectedPaths};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Move {
    pub from: PathBuf,
    pub to: PathBuf,
    pub snapshot: FileSnapshot,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MovePlan {
    pub root: PathBuf,
    pub moves: Vec<Move>,
    /// Entries left out with the reason (protected, unreadable, link).
    pub skipped: Vec<(PathBuf, String)>,
}

/// What the person ticked: groups to apply, single files to leave alone,
/// and new names for groups.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Selection {
    pub group_ids: Vec<String>,
    pub excluded: Vec<PathBuf>,
    pub renames: Vec<(String, String)>,
}

fn invalid(msg: &str) -> SapuError {
    SapuError::io(
        "organize plan",
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string()),
    )
}

/// `name.ext` → `name (n).ext`, folders get the suffix on the whole name.
fn numbered(name: &str, n: usize, is_dir: bool) -> String {
    match (is_dir, name.rsplit_once('.')) {
        (false, Some((stem, ext))) if !stem.is_empty() => format!("{stem} ({n}).{ext}"),
        _ => format!("{name} ({n})"),
    }
}

/// Builds the moves for the ticked groups. Destinations are always
/// `root/<group>/<name>`; clashes get ` (2)`, ` (3)`, …
pub fn build_move_plan(
    plan: &OrganizePlan,
    selection: &Selection,
    protected: &ProtectedPaths,
) -> Result<MovePlan> {
    let root = std::path::absolute(&plan.root).map_err(|e| SapuError::io("resolve root", e))?;
    let excluded: HashSet<&Path> = selection.excluded.iter().map(PathBuf::as_path).collect();
    let mut moves = Vec::new();
    let mut skipped = Vec::new();
    let mut taken: HashSet<PathBuf> = HashSet::new();

    for g in plan
        .groups
        .iter()
        .filter(|g| selection.group_ids.contains(&g.id))
    {
        let fixed = matches!(g.source, GroupSource::Existing | GroupSource::Unit);
        let name = match selection
            .renames
            .iter()
            .filter(|_| !fixed)
            .find(|(id, _)| *id == g.id)
        {
            Some((_, raw)) => {
                sanitize_name(raw).ok_or_else(|| invalid("group name is not a safe folder name"))?
            }
            None => sanitize_name(&g.name)
                .ok_or_else(|| invalid("group name is not a safe folder name"))?,
        };
        let dest_dir = root.join(&name);
        if !dest_dir.starts_with(&root) || dest_dir == root {
            return Err(invalid("destination escapes the folder"));
        }
        // A link, a file, a project or a bundle carrying the group's name
        // cannot receive files: it would open a project or leave the folder.
        let dest_ok = plain_folder_or_absent(&dest_dir);
        for e in &g.entries {
            if !dest_ok && e.path != dest_dir {
                skipped.push((
                    e.path.clone(),
                    format!("destination {} is not a plain folder", dest_dir.display()),
                ));
                continue;
            }
            if e.name.contains('/') || e.name.contains('\\') || e.name == ".." || e.name.is_empty()
            {
                skipped.push((e.path.clone(), "unsafe entry name".into()));
                continue;
            }
            if excluded.contains(e.path.as_path()) || e.unit == Unit::Link {
                continue;
            }
            // Already where it belongs (existing folder's own entry, or file already inside).
            if e.path == dest_dir || e.path.parent() == Some(dest_dir.as_path()) {
                continue;
            }
            if protected.is_protected(&e.path) {
                skipped.push((e.path.clone(), "protected".into()));
                continue;
            }
            let snapshot = match FileSnapshot::capture(&e.path) {
                Ok(s) => s,
                Err(err) => {
                    skipped.push((e.path.clone(), err.to_string()));
                    continue;
                }
            };
            let mut to = dest_dir.join(&e.name);
            let mut n = 2;
            while to.exists() || taken.contains(&to) {
                to = dest_dir.join(numbered(&e.name, n, e.unit != Unit::File));
                n += 1;
            }
            taken.insert(to.clone());
            moves.push(Move {
                from: e.path.clone(),
                to,
                snapshot,
                bytes: e.bytes,
            });
        }
    }
    Ok(MovePlan {
        root,
        moves,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{propose, scan_folder};
    use crate::safety::{ProtectedPaths, Protection};
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, b"x").expect("write");
    }
    fn plan_for(root: &Path) -> OrganizePlan {
        propose(&scan_folder(root).expect("scan"))
    }
    fn all_ids(p: &OrganizePlan) -> Vec<String> {
        p.groups.iter().map(|g| g.id.clone()).collect()
    }

    #[test]
    fn moves_go_under_root_slash_group_and_existing_folders_produce_no_moves() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/old.pdf"));
        touch(&t.path().join("a.pdf"));
        touch(&t.path().join("clip.mp4"));
        let plan = plan_for(t.path());
        let sel = Selection {
            group_ids: all_ids(&plan),
            excluded: vec![],
            renames: vec![],
        };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        let targets: Vec<_> = mp
            .moves
            .iter()
            .map(|m| {
                m.to.strip_prefix(t.path())
                    .expect("under root")
                    .to_path_buf()
            })
            .collect();
        assert!(targets.contains(&Path::new("Dokumen/a.pdf").to_path_buf()));
        assert!(targets.contains(&Path::new("Video/clip.mp4").to_path_buf()));
        assert!(
            mp.moves.iter().all(|m| m.from != t.path().join("Dokumen")),
            "existing folder itself is not moved"
        );
    }

    #[test]
    fn name_clashes_get_a_numbered_suffix() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/a.pdf"));
        touch(&t.path().join("a.pdf"));
        let plan = plan_for(t.path());
        let sel = Selection {
            group_ids: all_ids(&plan),
            excluded: vec![],
            renames: vec![],
        };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        let m = mp
            .moves
            .iter()
            .find(|m| m.from == t.path().join("a.pdf"))
            .expect("move");
        assert_eq!(m.to, t.path().join("Dokumen/a (2).pdf"));
    }

    #[test]
    fn unticked_groups_excluded_files_and_renames_are_respected() {
        let t = tempdir();
        for n in ["a.pdf", "b.pdf", "clip.mp4"] {
            touch(&t.path().join(n));
        }
        let plan = plan_for(t.path());
        let docs = plan
            .groups
            .iter()
            .find(|g| g.name == "Dokumen")
            .expect("docs")
            .id
            .clone();
        let sel = Selection {
            group_ids: vec![docs.clone()],
            excluded: vec![t.path().join("b.pdf")],
            renames: vec![(docs, "Berkas".into())],
        };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        assert_eq!(mp.moves.len(), 1);
        assert_eq!(mp.moves[0].to, t.path().join("Berkas/a.pdf"));
    }

    #[test]
    fn bad_rename_and_protected_source_are_refused() {
        let t = tempdir();
        for n in ["a.pdf", "b.pdf"] {
            touch(&t.path().join(n));
        }
        let plan = plan_for(t.path());
        let docs = plan.groups[0].id.clone();
        let bad = Selection {
            group_ids: vec![docs.clone()],
            excluded: vec![],
            renames: vec![(docs.clone(), "../out".into())],
        };
        assert!(build_move_plan(&plan, &bad, &ProtectedPaths::os_defaults()).is_err());
        let mut protected = ProtectedPaths::os_defaults();
        protected.add(t.path().join("a.pdf"), Protection::Exact, "test");
        let sel = Selection {
            group_ids: vec![docs],
            excluded: vec![],
            renames: vec![],
        };
        let mp = build_move_plan(&plan, &sel, &protected).expect("plan");
        assert_eq!(mp.moves.len(), 1, "protected file is skipped, not planned");
        assert_eq!(mp.skipped.len(), 1);
    }

    #[test]
    fn destination_that_is_a_link_file_or_project_is_refused() {
        let t = tempdir();
        // A symlinked "Dokumen" pointing outside, a plain file "Arsip", a project "Kode".
        std::fs::create_dir_all(t.path().join("outside")).expect("outside");
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.path().join("outside"), t.path().join("Dokumen"))
            .expect("symlink");
        touch(&t.path().join("a.pdf"));
        touch(&t.path().join("Arsip"));
        touch(&t.path().join("x.zip"));
        touch(&t.path().join("Kode/package.json"));
        touch(&t.path().join("main.rs"));
        let plan = plan_for(t.path());
        let sel = Selection {
            group_ids: all_ids(&plan),
            excluded: vec![],
            renames: vec![],
        };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        for name in ["a.pdf", "x.zip", "main.rs"] {
            assert!(
                mp.moves.iter().all(|m| m.from != t.path().join(name)),
                "{name} must not be planned into a non-folder destination"
            );
            assert!(
                mp.skipped
                    .iter()
                    .any(|(p, why)| p == &t.path().join(name) && why.contains("destination")),
                "{name} skipped with a destination reason: {:?}",
                mp.skipped
            );
        }
        assert!(
            mp.moves
                .iter()
                .all(|m| !m.to.starts_with(t.path().join("outside")))
        );
    }
}
