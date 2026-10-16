//! "Rawat" (Smart Care): one score and a short list of next steps,
//! computed deterministically from a scan and a clean evaluation.
//!
//! The score is only about space and clutter. It says nothing about
//! performance or security (PRD non-goals).

use crate::cleaner::CleanReport;
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
    if free_pct < 10 {
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
    use std::path::PathBuf;

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
}
