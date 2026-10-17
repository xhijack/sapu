//! Turns per-item advice (from the AI, through the app or CLI) into a plan.
//! Everything is validated here: unknown names, unsafe names, renames of
//! folders and junk flags on folders are dropped. Core never depends on the
//! AI crate; callers convert its answer into `ItemAdvice`.

use std::collections::{HashMap, HashSet};

use super::group::{Confidence, Group, GroupSource, OrganizePlan, sanitize_name};
use super::plan::renamed_file;
use super::scan::{FolderListing, OrganizeEntry, Unit};

/// What the advisor says about one listed item.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemAdvice {
    /// Exact entry name in the folder.
    pub name: String,
    /// Destination category (a new folder, or an existing folder's name).
    pub category: Option<String>,
    /// Better file name (files only).
    pub rename: Option<String>,
    /// Likely junk (files only).
    pub junk: bool,
    pub reason: String,
    pub confidence: f32,
}

const SURE_AT: f32 = 0.75;

/// Builds a plan from advice. `None` when no advice matches any entry, so
/// the caller keeps the heuristic plan.
pub fn plan_from_advice(listing: &FolderListing, advice: &[ItemAdvice]) -> Option<OrganizePlan> {
    let by_name: HashMap<&str, &ItemAdvice> =
        advice.iter().rev().map(|a| (a.name.as_str(), a)).collect();
    if !listing
        .entries
        .iter()
        .any(|e| by_name.contains_key(e.name.as_str()))
    {
        return None;
    }
    let existing: HashMap<String, &OrganizeEntry> = listing
        .entries
        .iter()
        .filter(|e| e.unit == Unit::Folder)
        .map(|e| (e.name.to_lowercase(), e))
        .collect();

    // Destination per entry, decided first so folders that receive items can be pinned.
    let mut dest: HashMap<&str, String> = HashMap::new();
    for e in &listing.entries {
        if e.unit == Unit::Link {
            continue;
        }
        let Some(a) = by_name.get(e.name.as_str()) else {
            continue;
        };
        if a.junk && e.unit == Unit::File {
            continue;
        }
        let Some(cat) = a.category.as_deref().and_then(sanitize_name) else {
            continue;
        };
        let cat = existing
            .get(&cat.to_lowercase())
            .map(|f| f.name.clone())
            .unwrap_or(cat);
        if cat.eq_ignore_ascii_case(&e.name) {
            continue;
        }
        dest.insert(e.name.as_str(), cat);
    }
    // A folder that receives items stays where it is.
    let receiving: HashSet<String> = dest.values().map(|c| c.to_lowercase()).collect();
    dest.retain(|name, _| {
        !(existing.contains_key(&name.to_lowercase()) && receiving.contains(&name.to_lowercase()))
    });

    let mut groups: Vec<Group> = Vec::new();
    let mut confidences: Vec<Vec<f32>> = Vec::new();
    let mut unplaced = Vec::new();
    for e in &listing.entries {
        if e.unit == Unit::Link {
            continue;
        }
        let a = by_name.get(e.name.as_str()).copied();
        let mut entry = e.clone();
        if let Some(a) = a
            && e.unit == Unit::File
        {
            entry.suggested_name = a.rename.as_deref().and_then(|r| renamed_file(&e.name, r));
            entry.junk = a.junk.then(|| {
                if a.reason.trim().is_empty() {
                    "—".to_string()
                } else {
                    a.reason.trim().to_string()
                }
            });
        }
        let Some(cat) = dest.get(e.name.as_str()) else {
            unplaced.push(entry);
            continue;
        };
        let i = match groups.iter().position(|g| g.name.eq_ignore_ascii_case(cat)) {
            Some(i) => i,
            None => {
                let folder = existing.get(&cat.to_lowercase());
                groups.push(Group {
                    id: format!("g{}", groups.len() + 1),
                    name: cat.clone(),
                    source: if folder.is_some() {
                        GroupSource::Existing
                    } else {
                        GroupSource::Ai
                    },
                    confidence: Confidence::Sure,
                    // An existing folder's own entry comes first, as in `propose`.
                    entries: folder.map(|f| vec![(*f).clone()]).unwrap_or_default(),
                    bytes: folder.map(|f| f.bytes).unwrap_or(0),
                });
                confidences.push(Vec::new());
                groups.len() - 1
            }
        };
        confidences[i].push(a.map(|a| a.confidence.clamp(0.0, 1.0)).unwrap_or(0.5));
        groups[i].bytes += entry.bytes;
        groups[i].entries.push(entry);
    }
    if groups.is_empty()
        && unplaced
            .iter()
            .all(|e| e.suggested_name.is_none() && e.junk.is_none())
    {
        return None;
    }
    // An existing folder pulled into a group as its target no longer "stays".
    let targets: HashSet<String> = groups.iter().map(|g| g.name.to_lowercase()).collect();
    unplaced.retain(|e| !(e.unit == Unit::Folder && targets.contains(&e.name.to_lowercase())));

    for (g, cs) in groups.iter_mut().zip(confidences) {
        let avg = if cs.is_empty() {
            0.0
        } else {
            cs.iter().sum::<f32>() / cs.len() as f32
        };
        let moves_folders = g
            .entries
            .iter()
            .any(|e| e.unit != Unit::File && !e.name.eq_ignore_ascii_case(&g.name));
        g.confidence = if avg >= SURE_AT && !moves_folders {
            Confidence::Sure
        } else {
            Confidence::Check
        };
    }
    Some(OrganizePlan {
        root: listing.root.clone(),
        groups,
        unplaced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{GroupSource, Unit, scan_folder};
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
    fn adv(name: &str, category: Option<&str>) -> ItemAdvice {
        ItemAdvice {
            name: name.into(),
            category: category.map(str::to_string),
            rename: None,
            junk: false,
            reason: String::new(),
            confidence: 0.9,
        }
    }
    /// A small copy of the owner's `Pribadi` folder.
    fn pribadi() -> tempfile::TempDir {
        let t = tempdir();
        let r = t.path();
        touch(&r.join("Saham/analis.xlsx"));
        touch(&r.join("Rekening Koran/Dec2018.pdf"));
        touch(&r.join("KTP/KTP2.jpeg"));
        for n in [
            "My Saham.xlsx",
            "potongan pajak pak dani.pdf",
            "jawaban interview riri.docx",
            "sedelah.xlsx",
            "Zikir pagi & petang.docx",
        ] {
            touch(&r.join(n));
        }
        t
    }

    #[test]
    fn items_go_to_categories_and_existing_folders_and_folders_can_nest() {
        let t = pribadi();
        let listing = scan_folder(t.path()).expect("scan");
        let plan = plan_from_advice(
            &listing,
            &[
                adv("My Saham.xlsx", Some("saham")),
                adv("potongan pajak pak dani.pdf", Some("Keuangan")),
                adv("Rekening Koran", Some("Keuangan")),
                adv("jawaban interview riri.docx", Some("Pekerjaan")),
                adv("Zikir pagi & petang.docx", Some("Ibadah")),
                adv("KTP", Some("KTP")),
                adv("ghost.pdf", Some("Keuangan")),
            ],
        )
        .expect("plan");
        let group = |n: &str| {
            plan.groups.iter().find(|g| g.name == n).unwrap_or_else(|| {
                panic!(
                    "{n}: {:?}",
                    plan.groups.iter().map(|g| &g.name).collect::<Vec<_>>()
                )
            })
        };
        let names = |n: &str| {
            group(n)
                .entries
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        };
        // Category matching an existing folder (case-insensitive) is that folder.
        assert_eq!(group("Saham").source, GroupSource::Existing);
        assert!(names("Saham").contains(&"My Saham.xlsx".to_string()));
        // A folder can move into a new category; such a group is never "sure".
        assert!(names("Keuangan").contains(&"Rekening Koran".to_string()));
        assert_eq!(
            group("Keuangan").confidence,
            crate::organize::Confidence::Check
        );
        assert_eq!(
            group("Pekerjaan").confidence,
            crate::organize::Confidence::Sure
        );
        // A folder assigned to itself stays; unknown names are ignored; unmentioned items stay.
        let stays: Vec<_> = plan.unplaced.iter().map(|e| e.name.as_str()).collect();
        assert!(stays.contains(&"KTP"));
        assert!(stays.contains(&"sedelah.xlsx"));
        assert!(
            plan.groups
                .iter()
                .all(|g| g.entries.iter().all(|e| e.name != "ghost.pdf"))
        );
        assert!(
            plan.groups
                .iter()
                .all(|g| g.entries.iter().all(|e| e.unit != Unit::Link))
        );
    }

    #[test]
    fn a_folder_that_receives_items_is_not_moved_itself() {
        let t = pribadi();
        let listing = scan_folder(t.path()).expect("scan");
        let plan = plan_from_advice(
            &listing,
            &[
                adv("Saham", Some("Keuangan")),
                adv("My Saham.xlsx", Some("Saham")),
            ],
        )
        .expect("plan");
        assert!(
            plan.groups
                .iter()
                .all(|g| g.name != "Keuangan" || g.entries.iter().all(|e| e.name != "Saham"))
        );
        assert!(
            plan.unplaced.iter().any(|e| e.name == "Saham")
                || plan.groups.iter().any(|g| g.name == "Saham")
        );
    }

    #[test]
    fn renames_and_junk_only_for_files_and_sanitised() {
        let t = pribadi();
        let listing = scan_folder(t.path()).expect("scan");
        let mut a = adv("sedelah.xlsx", None);
        a.rename = Some("Daftar Sedekah".into());
        let mut j = adv("Zikir pagi & petang.docx", None);
        j.junk = true;
        j.reason = "duplicate of another file".into();
        let mut f = adv("Saham", None);
        f.rename = Some("Investasi".into());
        f.junk = true;
        let mut bad = adv("My Saham.xlsx", None);
        bad.rename = Some("../../etc/passwd".into());
        let plan = plan_from_advice(&listing, &[a, j, f, bad]).expect("plan");
        let entry = |n: &str| {
            plan.groups
                .iter()
                .flat_map(|g| g.entries.iter())
                .chain(plan.unplaced.iter())
                .find(|e| e.name == n)
                .expect(n)
                .clone()
        };
        assert_eq!(
            entry("sedelah.xlsx").suggested_name.as_deref(),
            Some("Daftar Sedekah.xlsx")
        );
        assert_eq!(
            entry("Zikir pagi & petang.docx").junk.as_deref(),
            Some("duplicate of another file")
        );
        assert_eq!(
            entry("Saham").suggested_name,
            None,
            "folders are not renamed"
        );
        assert_eq!(entry("Saham").junk, None, "folders are never junk");
        assert_eq!(
            entry("My Saham.xlsx").suggested_name,
            None,
            "unsafe rename dropped"
        );
    }

    #[test]
    fn advice_that_matches_nothing_gives_no_plan() {
        let t = pribadi();
        let listing = scan_folder(t.path()).expect("scan");
        assert!(plan_from_advice(&listing, &[adv("nope.pdf", Some("X"))]).is_none());
        assert!(plan_from_advice(&listing, &[]).is_none());
    }
}
