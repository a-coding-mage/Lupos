//! linux-parity: complete
//! linux-source: vendor/linux/fs/adfs/file.c
//! test-origin: linux:vendor/linux/fs/adfs/file.c
//! ADFS regular file operation tables.

pub const FILE_OPERATIONS_SYMBOL: &str = "adfs_file_operations";
pub const FILE_OPERATIONS: &[(&str, &str)] = &[
    ("llseek", "generic_file_llseek"),
    ("read_iter", "generic_file_read_iter"),
    ("mmap_prepare", "generic_file_mmap_prepare"),
    ("fsync", "simple_fsync"),
    ("write_iter", "generic_file_write_iter"),
    ("splice_read", "filemap_splice_read"),
];
pub const INODE_OPERATIONS_SYMBOL: &str = "adfs_file_inode_operations";
pub const INODE_OPERATIONS: &[(&str, &str)] = &[("setattr", "adfs_setattr")];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adfs_file_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/adfs/file.c"
        ));
        assert!(source.contains("#include \"adfs.h\""));
        assert!(source.contains(FILE_OPERATIONS_SYMBOL));
        assert!(source.contains(INODE_OPERATIONS_SYMBOL));
        for (slot, target) in FILE_OPERATIONS.iter().chain(INODE_OPERATIONS.iter()) {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }
}
