//! Which CPU architectures an app bundle was built for, read from the
//! Mach-O header of its main executable. Nothing is executed.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSupport {
    /// Built for this machine's CPU.
    Native,
    /// Intel-only on Apple Silicon: runs through Rosetta 2, slower.
    Rosetta,
    /// 32-bit only (or PowerPC): cannot run on current macOS.
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Arches {
    pub arm64: bool,
    pub x86_64: bool,
    pub legacy32: bool,
}

const MH_MAGIC_64: u32 = 0xfeed_facf;
const MH_CIGAM_64: u32 = 0xcffa_edfe;
const MH_MAGIC: u32 = 0xfeed_face;
const MH_CIGAM: u32 = 0xcefa_edfe;
const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_CIGAM: u32 = 0xbeba_feca;
const CPU_X86_64: u32 = 0x0100_0007;
const CPU_ARM64: u32 = 0x0100_000c;

/// Parses the first bytes of a Mach-O (thin or fat) file.
pub fn arches_from_header(bytes: &[u8]) -> Option<Arches> {
    let be = |i: usize| -> Option<u32> {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let le = |i: usize| -> Option<u32> {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let magic = be(0)?;
    let mut a = Arches::default();
    let mut note = |cputype: u32| match cputype {
        CPU_ARM64 => a.arm64 = true,
        CPU_X86_64 => a.x86_64 = true,
        _ => a.legacy32 = true,
    };
    match magic {
        FAT_MAGIC => {
            let n = be(4)? as usize;
            for i in 0..n.min(16) {
                note(be(8 + i * 20)?);
            }
        }
        FAT_CIGAM => {
            let n = le(4)? as usize;
            for i in 0..n.min(16) {
                note(le(8 + i * 20)?);
            }
        }
        // Magic read big-endian equals MH_MAGIC when the file is big-endian
        // and MH_CIGAM when it is little-endian (every modern Mac binary).
        MH_MAGIC_64 | MH_MAGIC => note(be(4)?),
        MH_CIGAM_64 | MH_CIGAM => note(le(4)?),
        _ => return None,
    }
    Some(a)
}

/// Architectures of the bundle's main executable, if readable.
pub fn bundle_arches(bundle: &Path, executable: Option<&str>) -> Option<Arches> {
    let exe_name = executable
        .map(str::to_string)
        .or_else(|| bundle.file_stem().map(|s| s.to_string_lossy().into_owned()))?;
    let exe = bundle.join("Contents/MacOS").join(exe_name);
    let mut f = std::fs::File::open(exe).ok()?;
    let mut buf = [0u8; 8 + 16 * 20];
    use std::io::Read;
    let n = f.read(&mut buf).ok()?;
    arches_from_header(&buf[..n])
}

/// How well `arches` fit the machine running Sapu.
pub fn support_for(arches: Option<Arches>, host_arch: &str) -> AppSupport {
    let Some(a) = arches else {
        return AppSupport::Unknown;
    };
    match host_arch {
        "aarch64" => {
            if a.arm64 {
                AppSupport::Native
            } else if a.x86_64 {
                AppSupport::Rosetta
            } else if a.legacy32 {
                AppSupport::Unsupported
            } else {
                AppSupport::Unknown
            }
        }
        "x86_64" => {
            if a.x86_64 {
                AppSupport::Native
            } else if a.legacy32 {
                AppSupport::Unsupported
            } else if a.arm64 {
                // Apple Silicon only app on an Intel Mac.
                AppSupport::Unsupported
            } else {
                AppSupport::Unknown
            }
        }
        _ => AppSupport::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fat(cputypes: &[u32]) -> Vec<u8> {
        let mut v = FAT_MAGIC.to_be_bytes().to_vec();
        v.extend((cputypes.len() as u32).to_be_bytes());
        for c in cputypes {
            v.extend(c.to_be_bytes());
            v.extend([0u8; 16]);
        }
        v
    }

    #[test]
    fn reads_fat_and_thin_headers() {
        let a = arches_from_header(&fat(&[CPU_X86_64, CPU_ARM64])).expect("fat");
        assert!(a.arm64 && a.x86_64 && !a.legacy32);
        let mut thin = MH_MAGIC_64.to_le_bytes().to_vec();
        thin.extend(CPU_X86_64.to_le_bytes());
        let t = arches_from_header(&thin).expect("thin");
        assert!(t.x86_64 && !t.arm64);
        let mut old = MH_MAGIC.to_le_bytes().to_vec();
        old.extend(7u32.to_le_bytes()); // i386
        assert!(arches_from_header(&old).expect("32").legacy32);
        assert!(arches_from_header(b"#!/bin/sh").is_none());
    }

    #[test]
    fn support_depends_on_host() {
        let intel_only = Some(Arches {
            x86_64: true,
            ..Default::default()
        });
        assert_eq!(support_for(intel_only, "aarch64"), AppSupport::Rosetta);
        assert_eq!(support_for(intel_only, "x86_64"), AppSupport::Native);
        let old = Some(Arches {
            legacy32: true,
            ..Default::default()
        });
        assert_eq!(support_for(old, "aarch64"), AppSupport::Unsupported);
        assert_eq!(support_for(None, "aarch64"), AppSupport::Unknown);
    }
}
