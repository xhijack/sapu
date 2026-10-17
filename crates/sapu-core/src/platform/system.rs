//! Live system facts for the home dashboard: disks, CPU and memory.

use std::path::PathBuf;

use sysinfo::{Disks, System};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: PathBuf,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub removable: bool,
    pub file_system: String,
    /// Headroom this disk should keep (see `care::recommended_free_bytes`).
    pub recommended_free_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemSnapshot {
    /// 0..100 across all cores; `None` until two samples exist.
    pub cpu_percent: Option<f32>,
    /// Per-core usage 0..100, same order as the OS reports; empty until two samples exist.
    pub cpu_per_core: Vec<f32>,
    pub cpu_count: usize,
    pub mem_total_bytes: u64,
    pub mem_used_bytes: u64,
    pub swap_used_bytes: u64,
    pub disks: Vec<DiskInfo>,
    pub os_name: String,
}

/// Keeps a `sysinfo::System` so CPU usage can be derived between calls.
pub struct SystemMonitor {
    sys: System,
    samples: u32,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self { sys, samples: 1 }
    }

    pub fn snapshot(&mut self) -> SystemSnapshot {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.samples = self.samples.saturating_add(1);
        let disks = Disks::new_with_refreshed_list();
        let mut seen = std::collections::HashSet::new();
        let disks = disks
            .list()
            .iter()
            .filter(|d| d.total_space() > 0)
            .filter(|d| seen.insert(d.mount_point().to_path_buf()))
            .filter(|d| !is_system_internal(d.mount_point()))
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_path_buf(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
                removable: d.is_removable(),
                file_system: d.file_system().to_string_lossy().into_owned(),
                recommended_free_bytes: crate::care::recommended_free_bytes(d.total_space()),
            })
            .collect();
        SystemSnapshot {
            cpu_percent: (self.samples >= 2).then(|| self.sys.global_cpu_usage()),
            cpu_per_core: if self.samples >= 2 {
                self.sys.cpus().iter().map(|c| c.cpu_usage()).collect()
            } else {
                Vec::new()
            },
            cpu_count: self.sys.cpus().len(),
            mem_total_bytes: self.sys.total_memory(),
            mem_used_bytes: self.sys.used_memory(),
            swap_used_bytes: self.sys.used_swap(),
            disks,
            os_name: System::long_os_version().unwrap_or_default(),
        }
    }
}

/// macOS exposes many internal APFS volumes; show the user-facing ones.
fn is_system_internal(mount: &std::path::Path) -> bool {
    let s = mount.to_string_lossy();
    s.starts_with("/System/Volumes/") && !s.starts_with("/System/Volumes/Data")
        || s.starts_with("/private/var/vm")
        || s.starts_with("/dev")
        || s.starts_with("/boot/efi")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_has_memory_and_a_disk() {
        let mut m = SystemMonitor::new();
        let s = m.snapshot();
        assert!(s.mem_total_bytes > 0);
        assert!(!s.disks.is_empty());
        assert!(s.cpu_count > 0);
        let s2 = m.snapshot();
        assert!(s2.cpu_percent.is_some());
        assert_eq!(s2.cpu_per_core.len(), s2.cpu_count);
    }
}
