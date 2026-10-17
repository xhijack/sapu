//! Turns ticked groups into concrete, validated moves.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::group::{Confidence, GroupSource, OrganizePlan, sanitize_name};
use super::scan::{OrganizeEntry, Unit, plain_folder_or_absent};
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
    /// New names for groups (destination folders).
    pub renames: Vec<(String, String)>,
    /// New names for single files, accepted by the person. Files only; the
    /// original extension is always kept.
    pub file_renames: Vec<(PathBuf, String)>,
}

/// What is ticked when nobody has looked yet (the CLI's `--apply`, the
/// app's first view): sure groups whose moves are files only. Groups that
/// would move a folder, project or app wait for the person.
pub fn default_selection(plan: &OrganizePlan) -> Selection {
    let group_ids = plan
        .groups
        .iter()
        .filter(|g| g.confidence == Confidence::Sure)
        .filter(|g| {
            let own = plan.root.join(&g.name);
            let moving: Vec<_> = g.entries.iter().filter(|e| e.path != own).collect();
            !moving.is_empty() && moving.iter().all(|e| e.unit == Unit::File)
        })
        .map(|g| g.id.clone())
        .collect();
    Selection {
        group_ids,
        ..Selection::default()
    }
}

/// A safe new file name that keeps the original extension.
pub fn renamed_file(original: &str, proposed: &str) -> Option<String> {
    let name = sanitize_name(proposed)?;
    let ext = original
        .rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty());
    let named = match ext {
        Some(ext)
            if !name
                .to_lowercase()
                .ends_with(&format!(".{}", ext.to_lowercase())) =>
        {
            format!("{name}.{ext}")
        }
        _ => name,
    };
    // A case-only change is no rename: on a case-insensitive disk the new
    // name "exists" already and would end up as "A (2).pdf".
    (named.to_lowercase() != original.to_lowercase()).then_some(named)
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

/// One move of `e` to `dir/name`, or a skip with its reason.
fn plan_one(
    e: &OrganizeEntry,
    dir: &Path,
    name: &str,
    protected: &ProtectedPaths,
    taken: &mut HashSet<PathBuf>,
    skipped: &mut Vec<(PathBuf, String)>,
) -> Option<Move> {
    if e.name.contains('/') || e.name.contains('\\') || e.name == ".." || e.name.is_empty() {
        skipped.push((e.path.clone(), "unsafe entry name".into()));
        return None;
    }
    if protected.is_protected(&e.path) {
        skipped.push((e.path.clone(), "protected".into()));
        return None;
    }
    let snapshot = match FileSnapshot::capture(&e.path) {
        Ok(s) => s,
        Err(err) => {
            skipped.push((e.path.clone(), err.to_string()));
            return None;
        }
    };
    let mut to = dir.join(name);
    let mut n = 2;
    while to.exists() || taken.contains(&to) {
        to = dir.join(numbered(name, n, e.unit != Unit::File));
        n += 1;
    }
    taken.insert(to.clone());
    Some(Move {
        from: e.path.clone(),
        to,
        snapshot,
        bytes: e.bytes,
    })
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
    let mut handled: HashSet<PathBuf> = HashSet::new();

    // Accepted file renames, keyed by path, files only.
    let all_entries: Vec<&OrganizeEntry> = plan
        .groups
        .iter()
        .flat_map(|g| g.entries.iter())
        .chain(plan.unplaced.iter())
        .collect();
    let new_name = |e: &OrganizeEntry| -> Option<String> {
        if e.unit != Unit::File {
            return None;
        }
        selection
            .file_renames
            .iter()
            .find(|(p, _)| *p == e.path)
            .and_then(|(_, n)| renamed_file(&e.name, n))
    };

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
            // Tried by a ticked group: the in-place pass below leaves it alone.
            if !excluded.contains(e.path.as_path()) {
                handled.insert(e.path.clone());
            }
            if !dest_ok && e.path != dest_dir {
                skipped.push((
                    e.path.clone(),
                    format!("destination {} is not a plain folder", dest_dir.display()),
                ));
                continue;
            }
            if excluded.contains(e.path.as_path()) || e.unit == Unit::Link {
                continue;
            }
            let rename = new_name(e);
            // Already where it belongs (existing folder's own entry, or file already inside).
            if e.path == dest_dir
                || (e.path.parent() == Some(dest_dir.as_path()) && rename.is_none())
            {
                continue;
            }
            let target = rename.unwrap_or_else(|| e.name.clone());
            if let Some(m) = plan_one(e, &dest_dir, &target, protected, &mut taken, &mut skipped) {
                moves.push(m);
            }
        }
    }

    // Renames of entries that no ticked group moves: rename where they are.
    for e in all_entries {
        if handled.contains(&e.path) || excluded.contains(e.path.as_path()) {
            continue;
        }
        let Some(target) = new_name(e) else { continue };
        let Some(here) = e.path.parent() else {
            continue;
        };
        if !here.starts_with(&root) {
            continue;
        }
        if let Some(m) = plan_one(e, here, &target, protected, &mut taken, &mut skipped) {
            handled.insert(e.path.clone());
            moves.push(m);
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
            file_renames: vec![],
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
            file_renames: vec![],
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
            file_renames: vec![],
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
            file_renames: vec![],
        };
        assert!(build_move_plan(&plan, &bad, &ProtectedPaths::os_defaults()).is_err());
        let mut protected = ProtectedPaths::os_defaults();
        protected.add(t.path().join("a.pdf"), Protection::Exact, "test");
        let sel = Selection {
            group_ids: vec![docs],
            excluded: vec![],
            renames: vec![],
            file_renames: vec![],
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
            file_renames: vec![],
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

    fn sel_with_renames(ids: Vec<String>, renames: Vec<(PathBuf, String)>) -> Selection {
        Selection {
            group_ids: ids,
            excluded: vec![],
            renames: vec![],
            file_renames: renames,
        }
    }

    #[test]
    fn file_renames_apply_in_place_and_while_moving_and_keep_the_extension() {
        let t = tempdir();
        touch(&t.path().join("a.pdf"));
        touch(&t.path().join("clip.mp4"));
        let mut plan = plan_for(t.path());
        // One file stays where it is (not in any ticked group).
        let stay = plan
            .groups
            .iter()
            .position(|g| g.name == "Video")
            .expect("video");
        let video = plan.groups.remove(stay);
        plan.unplaced.extend(video.entries);
        let docs = plan
            .groups
            .iter()
            .find(|g| g.name == "Dokumen")
            .expect("docs")
            .id
            .clone();
        let sel = sel_with_renames(
            vec![docs],
            vec![
                (t.path().join("a.pdf"), "Surat Kuasa".into()),
                (t.path().join("clip.mp4"), "Rekaman Rapat.mov".into()),
            ],
        );
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        let to = |from: &str| {
            mp.moves
                .iter()
                .find(|m| m.from == t.path().join(from))
                .map(|m| m.to.clone())
        };
        assert_eq!(
            to("a.pdf"),
            Some(t.path().join("Dokumen/Surat Kuasa.pdf")),
            "extension kept"
        );
        assert_eq!(
            to("clip.mp4"),
            Some(t.path().join("Rekaman Rapat.mov.mp4")),
            "renamed in place, own extension kept"
        );
    }

    #[test]
    fn renames_of_folders_and_unsafe_names_are_ignored() {
        let t = tempdir();
        touch(&t.path().join("Arsip Lama/x.txt"));
        touch(&t.path().join("b.pdf"));
        let plan = plan_for(t.path());
        let sel = sel_with_renames(
            vec![],
            vec![
                (t.path().join("Arsip Lama"), "Arsip".into()),
                (t.path().join("b.pdf"), "../escape".into()),
            ],
        );
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::os_defaults()).expect("plan");
        assert!(mp.moves.is_empty(), "{:?}", mp.moves);
    }

    #[test]
    fn case_only_renames_are_no_change() {
        assert_eq!(renamed_file("a.pdf", "A"), None);
        assert_eq!(renamed_file("a.PDF", "a.pdf"), None);
        assert_eq!(renamed_file("a.pdf", "b"), Some("b.pdf".into()));
    }

    #[test]
    fn a_file_skipped_in_a_ticked_group_is_not_renamed_in_place_and_counted_once() {
        let t = tempdir();
        touch(&t.path().join("a.pdf"));
        touch(&t.path().join("b.pdf"));
        let mut plan = plan_for(t.path());
        let docs = plan
            .groups
            .iter()
            .position(|g| g.name == "Dokumen")
            .expect("docs");
        // The group's destination exists but is a project: nothing may go in.
        touch(&t.path().join("Dokumen/package.json"));
        let id = plan.groups[docs].id.clone();
        plan.groups[docs].entries.retain(|e| e.unit == Unit::File);
        let mut protected = ProtectedPaths::os_defaults();
        protected.add(t.path().join("b.pdf"), Protection::Exact, "test");
        let sel = sel_with_renames(
            vec![id],
            vec![
                (t.path().join("a.pdf"), "Surat".into()),
                (t.path().join("b.pdf"), "Kuasa".into()),
            ],
        );
        let mp = build_move_plan(&plan, &sel, &protected).expect("plan");
        assert!(mp.moves.is_empty(), "{:?}", mp.moves);
        let times = |f: &str| {
            mp.skipped
                .iter()
                .filter(|(p, _)| *p == t.path().join(f))
                .count()
        };
        assert_eq!(times("a.pdf"), 1);
        assert_eq!(times("b.pdf"), 1);
    }

    #[test]
    fn default_selection_takes_sure_groups_that_move_only_files() {
        let t = tempdir();
        touch(&t.path().join("Laporan Keuangan Q1.xlsx"));
        touch(&t.path().join("Laporan Keuangan Q2.xlsx"));
        touch(&t.path().join("Laporan Keuangan Q3.xlsx"));
        touch(&t.path().join("web/package.json"));
        touch(&t.path().join("Pajak/spt.pdf"));
        touch(&t.path().join("Bukti Pajak.pdf"));
        let plan = plan_for(t.path());
        let id = |n: &str| {
            plan.groups
                .iter()
                .find(|g| g.name == n)
                .expect(n)
                .id
                .clone()
        };
        let sel = default_selection(&plan);
        assert!(sel.group_ids.contains(&id("Laporan Keuangan")));
        assert!(
            !sel.group_ids.contains(&id("Proyek")),
            "projects move only when ticked"
        );
        assert!(
            !sel.group_ids.contains(&id("Pajak")),
            "a guess into a folder needs checking"
        );
        assert!(sel.file_renames.is_empty() && sel.renames.is_empty());
    }
}
