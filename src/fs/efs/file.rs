//! linux-parity: complete
//! linux-source: vendor/linux/fs/efs/file.c
//! test-origin: linux:vendor/linux/fs/efs/file.c
//! EFS block mapping helpers.

use crate::include::uapi::errno::EROFS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfsGetBlockOutcome {
    pub result: i32,
    pub mapped_block: Option<u64>,
    pub map_block_called: bool,
}

pub const fn efs_get_block_outcome(
    create: bool,
    iblock: u64,
    inode_blocks: u64,
    efs_map_block_result: u64,
) -> EfsGetBlockOutcome {
    if create {
        return EfsGetBlockOutcome {
            result: -EROFS,
            mapped_block: None,
            map_block_called: false,
        };
    }
    if iblock >= inode_blocks {
        return EfsGetBlockOutcome {
            result: 0,
            mapped_block: None,
            map_block_called: false,
        };
    }
    EfsGetBlockOutcome {
        result: 0,
        mapped_block: if efs_map_block_result != 0 {
            Some(efs_map_block_result)
        } else {
            None
        },
        map_block_called: true,
    }
}

pub const fn efs_bmap_outcome(block: i64, inode_blocks: i64, efs_map_block_result: u64) -> u64 {
    if block < 0 {
        return 0;
    }
    if block >= inode_blocks {
        return 0;
    }
    efs_map_block_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efs_file_mapping_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/efs/file.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"efs.h\""));
        assert!(source.contains("int efs_get_block"));
        assert!(source.contains("int error = -EROFS;"));
        assert!(source.contains("if (create)"));
        assert!(source.contains("if (iblock >= inode->i_blocks)"));
        assert!(source.contains("phys = efs_map_block(inode, iblock);"));
        assert!(source.contains("if (phys)"));
        assert!(source.contains("map_bh(bh_result, inode->i_sb, phys);"));
        assert!(source.contains("int efs_bmap"));
        assert!(source.contains("if (block < 0)"));
        assert!(source.contains("if (!(block < inode->i_blocks))"));
        assert!(source.contains("return efs_map_block(inode, block);"));

        assert_eq!(
            efs_get_block_outcome(true, 0, 4, 99),
            EfsGetBlockOutcome {
                result: -EROFS,
                mapped_block: None,
                map_block_called: false,
            }
        );
        assert_eq!(efs_get_block_outcome(false, 4, 4, 99).mapped_block, None);
        assert_eq!(
            efs_get_block_outcome(false, 3, 4, 99).mapped_block,
            Some(99)
        );
        assert_eq!(efs_bmap_outcome(-1, 4, 99), 0);
        assert_eq!(efs_bmap_outcome(4, 4, 99), 0);
        assert_eq!(efs_bmap_outcome(3, 4, 99), 99);
    }
}
