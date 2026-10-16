//! OS-specific facts, isolated behind `cfg(target_os)` (CLAUDE.md rule 5).

mod detect;
mod permissions;
mod size;
mod system;

pub use detect::{DevTool, detect_dev_tools, detect_dev_tools_in};
pub use permissions::{DiskAccess, full_disk_access};
pub use size::disk_usage_of;
pub use system::{DiskInfo, SystemMonitor, SystemSnapshot};
