//! linux-parity: complete
//! linux-source: vendor/linux/fs/filesystems.c
//! test-origin: linux:vendor/linux/fs/filesystems.c
//! Filesystem type listing and lookup helpers.
//!
//! Ref: `vendor/linux/fs/filesystems.c`

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::super_block::{FileSystemType, lookup_filesystem, registered_filesystems};

pub fn get_fs_type(name: &str) -> Option<FileSystemType> {
    lookup_filesystem(name)
}

pub fn filesystems() -> Vec<FileSystemType> {
    registered_filesystems()
}

pub fn filesystem_names() -> Vec<String> {
    filesystems()
        .into_iter()
        .map(|fs| String::from(fs.name))
        .collect()
}

pub fn render_filesystems() -> String {
    let mut out = String::new();
    for fs in filesystems() {
        if fs.fs_flags & super::super_block::FS_REQUIRES_DEV == 0 {
            out.push_str("nodev\t");
        }
        out.push_str(fs.name);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::super_block::{FileSystemType, register_filesystem};
    use crate::fs::types::{SuperBlock, SuperBlockRef};
    use crate::include::uapi::errno::EBUSY;

    fn test_mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        Ok(SuperBlock::alloc(
            "phase6_testfs",
            0x706836,
            &crate::fs::ops::NOOP_SUPER_OPS,
        ))
    }

    #[test]
    fn registry_lists_and_rejects_duplicates() {
        let fs = FileSystemType {
            name: "phase6_testfs",
            mount: test_mount,
            fs_flags: 0,
        };
        let first = register_filesystem(fs);
        assert!(first.is_ok() || first == Err(EBUSY));
        assert_eq!(register_filesystem(fs), Err(EBUSY));
        assert!(get_fs_type("phase6_testfs").is_some());
        assert!(
            filesystem_names()
                .iter()
                .any(|name| name == "phase6_testfs")
        );
    }
}
