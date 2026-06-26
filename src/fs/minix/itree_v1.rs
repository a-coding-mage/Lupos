//! linux-parity: complete
//! linux-source: vendor/linux/fs/minix/itree_v1.c
//! test-origin: linux:vendor/linux/fs/minix/itree_v1.c
//! Minix V1 block tree geometry.

pub const DEPTH: usize = 3;
pub const DIRECT: i64 = 7;
pub const INDIRECT_BLOCK_ENTRIES: i64 = 512;
pub const BLOCK_SIZE_BITS: u32 = 10;
pub const BLOCK_SIZE: u64 = 1 << BLOCK_SIZE_BITS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinixBlockPath {
    pub depth: usize,
    pub offsets: [i64; DEPTH],
}

pub const fn block_to_path(block: i64, maxbytes: u64) -> MinixBlockPath {
    let mut offsets = [0; DEPTH];
    if block < 0 {
        return MinixBlockPath { depth: 0, offsets };
    }
    if (block as u64).saturating_mul(BLOCK_SIZE) >= maxbytes {
        return MinixBlockPath { depth: 0, offsets };
    }

    if block < DIRECT {
        offsets[0] = block;
        return MinixBlockPath { depth: 1, offsets };
    }

    let mut remaining = block - DIRECT;
    if remaining < INDIRECT_BLOCK_ENTRIES {
        offsets[0] = DIRECT;
        offsets[1] = remaining;
        return MinixBlockPath { depth: 2, offsets };
    }

    remaining -= INDIRECT_BLOCK_ENTRIES;
    offsets[0] = DIRECT + 1;
    offsets[1] = remaining >> 9;
    offsets[2] = remaining & 511;
    MinixBlockPath { depth: 3, offsets }
}

pub const fn block_to_cpu(block: u16) -> u64 {
    block as u64
}

pub const fn cpu_to_block(block: u64) -> u16 {
    block as u16
}

pub const fn v1_minix_blocks(size: u64, block_size: u64, block_size_bits: u32) -> u64 {
    if block_size == 0 {
        return 0;
    }
    let ptrs = block_size / 2;
    if ptrs == 0 {
        return 0;
    }

    let mut blocks = size.saturating_add(block_size - 1) >> block_size_bits;
    let mut res = blocks;
    let mut direct = DIRECT as u64;
    let mut i = DEPTH;
    loop {
        i -= 1;
        if i == 0 || blocks <= direct {
            break;
        }
        blocks -= direct;
        blocks += ptrs - 1;
        blocks /= ptrs;
        res += blocks;
        direct = 1;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minix_v1_itree_geometry_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/minix/itree_v1.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"minix.h\""));
        assert!(source.contains("enum {DEPTH = 3, DIRECT = 7};"));
        assert!(source.contains("typedef u16 block_t;"));
        assert!(source.contains("static inline unsigned long block_to_cpu(block_t n)"));
        assert!(source.contains("static inline block_t cpu_to_block(unsigned long n)"));
        assert!(source.contains("static int block_to_path"));
        assert!(source.contains("if (block < 0)"));
        assert!(source.contains("if ((u64)block * BLOCK_SIZE >= inode->i_sb->s_maxbytes)"));
        assert!(source.contains("if (block < 7)"));
        assert!(source.contains("} else if ((block -= 7) < 512)"));
        assert!(source.contains("offsets[n++] = 8;"));
        assert!(source.contains("offsets[n++] = block>>9;"));
        assert!(source.contains("offsets[n++] = block & 511;"));
        assert!(source.contains("#include \"itree_common.c\""));
        assert!(source.contains("V1_minix_get_block"));
        assert!(source.contains("return get_block(inode, block, bh_result, create);"));
        assert!(source.contains("V1_minix_truncate"));
        assert!(source.contains("truncate(inode);"));
        assert!(source.contains("V1_minix_blocks"));
        assert!(source.contains("return nblocks(size, sb);"));

        assert_eq!(block_to_path(-1, u64::MAX).depth, 0);
        assert_eq!(block_to_path(7, 7 * BLOCK_SIZE).depth, 0);
        assert_eq!(
            block_to_path(6, u64::MAX),
            MinixBlockPath {
                depth: 1,
                offsets: [6, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7, u64::MAX),
            MinixBlockPath {
                depth: 2,
                offsets: [7, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7 + 512, u64::MAX),
            MinixBlockPath {
                depth: 3,
                offsets: [8, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7 + 512 + 513, u64::MAX),
            MinixBlockPath {
                depth: 3,
                offsets: [8, 1, 1],
            }
        );
        assert_eq!(block_to_cpu(42), 42);
        assert_eq!(cpu_to_block(42), 42);
        assert_eq!(v1_minix_blocks(0, 1024, 10), 0);
        assert_eq!(v1_minix_blocks(7 * 1024, 1024, 10), 7);
        assert_eq!(v1_minix_blocks(8 * 1024, 1024, 10), 9);
    }
}
