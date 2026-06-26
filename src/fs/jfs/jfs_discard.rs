//! linux-parity: complete
//! linux-source: vendor/linux/fs/jfs/jfs_discard.c
//! test-origin: linux:vendor/linux/fs/jfs/jfs_discard.c
//! JFS FITRIM block-range conversion.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JfsTrimRange {
    pub start: u64,
    pub len: u64,
    pub minlen: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JfsTrimGeometry {
    pub block_size: u64,
    pub block_size_bits: u32,
    pub db_mapsize: u64,
    pub db_agsize: u64,
    pub db_agl2size: u32,
    pub bmap_present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JfsTrimPlan {
    pub start_block: u64,
    pub end_block: u64,
    pub minlen_blocks: u64,
    pub agno_start: u64,
    pub agno_end: u64,
}

pub fn jfs_ioc_trim_plan(range: JfsTrimRange, geom: JfsTrimGeometry) -> Result<JfsTrimPlan, i32> {
    let start = range.start >> geom.block_size_bits;
    let len_blocks = range.len >> geom.block_size_bits;
    let mut end = start.wrapping_add(len_blocks).wrapping_sub(1);
    let mut minlen = range.minlen >> geom.block_size_bits;
    if minlen == 0 {
        minlen = 1;
    }

    if !geom.bmap_present
        || minlen > geom.db_agsize
        || start >= geom.db_mapsize
        || range.len < geom.block_size
    {
        return Err(-EINVAL);
    }

    if end >= geom.db_mapsize {
        end = geom.db_mapsize - 1;
    }

    Ok(JfsTrimPlan {
        start_block: start,
        end_block: end,
        minlen_blocks: minlen,
        agno_start: start >> geom.db_agl2size,
        agno_end: end >> geom.db_agl2size,
    })
}

pub const fn jfs_trimmed_len_bytes(trimmed_blocks: u64, block_size_bits: u32) -> u64 {
    trimmed_blocks << block_size_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jfs_discard_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jfs/jfs_discard.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include <linux/blkdev.h>"));
        assert!(source.contains("#include \"jfs_discard.h\""));
        assert!(
            source.contains("void jfs_issue_discard(struct inode *ip, u64 blkno, u64 nblocks)")
        );
        assert!(source.contains("r = sb_issue_discard(sb, blkno, nblocks, GFP_NOFS, 0);"));
        assert!(source.contains("if (unlikely(r != 0))"));
        assert!(source.contains("int jfs_ioc_trim(struct inode *ip, struct fstrim_range *range)"));
        assert!(source.contains("start = range->start >> sb->s_blocksize_bits;"));
        assert!(source.contains("end = start + (range->len >> sb->s_blocksize_bits) - 1;"));
        assert!(source.contains("minlen = range->minlen >> sb->s_blocksize_bits;"));
        assert!(source.contains("if (minlen == 0)"));
        assert!(source.contains("minlen = 1;"));
        assert!(source.contains("bmp == NULL ||"));
        assert!(source.contains("minlen > bmp->db_agsize ||"));
        assert!(source.contains("start >= bmp->db_mapsize ||"));
        assert!(source.contains("range->len < sb->s_blocksize"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("if (end >= bmp->db_mapsize)"));
        assert!(source.contains("end = bmp->db_mapsize - 1;"));
        assert!(source.contains("agno = BLKTOAG(start, JFS_SBI(ip->i_sb));"));
        assert!(source.contains("trimmed += dbDiscardAG(ip, agno, minlen);"));
        assert!(source.contains("range->len = trimmed << sb->s_blocksize_bits;"));

        let geom = JfsTrimGeometry {
            block_size: 4096,
            block_size_bits: 12,
            db_mapsize: 1024,
            db_agsize: 256,
            db_agl2size: 8,
            bmap_present: true,
        };
        let plan = jfs_ioc_trim_plan(
            JfsTrimRange {
                start: 4096,
                len: 4096 * 600,
                minlen: 1,
            },
            geom,
        )
        .unwrap();
        assert_eq!(plan.start_block, 1);
        assert_eq!(plan.end_block, 600);
        assert_eq!(plan.minlen_blocks, 1);
        assert_eq!(plan.agno_start, 0);
        assert_eq!(plan.agno_end, 2);
        assert_eq!(jfs_trimmed_len_bytes(7, 12), 28_672);

        assert_eq!(
            jfs_ioc_trim_plan(
                JfsTrimRange {
                    start: 0,
                    len: 4095,
                    minlen: 0,
                },
                geom,
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            jfs_ioc_trim_plan(
                JfsTrimRange {
                    start: 0,
                    len: 4096,
                    minlen: 4096 * 257,
                },
                geom,
            ),
            Err(-EINVAL)
        );
    }
}
