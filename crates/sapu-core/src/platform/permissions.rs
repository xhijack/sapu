//! Disk access permission probing.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskAccess {
    /// The OS has no such gate (Windows, Linux).
    NotApplicable,
    Granted,
    Denied,
    /// Could not determine (probe directory missing).
    Unknown,
}

/// macOS Full Disk Access status, detected without restarting the app by
/// probing a TCC-protected folder. Elsewhere returns `NotApplicable`.
pub fn full_disk_access() -> DiskAccess {
    imp::full_disk_access()
}

#[cfg(target_os = "macos")]
mod imp {
    use super::DiskAccess;

    pub(super) fn full_disk_access() -> DiskAccess {
        let Some(home) = dirs::home_dir() else {
            return DiskAccess::Unknown;
        };
        // Both folders are TCC-protected; reading either proves FDA.
        for probe in ["Library/Safari", "Library/Mail"] {
            let dir = home.join(probe);
            if !dir.exists() {
                continue;
            }
            return match std::fs::read_dir(&dir) {
                Ok(_) => DiskAccess::Granted,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => DiskAccess::Denied,
                Err(_) => DiskAccess::Unknown,
            };
        }
        DiskAccess::Unknown
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::DiskAccess;

    pub(super) fn full_disk_access() -> DiskAccess {
        DiskAccess::NotApplicable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_value_on_every_os() {
        let v = full_disk_access();
        if cfg!(target_os = "macos") {
            assert_ne!(v, DiskAccess::NotApplicable);
        } else {
            assert_eq!(v, DiskAccess::NotApplicable);
        }
    }
}
