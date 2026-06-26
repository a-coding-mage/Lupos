//! linux-parity: complete
//! linux-source: vendor/linux/fs/ext2/symlink.c
//! test-origin: linux:vendor/linux/fs/ext2/symlink.c
//! ext2 symlink inode operation tables.

pub const SYMLINK_OPERATIONS_SYMBOL: &str = "ext2_symlink_inode_operations";
pub const FAST_SYMLINK_OPERATIONS_SYMBOL: &str = "ext2_fast_symlink_inode_operations";
pub const SYMLINK_GET_LINK: &str = "page_get_link";
pub const FAST_SYMLINK_GET_LINK: &str = "simple_get_link";
pub const SHARED_OPERATIONS: &[(&str, &str)] = &[
    ("getattr", "ext2_getattr"),
    ("setattr", "ext2_setattr"),
    ("listxattr", "ext2_listxattr"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2InodeOperations {
    pub symbol: &'static str,
    pub get_link: &'static str,
    pub getattr: &'static str,
    pub setattr: &'static str,
    pub listxattr: &'static str,
}

pub const EXT2_SYMLINK_INODE_OPERATIONS: Ext2InodeOperations = Ext2InodeOperations {
    symbol: SYMLINK_OPERATIONS_SYMBOL,
    get_link: SYMLINK_GET_LINK,
    getattr: "ext2_getattr",
    setattr: "ext2_setattr",
    listxattr: "ext2_listxattr",
};

pub const EXT2_FAST_SYMLINK_INODE_OPERATIONS: Ext2InodeOperations = Ext2InodeOperations {
    symbol: FAST_SYMLINK_OPERATIONS_SYMBOL,
    get_link: FAST_SYMLINK_GET_LINK,
    getattr: "ext2_getattr",
    setattr: "ext2_setattr",
    listxattr: "ext2_listxattr",
};

pub const fn ext2_symlink_operations(fast: bool) -> Ext2InodeOperations {
    if fast {
        EXT2_FAST_SYMLINK_INODE_OPERATIONS
    } else {
        EXT2_SYMLINK_INODE_OPERATIONS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext2_symlink_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ext2/symlink.c"
        ));
        assert!(source.contains("#include \"ext2.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("Only fast symlinks left here"));
        assert!(source.contains(SYMLINK_OPERATIONS_SYMBOL));
        assert!(source.contains(FAST_SYMLINK_OPERATIONS_SYMBOL));
        assert!(source.contains(".get_link\t= page_get_link"));
        assert!(source.contains(".get_link\t= simple_get_link"));
        assert!(source.contains(".getattr\t= ext2_getattr"));
        assert!(source.contains(".setattr\t= ext2_setattr"));
        assert!(source.contains(".listxattr\t= ext2_listxattr"));
        for (slot, target) in SHARED_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }

        assert_eq!(
            ext2_symlink_operations(false),
            Ext2InodeOperations {
                symbol: "ext2_symlink_inode_operations",
                get_link: "page_get_link",
                getattr: "ext2_getattr",
                setattr: "ext2_setattr",
                listxattr: "ext2_listxattr",
            }
        );
        assert_eq!(
            ext2_symlink_operations(true),
            Ext2InodeOperations {
                symbol: "ext2_fast_symlink_inode_operations",
                get_link: "simple_get_link",
                getattr: "ext2_getattr",
                setattr: "ext2_setattr",
                listxattr: "ext2_listxattr",
            }
        );
    }
}
