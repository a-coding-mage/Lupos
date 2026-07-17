//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/scrub/trace.c
//! test-origin: linux:vendor/linux/fs/xfs/scrub/trace.c
//! XFS scrub trace helper for btree cursor locations.

pub const NULLFSBLOCK: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfsBtreeType {
    Inode,
    Other,
}

pub const fn xchk_btree_cur_fsbno(
    level: i32,
    cur_nlevels: i32,
    level_buffer_fsb: Option<u64>,
    btree_type: XfsBtreeType,
    inode_fsb: u64,
) -> u64 {
    if level >= 0 && level < cur_nlevels {
        if let Some(fsb) = level_buffer_fsb {
            return fsb;
        }
    }
    if level == cur_nlevels - 1 {
        match btree_type {
            XfsBtreeType::Inode => return inode_fsb,
            XfsBtreeType::Other => {}
        }
    }
    NULLFSBLOCK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_trace_helper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/scrub/trace.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"scrub/dirtree.h\""));
        assert!(source.contains("xchk_btree_cur_fsbno"));
        assert!(source.contains("if (level < cur->bc_nlevels && cur->bc_levels[level].bp)"));
        assert!(source.contains("return XFS_DADDR_TO_FSB(cur->bc_mp,"));
        assert!(source.contains("if (level == cur->bc_nlevels - 1 &&"));
        assert!(source.contains("cur->bc_ops->type == XFS_BTREE_TYPE_INODE"));
        assert!(source.contains("return XFS_INODE_TO_FSB(cur->bc_ino.ip);"));
        assert!(source.contains("return NULLFSBLOCK;"));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("#include \"scrub/trace.h\""));

        assert_eq!(
            xchk_btree_cur_fsbno(1, 3, Some(44), XfsBtreeType::Other, 99),
            44
        );
        assert_eq!(
            xchk_btree_cur_fsbno(2, 3, None, XfsBtreeType::Inode, 99),
            99
        );
        assert_eq!(
            xchk_btree_cur_fsbno(2, 3, None, XfsBtreeType::Other, 99),
            NULLFSBLOCK
        );
    }
}
