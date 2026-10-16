//! Cross-feature helpers: throughput history and completion notifications.

use std::time::Duration;

use sapu_core::eta::{OpKind, RateHistory};
use sapu_core::settings::Settings;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::CmdResult;

/// Record a finished operation's throughput for future estimates.
pub fn record_rate(kind: OpKind, amount: u64, elapsed: Duration) {
    let Some(path) = RateHistory::default_path() else {
        return;
    };
    let mut h = RateHistory::load(&path);
    h.record(kind, amount, elapsed);
    if let Err(e) = h.save(&path) {
        tracing::warn!(error = %e, "could not save rate history");
    }
}

/// Rounded estimate in seconds for processing `amount` units, when the
/// device has history for this operation.
#[tauri::command]
pub fn estimate_duration(kind: OpKind, amount: u64) -> CmdResult<Option<u64>> {
    let Some(path) = RateHistory::default_path() else {
        return Ok(None);
    };
    Ok(RateHistory::load(&path)
        .estimate(kind, amount)
        .map(|d| d.as_secs()))
}

const NOTIFY_AFTER: Duration = Duration::from_secs(10);

/// System notification after a long operation, if enabled in settings.
/// The window may be in the background; the OS decides how to show it.
pub fn notify_done(app: &AppHandle, elapsed: Duration, title: &str, body: &str) {
    if elapsed < NOTIFY_AFTER {
        return;
    }
    let enabled = Settings::default_path()
        .and_then(|p| Settings::load(&p))
        .map(|s| s.notify_when_done)
        .unwrap_or(true);
    if !enabled {
        return;
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!(error = %e, "notification failed");
    }
}
