//! Shared run / pause / cancel switch between the scan thread and the UI.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanState {
    Running,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub struct ScanControl(Arc<AtomicU8>);

impl ScanControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> ScanState {
        match self.0.load(Ordering::Acquire) {
            1 => ScanState::Paused,
            2 => ScanState::Cancelled,
            _ => ScanState::Running,
        }
    }

    pub fn pause(&self) {
        let _ = self
            .0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
    }

    pub fn resume(&self) {
        let _ = self
            .0
            .compare_exchange(1, 0, Ordering::AcqRel, Ordering::Acquire);
    }

    /// Cancellation is final.
    pub fn cancel(&self) {
        self.0.store(2, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.state() == ScanState::Cancelled
    }

    /// Blocks while paused; returns `false` when cancelled.
    pub(crate) fn wait_if_paused(&self) -> bool {
        loop {
            match self.state() {
                ScanState::Running => return true,
                ScanState::Cancelled => return false,
                ScanState::Paused => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions() {
        let c = ScanControl::new();
        assert_eq!(c.state(), ScanState::Running);
        c.pause();
        assert_eq!(c.state(), ScanState::Paused);
        c.resume();
        assert_eq!(c.state(), ScanState::Running);
        c.cancel();
        c.resume();
        assert_eq!(c.state(), ScanState::Cancelled, "cancel is final");
        assert!(!c.wait_if_paused());
    }
}
