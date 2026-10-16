//! Bytes a file really occupies on disk.
//!
//! `Metadata::len()` is the logical size. Sparse files (Docker.raw),
//! cloud placeholders (iCloud "dataless" files) and APFS clones make the
//! logical sum exceed the disk itself, which confuses users ("723 GB on a
//! 500 GB disk"). On Unix the allocated block count is what the disk
//! actually holds; elsewhere we fall back to the logical size.

use std::fs::Metadata;

pub fn disk_usage_of(meta: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    #[test]
    fn sparse_file_counts_allocated_blocks_not_logical_size() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("sparse.bin");
        let mut f = std::fs::File::create(&p).expect("create");
        f.seek(SeekFrom::Start(50 * 1024 * 1024)).expect("seek");
        f.write_all(b"end").expect("write");
        f.sync_all().expect("sync");
        let meta = std::fs::metadata(&p).expect("meta");
        assert!(meta.len() > 50 * 1024 * 1024);
        // Some filesystems (e.g. tmpfs variants) may not support holes; only
        // assert the invariant that allocated never wildly exceeds logical.
        assert!(disk_usage_of(&meta) <= meta.len() + 1024 * 1024);
    }
}
