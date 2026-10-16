//! Disk capacity facts for the first-scan progress denominator (PRD §10.1).

use std::path::Path;

/// `(total, available)` bytes of the volume that contains `path`, or
/// `None` when the OS does not answer (network drives, missing path).
pub fn capacity(path: &Path) -> Option<(u64, u64)> {
    let mut p = path;
    loop {
        if let (Ok(total), Ok(avail)) = (fs4::total_space(p), fs4::available_space(p)) {
            return Some((total, avail));
        }
        p = p.parent()?;
    }
}

/// Bytes in use on the volume that contains `path`.
pub fn used_bytes(path: &Path) -> Option<u64> {
    capacity(path).map(|(total, avail)| total.saturating_sub(avail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_for_tempdir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(used_bytes(tmp.path()).is_some_and(|b| b > 0));
    }
}
