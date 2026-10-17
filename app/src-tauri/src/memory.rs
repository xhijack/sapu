//! "Bebaskan memori": pressure reading, the heaviest apps, and the two
//! safe relief actions. Thin calls into `sapu_core::platform::memory*`.

use sapu_core::platform::memory::{MemorySnapshot, snapshot};
use sapu_core::platform::memory_relief::{
    PurgeOutcome, QuitOutcome, purge_file_cache, quit_gracefully,
};

use crate::{CmdResult, CommandError};

const TOP_PROCESSES: usize = 15;

/// Pressure plus the 15 heaviest apps / processes.
#[tauri::command]
pub async fn memory_snapshot() -> CmdResult<MemorySnapshot> {
    tauri::async_runtime::spawn_blocking(|| snapshot(TOP_PROCESSES))
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

/// Asks the picked apps to quit gracefully; one outcome per pid.
#[tauri::command]
pub async fn memory_quit_apps(pids: Vec<u32>) -> CmdResult<Vec<QuitOutcome>> {
    if pids.is_empty() {
        return Err(CommandError::new("validation", "nothing selected"));
    }
    tauri::async_runtime::spawn_blocking(move || quit_gracefully(&pids))
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}

/// macOS: Apple's `purge` behind the administrator prompt. Elsewhere
/// `unsupported`.
#[tauri::command]
pub async fn memory_purge() -> CmdResult<PurgeOutcome> {
    if !cfg!(target_os = "macos") {
        return Err(CommandError::new("unsupported", "purge is macOS only"));
    }
    tauri::async_runtime::spawn_blocking(purge_file_cache)
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))
}
