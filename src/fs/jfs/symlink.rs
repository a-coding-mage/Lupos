//! linux-parity: complete
//! linux-source: vendor/linux/fs/jfs/symlink.c
//! test-origin: linux:vendor/linux/fs/jfs/symlink.c
//! JFS symlink inode operations.

pub const FAST_SYMLINK_OPERATIONS_SYMBOL: &str = "jfs_fast_symlink_inode_operations";
pub const SYMLINK_OPERATIONS_SYMBOL: &str = "jfs_symlink_inode_operations";
pub const FAST_GET_LINK: &str = "simple_get_link";
pub const PAGE_GET_LINK: &str = "page_get_link";
pub const SHARED_OPERATIONS: &[(&str, &str)] =
    &[("setattr", "jfs_setattr"), ("listxattr", "jfs_listxattr")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JfsInodeOperations {
    pub symbol: &'static str,
    pub get_link: &'static str,
    pub setattr: &'static str,
    pub listxattr: &'static str,
}

pub const JFS_FAST_SYMLINK_INODE_OPERATIONS: JfsInodeOperations = JfsInodeOperations {
    symbol: FAST_SYMLINK_OPERATIONS_SYMBOL,
    get_link: FAST_GET_LINK,
    setattr: "jfs_setattr",
    listxattr: "jfs_listxattr",
};

pub const JFS_SYMLINK_INODE_OPERATIONS: JfsInodeOperations = JfsInodeOperations {
    symbol: SYMLINK_OPERATIONS_SYMBOL,
    get_link: PAGE_GET_LINK,
    setattr: "jfs_setattr",
    listxattr: "jfs_listxattr",
};

pub const fn jfs_symlink_operations(fast: bool) -> JfsInodeOperations {
    if fast {
        JFS_FAST_SYMLINK_INODE_OPERATIONS
    } else {
        JFS_SYMLINK_INODE_OPERATIONS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jfs_symlink_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jfs/symlink.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include \"jfs_incore.h\""));
        assert!(source.contains("#include \"jfs_inode.h\""));
        assert!(source.contains("#include \"jfs_xattr.h\""));
        assert!(source.contains(FAST_SYMLINK_OPERATIONS_SYMBOL));
        assert!(source.contains(SYMLINK_OPERATIONS_SYMBOL));
        assert!(source.contains(".get_link\t= simple_get_link"));
        assert!(source.contains(".get_link\t= page_get_link"));
        assert!(source.contains(".setattr\t= jfs_setattr"));
        assert!(source.contains(".listxattr\t= jfs_listxattr"));
        for (slot, target) in SHARED_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }

        assert_eq!(
            jfs_symlink_operations(true),
            JfsInodeOperations {
                symbol: "jfs_fast_symlink_inode_operations",
                get_link: "simple_get_link",
                setattr: "jfs_setattr",
                listxattr: "jfs_listxattr",
            }
        );
        assert_eq!(
            jfs_symlink_operations(false),
            JfsInodeOperations {
                symbol: "jfs_symlink_inode_operations",
                get_link: "page_get_link",
                setattr: "jfs_setattr",
                listxattr: "jfs_listxattr",
            }
        );
    }
}
