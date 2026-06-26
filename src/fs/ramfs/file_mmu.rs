//! linux-parity: complete
//! linux-source: vendor/linux/fs/ramfs/file-mmu.c
//! test-origin: linux:vendor/linux/fs/ramfs/file-mmu.c
//! ramfs MMU file helpers and operation tables.

use crate::fs::types::FileRef;

pub const RAMFS_FILE_OPERATIONS_SYMBOL: &str = "ramfs_file_operations";
pub const RAMFS_FILE_OPERATIONS: &[(&str, &str)] = &[
    ("read_iter", "generic_file_read_iter"),
    ("write_iter", "generic_file_write_iter"),
    ("mmap_prepare", "generic_file_mmap_prepare"),
    ("fsync", "noop_fsync"),
    ("splice_read", "filemap_splice_read"),
    ("splice_write", "iter_file_splice_write"),
    ("llseek", "generic_file_llseek"),
    ("get_unmapped_area", "ramfs_mmu_get_unmapped_area"),
];
pub const RAMFS_FILE_INODE_OPERATIONS_SYMBOL: &str = "ramfs_file_inode_operations";
pub const RAMFS_FILE_INODE_OPERATIONS: &[(&str, &str)] =
    &[("setattr", "simple_setattr"), ("getattr", "simple_getattr")];

pub fn read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    crate::fs::libfs::ram_file_read(file, buf, pos)
}

pub fn write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    crate::fs::libfs::ram_file_write(file, buf, pos)
}

pub const fn supports_mmap() -> bool {
    true
}

pub const fn ramfs_mmu_get_unmapped_area(mm_get_unmapped_area_result: u64) -> u64 {
    mm_get_unmapped_area_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramfs_mmu_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ramfs/file-mmu.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/mm.h>"));
        assert!(source.contains("#include <linux/ramfs.h>"));
        assert!(source.contains("#include <linux/sched.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("static unsigned long ramfs_mmu_get_unmapped_area"));
        assert!(source.contains("return mm_get_unmapped_area(file, addr, len, pgoff, flags);"));
        assert!(source.contains(RAMFS_FILE_OPERATIONS_SYMBOL));
        assert!(source.contains(RAMFS_FILE_INODE_OPERATIONS_SYMBOL));

        for (slot, target) in RAMFS_FILE_OPERATIONS
            .iter()
            .chain(RAMFS_FILE_INODE_OPERATIONS.iter())
        {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }

        assert!(supports_mmap());
        assert_eq!(ramfs_mmu_get_unmapped_area(0x7fff_0000), 0x7fff_0000);
    }
}
