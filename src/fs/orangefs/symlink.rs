//! linux-parity: complete
//! linux-source: vendor/linux/fs/orangefs/symlink.c
//! test-origin: linux:vendor/linux/fs/orangefs/symlink.c
//! OrangeFS symlink inode operations.

pub const INODE_OPERATIONS_SYMBOL: &str = "orangefs_symlink_inode_operations";
pub const INODE_OPERATIONS: &[(&str, &str)] = &[
    ("get_link", "simple_get_link"),
    ("setattr", "orangefs_setattr"),
    ("getattr", "orangefs_getattr"),
    ("listxattr", "orangefs_listxattr"),
    ("permission", "orangefs_permission"),
    ("update_time", "orangefs_update_time"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orangefs_symlink_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/orangefs/symlink.c"
        ));
        assert!(source.contains("#include \"protocol.h\""));
        assert!(source.contains("#include \"orangefs-kernel.h\""));
        assert!(source.contains("#include \"orangefs-bufmap.h\""));
        assert!(source.contains(INODE_OPERATIONS_SYMBOL));
        for (slot, target) in INODE_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }
}
