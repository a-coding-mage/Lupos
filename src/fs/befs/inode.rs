//! linux-parity: complete
//! linux-source: vendor/linux/fs/befs/inode.c
//! test-origin: linux:vendor/linux/fs/befs/inode.c
//! BeFS raw inode validation checks.

pub const BEFS_INODE_MAGIC1: u32 = 0x3bbe0ad9;
pub const BEFS_INODE_IN_USE: u32 = 0x0000_0001;
pub const BEFS_OK: i32 = 0;
pub const BEFS_BAD_INODE: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BefsRawInodeView {
    pub magic1: u32,
    pub inode_block: u64,
    pub stored_inode_block: u64,
    pub flags: u32,
}

pub const fn befs_check_inode(raw: BefsRawInodeView) -> i32 {
    if raw.magic1 != BEFS_INODE_MAGIC1 {
        return BEFS_BAD_INODE;
    }
    if raw.inode_block != raw.stored_inode_block {
        return BEFS_BAD_INODE;
    }
    if raw.flags & BEFS_INODE_IN_USE == 0 {
        return BEFS_BAD_INODE;
    }
    BEFS_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn befs_check_inode_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/befs/inode.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include \"befs.h\""));
        assert!(source.contains("#include \"inode.h\""));
        assert!(source.contains("int"));
        assert!(source.contains("befs_check_inode"));
        assert!(source.contains("fs32_to_cpu(sb, raw_inode->magic1)"));
        assert!(source.contains("fsrun_to_cpu(sb, raw_inode->inode_num)"));
        assert!(source.contains("fs32_to_cpu(sb, raw_inode->flags)"));
        assert!(source.contains("magic1 != BEFS_INODE_MAGIC1"));
        assert!(source.contains("return BEFS_BAD_INODE;"));
        assert!(source.contains("inode != iaddr2blockno(sb, &ino_num)"));
        assert!(source.contains("!(flags & BEFS_INODE_IN_USE)"));
        assert!(source.contains("return BEFS_OK;"));

        let valid = BefsRawInodeView {
            magic1: BEFS_INODE_MAGIC1,
            inode_block: 44,
            stored_inode_block: 44,
            flags: BEFS_INODE_IN_USE,
        };
        assert_eq!(befs_check_inode(valid), BEFS_OK);
        assert_eq!(
            befs_check_inode(BefsRawInodeView { magic1: 0, ..valid }),
            BEFS_BAD_INODE
        );
        assert_eq!(
            befs_check_inode(BefsRawInodeView {
                stored_inode_block: 45,
                ..valid
            }),
            BEFS_BAD_INODE
        );
        assert_eq!(
            befs_check_inode(BefsRawInodeView { flags: 0, ..valid }),
            BEFS_BAD_INODE
        );
    }
}
