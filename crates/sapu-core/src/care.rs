//! "Rawat" (Smart Care): one score and a short list of next steps,
//! computed deterministically from a scan and a clean evaluation.
//!
//! The score is only about space and clutter. It says nothing about
//! performance or security (PRD non-goals).

use std::path::{Path, PathBuf};

use crate::cleaner::CleanReport;
use crate::error::{Result, SapuError};
use crate::rules::Risk;
use crate::scanner::ScanResult;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScoreComponent {
    /// Stable key, translated by the UI: free_space, safe_clutter, review_clutter, stale_projects.
    pub key: String,
    pub points: u8,
    pub max: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Recommendation {
    /// Run the green rules; `bytes` is what they free.
    CleanSafe { bytes: u64 },
    /// Review yellow findings.
    ReviewClutter { bytes: u64 },
    /// Stale development projects worth cleaning.
    StaleProjects { count: usize, bytes: u64 },
    /// The biggest folder in the scan, for the user to look at.
    LargestFolder {
        path: std::path::PathBuf,
        bytes: u64,
    },
    /// Disk is nearly full; free space is the priority.
    DiskNearlyFull { free_pct: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpaceScore {
    pub score: u8,
    pub components: Vec<ScoreComponent>,
    pub free_pct: u8,
    pub safe_bytes: u64,
    pub review_bytes: u64,
    pub stale_project_bytes: u64,
    pub stale_projects: usize,
    pub recommendations: Vec<Recommendation>,
}

fn linear(value: f64, zero_at: f64, full_at: f64, max: u8) -> u8 {
    // Points fall from `max` at `full_at` (good) to 0 at `zero_at` (bad).
    let t = if full_at > zero_at {
        (value - zero_at) / (full_at - zero_at)
    } else {
        (zero_at - value) / (zero_at - full_at)
    };
    (t.clamp(0.0, 1.0) * f64::from(max)).round() as u8
}

/// Score a scan plus the clean evaluation that followed it.
/// One finished Care run, kept so the Care screen can show how the disk
/// has been doing over time (`care/history.jsonl`, newest last).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CareLogEntry {
    pub finished_at_unix: u64,
    pub score: u8,
    pub free_pct: u8,
    pub safe_bytes: u64,
}

/// `<local data>/sapu/care`.
pub fn care_log_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir().ok_or_else(|| {
        SapuError::io(
            "locate local data dir",
            std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"),
        )
    })?;
    Ok(base.join("sapu").join("care"))
}

/// Appends one run; keeps the last 200.
pub fn append_care_log(dir: &Path, entry: &CareLogEntry) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| SapuError::io("create care dir", e))?;
    let path = dir.join("history.jsonl");
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default();
    lines.push(serde_json::to_string(entry)?);
    if lines.len() > 200 {
        let drop = lines.len() - 200;
        lines.drain(..drop);
    }
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| SapuError::io("write care log", e))
}

/// Newest first.
pub fn read_care_log(dir: &Path, limit: usize) -> Vec<CareLogEntry> {
    let Ok(text) = std::fs::read_to_string(dir.join("history.jsonl")) else {
        return Vec::new();
    };
    let mut v: Vec<CareLogEntry> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    v.reverse();
    v.truncate(limit);
    v
}

/// How much of a disk should stay free: 10 % of its size or 20 GB,
/// whichever is larger, but never more than a quarter of a small disk.
/// APFS snapshots, swap and OS updates need the headroom; below it macOS
/// and Windows slow down and updates fail. The one threshold every
/// surface (status bar, Care) uses.
pub fn recommended_free_bytes(disk_total: u64) -> u64 {
    const FLOOR: u64 = 20 * 1024 * 1024 * 1024;
    (disk_total / 10).max(FLOOR.min(disk_total / 4))
}

/// How much room a disk has left, the one scale the status bar, the
/// tray and notifications agree on. `Low` below 20 % free, `Critical`
/// below 5 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceLevel {
    Ok,
    Low,
    Critical,
}

/// Level for `free` bytes left on a disk of `total` bytes. An empty or
/// unknown disk (`total == 0`) is `Ok`: nothing to warn about.
pub fn space_level(free: u64, total: u64) -> SpaceLevel {
    if total == 0 {
        return SpaceLevel::Ok;
    }
    let free = u128::from(free);
    let total = u128::from(total);
    if free * 100 < total * 5 {
        SpaceLevel::Critical
    } else if free * 100 < total * 20 {
        SpaceLevel::Low
    } else {
        SpaceLevel::Ok
    }
}

pub fn score(scan: &ScanResult, clean: &CleanReport) -> SpaceScore {
    let disk_total = scan.disk_total_bytes.unwrap_or(scan.total_bytes).max(1);
    let free = scan.disk_free_bytes.unwrap_or(0);
    let free_frac = free as f64 / disk_total as f64;
    let free_pct = (free_frac * 100.0).round() as u8;

    let safe_bytes: u64 = clean
        .findings
        .iter()
        .filter(|f| f.risk == Risk::Green)
        .map(|f| f.bytes)
        .sum::<u64>()
        + clean
            .commands
            .iter()
            .filter(|c| c.risk == Risk::Green && c.available)
            .filter_map(|c| c.estimate_bytes)
            .sum::<u64>()
        + clean
            .projects
            .iter()
            .flat_map(|p| p.artefacts.iter())
            .filter(|a| a.risk == Risk::Green)
            .map(|a| a.bytes)
            .sum::<u64>();
    let review_bytes: u64 = clean
        .findings
        .iter()
        .filter(|f| f.risk == Risk::Yellow)
        .map(|f| f.bytes)
        .sum::<u64>()
        + clean
            .commands
            .iter()
            .filter(|c| c.risk == Risk::Yellow && c.available)
            .filter_map(|c| c.estimate_bytes)
            .sum::<u64>();
    let stale: Vec<_> = clean
        .projects
        .iter()
        .filter(|p| p.idle_days.is_some_and(|d| d >= 30))
        .collect();
    let stale_project_bytes: u64 = stale.iter().map(|p| p.bytes).sum();

    let components = vec![
        // 40 pts: ≥ 25 % free is full marks, 0 % free is none.
        ScoreComponent {
            key: "free_space".into(),
            points: linear(free_frac, 0.0, 0.25, 40),
            max: 40,
        },
        // 30 pts: safe clutter ≥ 5 % of the disk costs everything.
        ScoreComponent {
            key: "safe_clutter".into(),
            points: linear(safe_bytes as f64 / disk_total as f64, 0.05, 0.0, 30),
            max: 30,
        },
        // 15 pts: review clutter ≥ 10 % of the disk costs everything.
        ScoreComponent {
            key: "review_clutter".into(),
            points: linear(review_bytes as f64 / disk_total as f64, 0.10, 0.0, 15),
            max: 15,
        },
        // 15 pts: stale project artefacts ≥ 3 % of the disk cost everything.
        ScoreComponent {
            key: "stale_projects".into(),
            points: linear(
                stale_project_bytes as f64 / disk_total as f64,
                0.03,
                0.0,
                15,
            ),
            max: 15,
        },
    ];
    let total: u32 = components.iter().map(|c| u32::from(c.points)).sum();

    let mut recommendations = Vec::new();
    if free < recommended_free_bytes(disk_total) {
        recommendations.push(Recommendation::DiskNearlyFull { free_pct });
    }
    if safe_bytes > 0 {
        recommendations.push(Recommendation::CleanSafe { bytes: safe_bytes });
    }
    if !stale.is_empty() {
        recommendations.push(Recommendation::StaleProjects {
            count: stale.len(),
            bytes: stale_project_bytes,
        });
    }
    if review_bytes > 0 {
        recommendations.push(Recommendation::ReviewClutter {
            bytes: review_bytes,
        });
    }
    if let Some(d) = scan.top_dirs(1).into_iter().next() {
        recommendations.push(Recommendation::LargestFolder {
            path: d.path,
            bytes: d.bytes,
        });
    }
    recommendations.truncate(4);

    SpaceScore {
        score: total.min(100) as u8,
        components,
        free_pct,
        safe_bytes,
        review_bytes,
        stale_project_bytes,
        stale_projects: stale.len(),
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::{CleanReport, Finding};
    use crate::rules::Profile;

    use std::collections::HashMap;

    fn scan(total: u64, disk_total: u64, free: u64) -> ScanResult {
        let mut dirs = HashMap::new();
        dirs.insert(
            PathBuf::from("/r"),
            crate::scanner::DirStats {
                own_bytes: 0,
                own_files: 0,
                total_bytes: total,
                total_files: 1,
            },
        );
        dirs.insert(
            PathBuf::from("/r/big"),
            crate::scanner::DirStats {
                own_bytes: total,
                own_files: 1,
                total_bytes: total,
                total_files: 1,
            },
        );
        ScanResult {
            root: PathBuf::from("/r"),
            diff: None,
            disk_total_bytes: Some(disk_total),
            disk_free_bytes: Some(free),
            started_at_unix: 0,
            finished_at_unix: 0,
            cancelled: false,
            total_bytes: total,
            file_count: 1,
            dir_count: 2,
            denied_count: 0,
            denied_sample: Vec::new(),
            files: Vec::new(),
            dirs,
        }
    }

    fn finding(risk: Risk, bytes: u64) -> Finding {
        Finding {
            rule_id: "x".into(),
            risk,
            profile: Profile::General,
            group: "g".into(),
            permanent: false,
            scope: PathBuf::from("/r"),
            items: Vec::new(),
            bytes,
            skipped_protected: 0,
        }
    }

    #[test]
    fn clean_half_empty_disk_scores_high() {
        let s = score(&scan(200, 1000, 500), &CleanReport::default());
        assert_eq!(s.score, 100);
        assert_eq!(s.free_pct, 50);
        assert!(matches!(
            s.recommendations[0],
            Recommendation::LargestFolder { .. }
        ));
    }

    #[test]
    fn full_disk_with_clutter_scores_low_and_recommends_cleaning_first() {
        let mut report = CleanReport::default();
        report.findings.push(finding(Risk::Green, 80)); // 8 % of disk → safe_clutter 0 pts
        report.findings.push(finding(Risk::Yellow, 50)); // 5 % → half of 15
        let s = score(&scan(960, 1000, 40), &report); // 4 % free → 6 of 40 pts
        assert!(s.score < 40, "{s:?}");
        assert_eq!(s.safe_bytes, 80);
        assert!(matches!(
            s.recommendations[0],
            Recommendation::DiskNearlyFull { free_pct: 4 }
        ));
        assert!(matches!(
            s.recommendations[1],
            Recommendation::CleanSafe { bytes: 80 }
        ));
        assert_eq!(s.components.iter().map(|c| c.max as u32).sum::<u32>(), 100);
    }

    #[test]
    fn recommended_free_is_ten_percent_with_a_floor() {
        const GB: u64 = 1024 * 1024 * 1024;
        assert_eq!(recommended_free_bytes(494 * GB), 494 * GB / 10);
        assert_eq!(recommended_free_bytes(120 * GB), 20 * GB);
        assert_eq!(recommended_free_bytes(40 * GB), 10 * GB);
        assert_eq!(recommended_free_bytes(0), 0);
    }

    #[test]
    fn space_level_thresholds() {
        assert_eq!(space_level(500, 1000), SpaceLevel::Ok);
        assert_eq!(space_level(200, 1000), SpaceLevel::Ok); // exactly 20 % is still ok
        assert_eq!(space_level(199, 1000), SpaceLevel::Low);
        assert_eq!(space_level(50, 1000), SpaceLevel::Low); // exactly 5 % is low
        assert_eq!(space_level(49, 1000), SpaceLevel::Critical);
        assert_eq!(space_level(0, 1000), SpaceLevel::Critical);
        assert_eq!(space_level(0, 0), SpaceLevel::Ok);
        // No overflow on multi-terabyte disks.
        let tb = 1024u64.pow(4);
        assert_eq!(space_level(3 * tb, 100 * tb), SpaceLevel::Critical);
        assert_eq!(space_level(30 * tb, 100 * tb), SpaceLevel::Ok);
    }

    #[test]
    fn care_log_round_trips_newest_first() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        let dir = tempfile::tempdir_in(&base).expect("tempdir");
        for (i, score) in [40u8, 70, 90].iter().enumerate() {
            append_care_log(
                dir.path(),
                &CareLogEntry {
                    finished_at_unix: 100 + i as u64,
                    score: *score,
                    free_pct: 10,
                    safe_bytes: 5,
                },
            )
            .expect("append");
        }
        let log = read_care_log(dir.path(), 2);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].score, 90);
        assert_eq!(log[1].score, 70);
    }
}
