//! OS-specific facts, isolated behind `cfg(target_os)` (CLAUDE.md rule 5).

mod detect;
mod permissions;
mod size;
mod system;

pub use detect::{DevTool, detect_dev_tools, detect_dev_tools_in};
pub use permissions::{DiskAccess, full_disk_access};
pub use size::disk_usage_of;
pub use system::{DiskInfo, SystemMonitor, SystemSnapshot};

/// Parallelism for every `jwalk` walk in Sapu. jwalk's default gives up
/// after one second when the shared rayon pool is busy and yields an
/// error entry, which silently produced empty results (0 B app bundles)
/// whenever two walks overlapped. Waiting is always better than lying.
pub fn walk_parallelism() -> jwalk::Parallelism {
    jwalk::Parallelism::RayonDefaultPool {
        busy_timeout: std::time::Duration::from_secs(600),
    }
}
