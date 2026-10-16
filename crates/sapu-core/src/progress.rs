//! Progress events emitted by long-running operations.
//!
//! Core only reports raw counters; estimating time-to-completion and
//! formatting human-readable strings is the UI layer's job (PRD §10.1).

use std::time::Instant;

/// A snapshot of how far an operation has progressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    /// Items processed so far (files, groups, ...).
    pub items_done: u64,
    /// Total items when known in advance; `None` means indeterminate.
    pub items_total: Option<u64>,
    /// Bytes processed so far.
    pub bytes_done: u64,
    /// Total bytes when known in advance; `None` means indeterminate.
    pub bytes_total: Option<u64>,
}

/// Receiver for progress events. Implemented by CLI/GUI adapters.
pub trait ProgressSink {
    fn report(&mut self, event: &ProgressEvent);
}

/// A sink that drops every event.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn report(&mut self, _event: &ProgressEvent) {}
}

impl<S: ProgressSink + ?Sized> ProgressSink for &mut S {
    fn report(&mut self, event: &ProgressEvent) {
        (**self).report(event)
    }
}

/// Adapts a closure into a [`ProgressSink`].
pub struct FnSink<F>(pub F);

impl<F: FnMut(&ProgressEvent)> ProgressSink for FnSink<F> {
    fn report(&mut self, event: &ProgressEvent) {
        (self.0)(event)
    }
}

/// Wraps a sink and limits how often events reach it (max `hz` per second).
/// The final event (`items_done == items_total`) is always delivered.
pub struct Throttled<S> {
    inner: S,
    min_interval: std::time::Duration,
    last: Option<Instant>,
}

impl<S: ProgressSink> Throttled<S> {
    pub fn new(inner: S, hz: u32) -> Self {
        let hz = hz.max(1);
        Self {
            inner,
            min_interval: std::time::Duration::from_secs(1) / hz,
            last: None,
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: ProgressSink> ProgressSink for Throttled<S> {
    fn report(&mut self, event: &ProgressEvent) {
        let is_final = event.items_total.is_some_and(|t| event.items_done >= t);
        let due = self
            .last
            .is_none_or(|last| last.elapsed() >= self.min_interval);
        if is_final || due {
            self.last = Some(Instant::now());
            self.inner.report(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(done: u64, total: Option<u64>) -> ProgressEvent {
        ProgressEvent {
            items_done: done,
            items_total: total,
            bytes_done: 0,
            bytes_total: None,
        }
    }

    #[test]
    fn throttled_delivers_first_and_final_event() {
        let mut seen = Vec::new();
        {
            let mut t = Throttled::new(FnSink(|e: &ProgressEvent| seen.push(e.items_done)), 1);
            for i in 0..=10 {
                t.report(&ev(i, Some(10)));
            }
        }
        assert_eq!(seen.first(), Some(&0));
        assert_eq!(seen.last(), Some(&10));
        assert!(seen.len() < 11, "intermediate events should be throttled");
    }

    #[test]
    fn indeterminate_total_is_never_treated_as_final() {
        let mut count = 0;
        {
            let mut t = Throttled::new(FnSink(|_: &ProgressEvent| count += 1), 1);
            t.report(&ev(5, None));
            t.report(&ev(6, None));
        }
        assert_eq!(count, 1);
    }
}
