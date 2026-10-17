//! Candidate groups from names, kinds and existing folders; names and
//! merges coming from outside (the AI) are applied here under strict rules.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::scan::{FolderListing, OrganizeEntry, Unit};
use crate::scanner::FileKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupSource {
    Existing,
    Prefix,
    Kind,
    Unit,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Sure,
    Check,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub source: GroupSource,
    pub confidence: Confidence,
    pub entries: Vec<OrganizeEntry>,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrganizePlan {
    pub root: PathBuf,
    pub groups: Vec<Group>,
    /// Entries that stay where they are; they may still carry a suggested
    /// name or a junk reason.
    #[serde(default)]
    pub unplaced: Vec<OrganizeEntry>,
}

/// A name (and how sure the namer is) for one group, from the AI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Naming {
    pub id: String,
    pub name: String,
    pub confidence: f32,
}

const MIN_PREFIX_MEMBERS: usize = 3;
const MAX_NAME_LEN: usize = 60;
const MERGE_INTO_EXISTING_MIN_CONFIDENCE: f32 = 0.7;

/// Folder name for a kind group (Indonesian; the UI translates by key, the
/// folder on disk gets this name).
pub fn kind_folder(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Video => "Video",
        FileKind::Audio => "Audio",
        FileKind::Image => "Gambar",
        FileKind::Pdf | FileKind::Document | FileKind::Spreadsheet | FileKind::Presentation => {
            "Dokumen"
        }
        FileKind::Archive => "Arsip",
        FileKind::Installer | FileKind::DiskImage => "Installer",
        FileKind::Code | FileKind::Database | FileKind::Log => "Kode",
        FileKind::Application => "Aplikasi",
        FileKind::Folder => "Folder",
        FileKind::Font
        | FileKind::Library
        | FileKind::VirtualDisk
        | FileKind::Backup
        | FileKind::Other => "Lainnya",
    }
}

/// A single safe folder name: trimmed, no separators, not dotted, ≤ 60 chars.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let s: String = raw
        .trim()
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '\0'))
        .take(MAX_NAME_LEN)
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() || s.starts_with('.') || raw.contains('/') || raw.contains('\\') || s == ".." {
        return None;
    }
    Some(s)
}

/// Name tokens: split on spaces, `-`, `_`, `.`, and camelCase boundaries;
/// digit-only tokens are dropped. Case-insensitive comparison, original
/// case kept for display.
fn tokens(name: &str) -> Vec<String> {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        let boundary =
            c == ' ' || c == '-' || c == '_' || c == '.' || (c.is_uppercase() && prev_lower);
        if boundary && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if !(c == ' ' || c == '-' || c == '_' || c == '.') {
            cur.push(c);
        }
        prev_lower = c.is_lowercase();
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter()
        .filter(|t| !t.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

/// Words of a name in lowercase, for whole-word comparison.
fn lower_tokens(name: &str) -> Vec<String> {
    tokens(name).iter().map(|t| t.to_lowercase()).collect()
}

/// Lowercase letters and digits only: "accel bytes" and "AccelBytes" match.
fn normalised(s: &str) -> String {
    s.rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(s)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn next_id(n: usize) -> String {
    format!("g{n}")
}

/// Deterministic proposal. Every non-link entry lands in exactly one group.
pub fn propose(listing: &FolderListing) -> OrganizePlan {
    let mut groups: Vec<Group> = Vec::new();
    let push = |name: String,
                source: GroupSource,
                confidence: Confidence,
                entries: Vec<OrganizeEntry>,
                groups: &mut Vec<Group>| {
        let bytes = entries.iter().map(|e| e.bytes).sum();
        groups.push(Group {
            id: next_id(groups.len() + 1),
            name,
            source,
            confidence,
            entries,
            bytes,
        });
    };

    // 1. Existing folders, projects, bundles.
    let mut projects = Vec::new();
    let mut bundles = Vec::new();
    let mut files: Vec<&OrganizeEntry> = Vec::new();
    for e in &listing.entries {
        match e.unit {
            Unit::Folder => push(
                e.name.clone(),
                GroupSource::Existing,
                Confidence::Sure,
                vec![e.clone()],
                &mut groups,
            ),
            Unit::Project => projects.push(e.clone()),
            Unit::Bundle => bundles.push(e.clone()),
            Unit::File => files.push(e),
            Unit::Link => {}
        }
    }
    if !projects.is_empty() {
        push(
            "Proyek".into(),
            GroupSource::Unit,
            Confidence::Sure,
            projects,
            &mut groups,
        );
    }
    if !bundles.is_empty() {
        push(
            "Aplikasi".into(),
            GroupSource::Unit,
            Confidence::Sure,
            bundles,
            &mut groups,
        );
    }

    // 1b. A loose file whose name contains the existing folder's name as
    // whole words goes into it ("Bukti Pajak" → "Pajak", not "blueprint" →
    // "Print"). Only a guess, so such a folder is marked to check first.
    let mut into_existing: Vec<(usize, OrganizeEntry)> = Vec::new();
    files.retain(|e| {
        let words = lower_tokens(&e.name);
        let hit = groups.iter().position(|g| {
            g.source == GroupSource::Existing && {
                let folder = lower_tokens(&g.name);
                normalised(&g.name).chars().count() >= 4
                    && !folder.is_empty()
                    && words.windows(folder.len()).any(|w| w == folder.as_slice())
            }
        });
        match hit {
            Some(i) => {
                into_existing.push((i, (*e).clone()));
                false
            }
            None => true,
        }
    });
    for (i, e) in into_existing {
        groups[i].bytes += e.bytes;
        groups[i].confidence = Confidence::Check;
        groups[i].entries.push(e);
    }

    // 2. Shared first token (3+ files).
    let mut by_token: BTreeMap<String, Vec<&OrganizeEntry>> = BTreeMap::new();
    for e in &files {
        if let Some(first) = tokens(&e.name).first() {
            by_token.entry(first.to_lowercase()).or_default().push(e);
        }
    }
    let mut placed: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for members in by_token.values() {
        if members.len() < MIN_PREFIX_MEMBERS {
            continue;
        }
        // Longest run of shared tokens (case-insensitive) decides the name and confidence.
        let token_lists: Vec<Vec<String>> = members.iter().map(|e| tokens(&e.name)).collect();
        let mut shared = 0usize;
        while let Some(first) = token_lists[0].get(shared) {
            if token_lists
                .iter()
                .all(|t| t.get(shared).is_some_and(|x| x.eq_ignore_ascii_case(first)))
            {
                shared += 1;
            } else {
                break;
            }
        }
        // Display case: the spelling most members use; ties go to a
        // capitalised one, so "invoice March" does not rename "Invoice-…".
        let representative = {
            let mut best = 0usize;
            let mut best_score = (0usize, false);
            for (i, t) in token_lists.iter().enumerate() {
                let spelling = &t[0];
                let count = token_lists.iter().filter(|o| o[0] == *spelling).count();
                let capital = spelling.chars().next().is_some_and(char::is_uppercase);
                if (count, capital) > best_score {
                    best_score = (count, capital);
                    best = i;
                }
            }
            best
        };
        let name = token_lists[representative][..shared.max(1)].join(" ");
        let Some(name) = sanitize_name(&name) else {
            continue;
        };
        let confidence = if shared >= 2 {
            Confidence::Sure
        } else {
            Confidence::Check
        };
        let entries: Vec<OrganizeEntry> = members.iter().map(|e| (*e).clone()).collect();
        for e in &entries {
            placed.insert(e.path.clone());
        }
        push(name, GroupSource::Prefix, confidence, entries, &mut groups);
    }

    // 3. Remaining files by kind.
    let mut by_kind: BTreeMap<&'static str, Vec<OrganizeEntry>> = BTreeMap::new();
    for e in files {
        if placed.contains(&e.path) {
            continue;
        }
        by_kind
            .entry(kind_folder(e.kind))
            .or_default()
            .push(e.clone());
    }
    for (name, entries) in by_kind {
        push(
            name.to_string(),
            GroupSource::Kind,
            Confidence::Sure,
            entries,
            &mut groups,
        );
    }

    // Same name as an existing folder: fold into it instead of creating "(2)".
    let mut plan = OrganizePlan {
        root: listing.root.clone(),
        groups,
        unplaced: Vec::new(),
    };
    fold_duplicate_names(&mut plan);
    plan
}

/// Merges groups that share a name (case-insensitive); the existing folder
/// wins the identity, otherwise the first one does.
fn fold_duplicate_names(plan: &mut OrganizePlan) {
    let mut i = 0;
    while i < plan.groups.len() {
        let name = plan.groups[i].name.to_lowercase();
        let mut j = i + 1;
        while j < plan.groups.len() {
            if plan.groups[j].name.to_lowercase() == name {
                let other = plan.groups.remove(j);
                let keep_other_identity = other.source == GroupSource::Existing
                    && plan.groups[i].source != GroupSource::Existing;
                let target = &mut plan.groups[i];
                if keep_other_identity {
                    target.name = other.name;
                    target.source = GroupSource::Existing;
                    target.confidence = Confidence::Sure;
                }
                target.entries.extend(other.entries);
                target.bytes += other.bytes;
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

/// Applies names and merges from outside (the AI). Unknown ids and unsafe
/// names are ignored; a merge into an existing folder needs confidence ≥ 0.7.
pub fn apply_naming(plan: &mut OrganizePlan, names: &[Naming], merges: &[(String, String)]) {
    for n in names {
        let Some(name) = sanitize_name(&n.name) else {
            continue;
        };
        if let Some(g) = plan.groups.iter_mut().find(|g| g.id == n.id) {
            // An existing folder or a unit group is what it is; renaming it
            // would mean moving the folder itself.
            if matches!(g.source, GroupSource::Existing | GroupSource::Unit) {
                continue;
            }
            if g.name != name {
                g.name = name;
                g.source = GroupSource::Ai;
            }
            g.confidence = if n.confidence >= MERGE_INTO_EXISTING_MIN_CONFIDENCE {
                Confidence::Sure
            } else {
                Confidence::Check
            };
        }
    }
    for (from, into) in merges {
        if from == into {
            continue;
        }
        let Some(fi) = plan.groups.iter().position(|g| g.id == *from) else {
            continue;
        };
        if matches!(
            plan.groups[fi].source,
            GroupSource::Existing | GroupSource::Unit
        ) {
            continue;
        }
        let Some(ii) = plan.groups.iter().position(|g| g.id == *into) else {
            continue;
        };
        let from_conf = names
            .iter()
            .find(|n| n.id == *from)
            .map(|n| n.confidence)
            .unwrap_or(0.0);
        if plan.groups[ii].source == GroupSource::Existing
            && from_conf < MERGE_INTO_EXISTING_MIN_CONFIDENCE
        {
            continue;
        }
        let moved = plan.groups.remove(fi);
        let ii = if fi < ii { ii - 1 } else { ii };
        let target = &mut plan.groups[ii];
        target.entries.extend(moved.entries);
        target.bytes += moved.bytes;
        if target.source != GroupSource::Existing {
            target.source = GroupSource::Ai;
        }
    }
    fold_duplicate_names(plan);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::scan_folder;
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
    fn group<'a>(plan: &'a OrganizePlan, name: &str) -> &'a Group {
        plan.groups
            .iter()
            .find(|g| g.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "group {name} in {:?}",
                    plan.groups.iter().map(|g| &g.name).collect::<Vec<_>>()
                )
            })
    }

    #[test]
    fn three_files_with_a_shared_first_token_form_a_prefix_group() {
        let t = tempdir();
        for n in [
            "Invoice-2024-01.pdf",
            "Invoice-2024-02.pdf",
            "invoice March.pdf",
            "photo.jpg",
        ] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let g = group(&plan, "Invoice");
        assert_eq!(g.entries.len(), 3);
        assert_eq!(g.source, GroupSource::Prefix);
        assert_eq!(g.confidence, Confidence::Check, "one shared token only");
        assert_eq!(group(&plan, "Gambar").entries.len(), 1);
    }

    #[test]
    fn two_shared_tokens_are_sure_and_name_keeps_both() {
        let t = tempdir();
        for n in [
            "Laporan Keuangan Q1.xlsx",
            "Laporan Keuangan Q2.xlsx",
            "Laporan Keuangan Q3.xlsx",
        ] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let g = group(&plan, "Laporan Keuangan");
        assert_eq!(g.confidence, Confidence::Sure);
    }

    #[test]
    fn fewer_than_three_fall_back_to_kind_groups() {
        let t = tempdir();
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "clip.mp4", "song.mp3"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert!(plan.groups.iter().all(|g| g.name != "Invoice"));
        assert_eq!(group(&plan, "Dokumen").entries.len(), 2);
        assert_eq!(group(&plan, "Video").entries.len(), 1);
        assert_eq!(group(&plan, "Audio").entries.len(), 1);
    }

    #[test]
    fn existing_folders_projects_bundles_and_links_are_handled() {
        let t = tempdir();
        touch(&t.path().join("Arsip Lama/a.txt"));
        touch(&t.path().join("backend/package.json"));
        touch(&t.path().join("Tool.app/Contents/Info.plist"));
        touch(&t.path().join("x.txt"));
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.path().join("x.txt"), t.path().join("y.txt"))
            .expect("symlink");
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert_eq!(group(&plan, "Arsip Lama").source, GroupSource::Existing);
        assert_eq!(group(&plan, "Proyek").entries[0].name, "backend");
        assert_eq!(group(&plan, "Aplikasi").entries[0].name, "Tool.app");
        assert!(
            plan.groups
                .iter()
                .all(|g| g.entries.iter().all(|e| e.unit != Unit::Link)),
            "links are never grouped"
        );
        let total: usize = plan.groups.iter().map(|g| g.entries.len()).sum();
        assert_eq!(total, 4, "every non-link entry is in exactly one group");
    }

    #[test]
    fn existing_folder_absorbs_kind_group_of_same_name() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/old.pdf"));
        for n in ["a.pdf", "b.docx"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let docs: Vec<_> = plan.groups.iter().filter(|g| g.name == "Dokumen").collect();
        assert_eq!(docs.len(), 1, "one group, not Dokumen and Dokumen (2)");
        assert_eq!(docs[0].source, GroupSource::Existing);
        assert_eq!(docs[0].entries.len(), 3);
    }

    #[test]
    fn names_are_never_empty_or_dotted() {
        assert_eq!(sanitize_name("  Project A  "), Some("Project A".into()));
        assert_eq!(sanitize_name("../Desktop"), None);
        assert_eq!(sanitize_name(".env"), None);
        assert_eq!(sanitize_name("a/b"), None);
        assert_eq!(sanitize_name(""), None);
        assert_eq!(sanitize_name(&"x".repeat(80)).map(|s| s.len()), Some(60));
        let t = tempdir();
        for n in ["Makefile", "README", "LICENSE"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert!(
            plan.groups
                .iter()
                .all(|g| sanitize_name(&g.name).as_deref() == Some(g.name.as_str()))
        );
    }

    #[test]
    fn apply_naming_renames_merges_and_ignores_bad_input() {
        let t = tempdir();
        for n in [
            "Invoice-1.pdf",
            "Invoice-2.pdf",
            "Invoice-3.pdf",
            "Bill-1.pdf",
            "Bill-2.pdf",
            "Bill-3.pdf",
        ] {
            touch(&t.path().join(n));
        }
        let mut plan = propose(&scan_folder(t.path()).expect("scan"));
        let inv = group(&plan, "Invoice").id.clone();
        let bill = group(&plan, "Bill").id.clone();
        apply_naming(
            &mut plan,
            &[
                Naming {
                    id: inv.clone(),
                    name: "Tagihan".into(),
                    confidence: 0.9,
                },
                Naming {
                    id: "nope".into(),
                    name: "Ghost".into(),
                    confidence: 0.9,
                },
                Naming {
                    id: bill.clone(),
                    name: "../x".into(),
                    confidence: 0.9,
                },
            ],
            &[(bill.clone(), inv.clone()), ("nope".into(), inv.clone())],
        );
        let g = group(&plan, "Tagihan");
        assert_eq!(g.entries.len(), 6, "Bill merged into Invoice/Tagihan");
        assert_eq!(g.source, GroupSource::Ai);
        assert!(
            plan.groups
                .iter()
                .all(|g| g.name != "Ghost" && g.name != "../x")
        );
    }

    #[test]
    fn low_confidence_merge_into_existing_folder_is_ignored() {
        let t = tempdir();
        touch(&t.path().join("Keuangan/old.xlsx"));
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "Invoice-3.pdf"] {
            touch(&t.path().join(n));
        }
        let mut plan = propose(&scan_folder(t.path()).expect("scan"));
        let inv = group(&plan, "Invoice").id.clone();
        let fin = group(&plan, "Keuangan").id.clone();
        apply_naming(
            &mut plan,
            &[Naming {
                id: inv.clone(),
                name: "Invoice".into(),
                confidence: 0.5,
            }],
            &[(inv.clone(), fin.clone())],
        );
        assert_eq!(
            group(&plan, "Invoice").entries.len(),
            3,
            "kept apart below 0.7"
        );
        apply_naming(
            &mut plan,
            &[Naming {
                id: inv.clone(),
                name: "Invoice".into(),
                confidence: 0.9,
            }],
            &[(inv, fin)],
        );
        assert_eq!(group(&plan, "Keuangan").entries.len(), 4);
    }

    #[test]
    fn existing_and_unit_groups_are_never_renamed_or_merged_away() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/old.pdf"));
        touch(&t.path().join("backend/package.json"));
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "Invoice-3.pdf"] {
            touch(&t.path().join(n));
        }
        let mut plan = propose(&scan_folder(t.path()).expect("scan"));
        let docs = group(&plan, "Dokumen").id.clone();
        let proj = group(&plan, "Proyek").id.clone();
        let inv = group(&plan, "Invoice").id.clone();
        apply_naming(
            &mut plan,
            &[
                Naming {
                    id: docs.clone(),
                    name: "Documents".into(),
                    confidence: 0.99,
                },
                Naming {
                    id: proj.clone(),
                    name: "Code".into(),
                    confidence: 0.99,
                },
            ],
            &[(docs.clone(), inv.clone()), (proj.clone(), inv.clone())],
        );
        assert_eq!(
            group(&plan, "Dokumen").entries.len(),
            1,
            "existing folder keeps its name and is not merged away"
        );
        assert_eq!(
            group(&plan, "Proyek").entries.len(),
            1,
            "projects keep their unit group"
        );
        assert_eq!(group(&plan, "Invoice").entries.len(), 3);
    }

    #[test]
    fn loose_files_named_after_an_existing_folder_go_into_it() {
        let t = tempdir();
        touch(&t.path().join("Saham/analis.xlsx"));
        touch(&t.path().join("accel bytes/PRD.docx"));
        touch(&t.path().join("Print/a.pdf"));
        for n in ["My Saham.xlsx", "AccelBytes.docx", "notes.txt"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let names = |g: &str| {
            group(&plan, g)
                .entries
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        };
        assert!(names("Saham").contains(&"My Saham.xlsx".to_string()));
        assert!(names("accel bytes").contains(&"AccelBytes.docx".to_string()));
        assert!(!names("Print").contains(&"notes.txt".to_string()));
    }

    #[test]
    fn only_whole_words_match_a_folder_and_such_folders_need_checking() {
        let t = tempdir();
        touch(&t.path().join("Print/a.pdf"));
        touch(&t.path().join("Pajak/spt.pdf"));
        for n in ["blueprint house.pdf", "Bukti Pajak 2024.pdf"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let print = group(&plan, "Print");
        assert!(
            print
                .entries
                .iter()
                .all(|e| e.name != "blueprint house.pdf"),
            "a word that merely contains the folder name is not a match"
        );
        let pajak = group(&plan, "Pajak");
        assert!(
            pajak
                .entries
                .iter()
                .any(|e| e.name == "Bukti Pajak 2024.pdf")
        );
        assert_eq!(
            pajak.confidence,
            Confidence::Check,
            "files guessed into an existing folder are never pre-ticked"
        );
    }
}
