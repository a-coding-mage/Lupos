//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/symlink.c
//! test-origin: linux:vendor/linux/fs/jffs2/symlink.c
//! JFFS2 symlink inode operations.

pub const INODE_OPERATIONS_SYMBOL: &str = "jffs2_symlink_inode_operations";
pub const INODE_OPERATIONS: &[(&str, &str)] = &[
    ("get_link", "simple_get_link"),
    ("setattr", "jffs2_setattr"),
    ("listxattr", "jffs2_listxattr"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_symlink_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/symlink.c"
        ));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(source.contains(INODE_OPERATIONS_SYMBOL));
        for (slot, target) in INODE_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }
}
