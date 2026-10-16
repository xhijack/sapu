//! Disk capacity facts for the first-scan progress denominator (PRD §10.1).

use std::path::Path;

/// Bytes in use on the volume that contains `path`, or `None` when the
/// OS does not answer (network drives, missing path).
pub fn used_bytes(path: &Path) -> Option<u64> {
    let mut p = path;
    loop {
        if let (Ok(total), Ok(avail)) = (fs4::total_space(p), fs4::available_space(p)) {
            return Some(total.saturating_sub(avail));
        }
        p = p.parent()?;
    }
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
