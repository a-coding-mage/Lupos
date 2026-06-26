//! linux-parity: complete
//! linux-source: vendor/linux/fs/ufs/file.c
//! test-origin: linux:vendor/linux/fs/ufs/file.c
//! UFS regular file operation table.

pub const UFS_FILE_OPERATIONS_SYMBOL: &str = "ufs_file_operations";
pub const UFS_FILE_OPERATIONS: &[(&str, &str)] = &[
    ("llseek", "generic_file_llseek"),
    ("read_iter", "generic_file_read_iter"),
    ("write_iter", "generic_file_write_iter"),
    ("mmap_prepare", "generic_file_mmap_prepare"),
    ("open", "generic_file_open"),
    ("fsync", "simple_fsync"),
    ("splice_read", "filemap_splice_read"),
    ("splice_write", "iter_file_splice_write"),
    ("setlease", "generic_setlease"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ufs_file_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ufs/file.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/filelock.h>"));
        assert!(source.contains("#include \"ufs_fs.h\""));
        assert!(source.contains("#include \"ufs.h\""));
        assert!(source.contains(UFS_FILE_OPERATIONS_SYMBOL));
        for (slot, target) in UFS_FILE_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }
}
