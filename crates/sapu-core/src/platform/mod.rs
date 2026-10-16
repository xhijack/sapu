//! OS-specific facts, isolated behind `cfg(target_os)` (CLAUDE.md rule 5).

mod detect;
mod permissions;

pub use detect::{DevTool, detect_dev_tools, detect_dev_tools_in};
pub use permissions::{DiskAccess, full_disk_access};
