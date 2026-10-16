//! Honest time-to-completion estimates (PRD §10.1).
//!
//! The estimator tracks a *moving* rate over a sliding window so it adapts
//! when throughput changes (SSD → folder of tiny files), smooths the
//! resulting ETA so it never jumps around, and refuses to give a number
//! until it has enough data. It returns structured values; the UI turns
//! them into "about 2 minutes left" in the user's language.

use std::time::Duration;

use crate::progress::ProgressEvent;

/// What the UI should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Eta {
    /// No total known: show an indeterminate indicator and elapsed time.
    Indeterminate,
    /// Not enough samples yet ("calculating...").
    Computing,
    /// Uncertainty is high: show a range, e.g. 2–4 minutes.
    Range { low: Duration, high: Duration },
    /// Confident enough for a single rounded figure.
    About(Duration),
    /// The last few seconds.
    AlmostDone,
}

/// Rounds a duration to a human bucket so the UI never shows false
/// precision (no "1:47").
pub fn round_human(d: Duration) -> Duration {
    let s = d.as_secs();
    let rounded = match s {
        0..=10 => 10,
        11..=30 => 30,
        31..=90 => 60,
        91..=600 => s.div_ceil(60) * 60,
        601..=3600 => s.div_ceil(300) * 300,
        _ => s.div_ceil(900) * 900,
    };
    Duration::from_secs(rounded)
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    at: Duration,
    units: u64,
}

/// Rate estimator driven by [`ProgressEvent`]s and a monotonic clock
/// supplied by the caller (`elapsed` since the operation started).
#[derive(Debug, Clone)]
pub struct Estimator {
    window: Duration,
    min_elapsed: Duration,
    min_samples: usize,
    samples: Vec<Sample>,
    smoothed: Option<f64>,
    last_fraction: f64,
    last_eta: Option<Duration>,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new(Duration::from_secs(10), Duration::from_secs(3), 3)
    }
}

impl Estimator {
    /// `window`: how far back the moving rate looks; `min_elapsed` and
    /// `min_samples`: how much evidence is required before a number is shown.
    pub fn new(window: Duration, min_elapsed: Duration, min_samples: usize) -> Self {
        Self {
            window,
            min_elapsed,
            min_samples,
            samples: Vec::new(),
            smoothed: None,
            last_fraction: 0.0,
            last_eta: None,
        }
    }

    /// Feed a progress event observed at `elapsed` since start.
    pub fn observe(&mut self, event: &ProgressEvent, elapsed: Duration) {
        let units = if event.bytes_total.is_some() {
            event.bytes_done
        } else {
            event.items_done
        };
        self.samples.push(Sample { at: elapsed, units });
        let cutoff = elapsed.saturating_sub(self.window);
        // Keep one sample before the cutoff so the window spans `window` seconds.
        while self.samples.len() > 2 && self.samples[1].at <= cutoff {
            self.samples.remove(0);
        }
    }

    /// Completed fraction in `0.0..=1.0`, monotonic non-decreasing, never
    /// reaching 1.0 until done.
    pub fn fraction(&mut self, event: &ProgressEvent) -> f64 {
        let f = match (event.bytes_total, event.items_total) {
            (Some(total), _) if total > 0 => event.bytes_done as f64 / total as f64,
            (_, Some(total)) if total > 0 => event.items_done as f64 / total as f64,
            _ => return self.last_fraction,
        };
        let done = event.items_total.is_some_and(|t| event.items_done >= t);
        let f = if done { 1.0 } else { f.clamp(0.0, 0.999) };
        self.last_fraction = self.last_fraction.max(f);
        self.last_fraction
    }

    /// Current estimate for `event` at `elapsed`.
    pub fn eta(&mut self, event: &ProgressEvent, elapsed: Duration) -> Eta {
        let (done, total) = match (event.bytes_total, event.items_total) {
            (Some(t), _) if t > 0 => (event.bytes_done, t),
            (_, Some(t)) if t > 0 => (event.items_done, t),
            _ => return Eta::Indeterminate,
        };
        if done >= total {
            return Eta::AlmostDone;
        }
        if elapsed < self.min_elapsed || self.samples.len() < self.min_samples {
            return Eta::Computing;
        }
        let Some(rate) = self.moving_rate() else {
            return Eta::Computing;
        };
        let remaining = (total - done) as f64 / rate;
        // Exponential smoothing so the figure does not flicker.
        let smoothed = match self.smoothed {
            Some(prev) => prev * 0.7 + remaining * 0.3,
            None => remaining,
        };
        self.smoothed = Some(smoothed);
        let secs = Duration::from_secs_f64(smoothed.max(0.0));
        if secs <= Duration::from_secs(5) {
            return Eta::AlmostDone;
        }
        // Early in the operation the rate is unreliable: report a range.
        let fraction = done as f64 / total as f64;
        let eta = if fraction < 0.1 || elapsed < self.min_elapsed * 3 {
            Eta::Range {
                low: round_human(secs.mul_f64(0.7)),
                high: round_human(secs.mul_f64(1.5)),
            }
        } else {
            Eta::About(round_human(secs))
        };
        if let Eta::About(d) = eta {
            // Never let a rounded figure climb by one bucket and fall back
            // next tick: only move when the change is at least a bucket.
            if let Some(prev) = self.last_eta
                && prev != d
                && prev.abs_diff(d) < bucket_step(prev)
            {
                return Eta::About(prev);
            }
            self.last_eta = Some(d);
        }
        eta
    }

    fn moving_rate(&self) -> Option<f64> {
        let first = self.samples.first()?;
        let last = self.samples.last()?;
        let dt = last.at.saturating_sub(first.at).as_secs_f64();
        let du = last.units.saturating_sub(first.units) as f64;
        if dt <= 0.0 || du <= 0.0 {
            return None;
        }
        Some(du / dt)
    }
}

fn bucket_step(d: Duration) -> Duration {
    match d.as_secs() {
        0..=90 => Duration::from_secs(20),
        91..=600 => Duration::from_secs(60),
        601..=3600 => Duration::from_secs(300),
        _ => Duration::from_secs(900),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(done: u64, total: u64) -> ProgressEvent {
        ProgressEvent {
            items_done: 0,
            items_total: None,
            bytes_done: done,
            bytes_total: Some(total),
        }
    }

    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn no_total_is_indeterminate() {
        let mut e = Estimator::default();
        let ev = ProgressEvent {
            items_done: 5,
            items_total: None,
            bytes_done: 0,
            bytes_total: None,
        };
        e.observe(&ev, s(10));
        assert_eq!(e.eta(&ev, s(10)), Eta::Indeterminate);
    }

    #[test]
    fn no_number_in_first_seconds() {
        let mut e = Estimator::default();
        for t in 0..3 {
            e.observe(&ev(t * 100, 10_000), s(t));
        }
        assert_eq!(e.eta(&ev(200, 10_000), s(2)), Eta::Computing);
    }

    #[test]
    fn steady_rate_gives_range_then_about() {
        let mut e = Estimator::default();
        // 100 units/s, total 10_000 → 100 s.
        for t in 0..=5 {
            e.observe(&ev(t * 100, 10_000), s(t));
        }
        match e.eta(&ev(500, 10_000), s(5)) {
            Eta::Range { low, high } => assert!(low < high),
            other => panic!("expected range early on, got {other:?}"),
        }
        for t in 6..=30 {
            e.observe(&ev(t * 100, 10_000), s(t));
        }
        match e.eta(&ev(3000, 10_000), s(30)) {
            Eta::About(d) => assert!(d >= s(60) && d <= s(120), "got {d:?}"),
            other => panic!("expected About, got {other:?}"),
        }
    }

    #[test]
    fn moving_rate_adapts_to_slowdown() {
        let mut e = Estimator::default();
        for t in 0..=20 {
            e.observe(&ev(t * 1000, 100_000), s(t)); // fast: 1000/s
        }
        let _ = e.eta(&ev(20_000, 100_000), s(20));
        for t in 21..=60 {
            e.observe(&ev(20_000 + (t - 20) * 10, 100_000), s(t)); // slow: 10/s
        }
        let eta = e.eta(&ev(20_400, 100_000), s(60));
        // Remaining 79_600 at 10/s ≈ 7960 s; a total-average rate would say ≈ 234 s.
        match eta {
            Eta::About(d) => assert!(
                d > s(1000),
                "moving rate should reflect slowdown, got {d:?}"
            ),
            other => panic!("expected About, got {other:?}"),
        }
    }

    #[test]
    fn fraction_is_monotonic_and_capped() {
        let mut e = Estimator::default();
        assert!(e.fraction(&ev(5000, 10_000)) > 0.49);
        assert!(e.fraction(&ev(4000, 10_000)) > 0.49, "never goes backwards");
        assert!(e.fraction(&ev(9999, 10_000)) < 1.0);
        let done = ProgressEvent {
            items_done: 3,
            items_total: Some(3),
            bytes_done: 10_000,
            bytes_total: Some(10_000),
        };
        assert_eq!(e.fraction(&done), 1.0);
    }

    /// Locks the IPC shape the frontend relies on.
    #[test]
    fn json_shape_is_stable() {
        let about = serde_json::to_value(Eta::About(s(120))).expect("json");
        assert_eq!(
            about,
            serde_json::json!({"kind": "about", "secs": 120, "nanos": 0})
        );
        let range = serde_json::to_value(Eta::Range {
            low: s(60),
            high: s(120),
        })
        .expect("json");
        assert_eq!(
            range,
            serde_json::json!({"kind": "range", "low": {"secs": 60, "nanos": 0}, "high": {"secs": 120, "nanos": 0}})
        );
        assert_eq!(
            serde_json::to_value(Eta::Computing).expect("json"),
            serde_json::json!({"kind": "computing"})
        );
    }

    #[test]
    fn rounding_has_no_false_precision() {
        assert_eq!(round_human(s(107)), s(120));
        assert_eq!(round_human(s(47)), s(60));
        assert_eq!(round_human(s(1300)), s(1500));
        assert_eq!(round_human(s(7)), s(10));
    }
}

/// Which long operation a rate belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    /// bytes per second
    Scan,
    /// items per second
    Clean,
    /// bytes per second (stage 3 hashing dominates)
    Dedupe,
}

/// Observed throughput on this device, so a preview can say "about a
/// minute" before anything starts (PRD §10.1 P1). Keeps the last five
/// rates per operation and estimates from their median.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RateHistory {
    rates: std::collections::HashMap<OpKind, Vec<f64>>,
}

impl RateHistory {
    const KEEP: usize = 5;

    pub fn default_path() -> Option<std::path::PathBuf> {
        dirs::data_local_dir().map(|d| d.join("sapu").join("rates.json"))
    }

    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Record `amount` units processed in `elapsed`. Ignores tiny runs,
    /// which would only add noise.
    pub fn record(&mut self, kind: OpKind, amount: u64, elapsed: Duration) {
        if elapsed < Duration::from_secs(2) || amount == 0 {
            return;
        }
        let v = self.rates.entry(kind).or_default();
        v.push(amount as f64 / elapsed.as_secs_f64());
        if v.len() > Self::KEEP {
            v.remove(0);
        }
    }

    /// Rounded estimate for processing `amount` units, or `None` without history.
    pub fn estimate(&self, kind: OpKind, amount: u64) -> Option<Duration> {
        let v = self.rates.get(&kind)?;
        if v.is_empty() {
            return None;
        }
        let mut sorted = v.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        if median <= 0.0 {
            return None;
        }
        Some(round_human(Duration::from_secs_f64(amount as f64 / median)))
    }
}

#[cfg(test)]
mod rate_tests {
    use super::*;

    #[test]
    fn estimates_from_median_and_rounds() {
        let mut h = RateHistory::default();
        assert_eq!(h.estimate(OpKind::Scan, 1000), None);
        h.record(OpKind::Scan, 1_000_000, Duration::from_secs(10)); // 100 KB/s
        h.record(OpKind::Scan, 3_000_000, Duration::from_secs(10)); // 300 KB/s
        h.record(OpKind::Scan, 2_000_000, Duration::from_secs(10)); // 200 KB/s (median)
        assert_eq!(
            h.estimate(OpKind::Scan, 20_000_000),
            Some(Duration::from_secs(120))
        );
        h.record(OpKind::Scan, 5, Duration::from_millis(100)); // ignored
        assert_eq!(h.rates[&OpKind::Scan].len(), 3);
    }

    #[test]
    fn roundtrip_and_cap() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("x/rates.json");
        let mut h = RateHistory::default();
        for i in 1..=7 {
            h.record(OpKind::Clean, i * 100, Duration::from_secs(10));
        }
        assert_eq!(h.rates[&OpKind::Clean].len(), 5);
        h.save(&p).expect("save");
        assert_eq!(RateHistory::load(&p), h);
    }
}
