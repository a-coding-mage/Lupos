//! linux-parity: complete
//! linux-source: vendor/linux/fs/minix/itree_v2.c
//! test-origin: linux:vendor/linux/fs/minix/itree_v2.c
//! Minix V2 block tree geometry.

pub const DEPTH: usize = 4;
pub const DIRECT: i64 = 7;
pub const DIRCOUNT: i64 = 7;
pub const BLOCK_SIZE_BITS: u32 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinixV2BlockPath {
    pub depth: usize,
    pub offsets: [i64; DEPTH],
}

pub const fn indirect_block_entries(block_size_bits: u32) -> i64 {
    if block_size_bits < 2 {
        0
    } else {
        1i64 << (block_size_bits - 2)
    }
}

pub const fn block_to_path(
    block: i64,
    block_size: u64,
    block_size_bits: u32,
    maxbytes: u64,
) -> MinixV2BlockPath {
    let mut offsets = [0; DEPTH];
    if block < 0 {
        return MinixV2BlockPath { depth: 0, offsets };
    }
    if (block as u64).saturating_mul(block_size) >= maxbytes {
        return MinixV2BlockPath { depth: 0, offsets };
    }

    let indir = indirect_block_entries(block_size_bits);
    if indir <= 0 {
        return MinixV2BlockPath { depth: 0, offsets };
    }

    if block < DIRCOUNT {
        offsets[0] = block;
        return MinixV2BlockPath { depth: 1, offsets };
    }

    let mut remaining = block - DIRCOUNT;
    if remaining < indir {
        offsets[0] = DIRCOUNT;
        offsets[1] = remaining;
        return MinixV2BlockPath { depth: 2, offsets };
    }

    remaining -= indir;
    let double_indirect = indir * indir;
    if remaining < double_indirect {
        offsets[0] = DIRCOUNT + 1;
        offsets[1] = remaining / indir;
        offsets[2] = remaining % indir;
        return MinixV2BlockPath { depth: 3, offsets };
    }

    remaining -= double_indirect;
    offsets[0] = DIRCOUNT + 2;
    offsets[1] = (remaining / indir) / indir;
    offsets[2] = (remaining / indir) % indir;
    offsets[3] = remaining % indir;
    MinixV2BlockPath { depth: 4, offsets }
}

pub const fn block_to_cpu(block: u32) -> u64 {
    block as u64
}

pub const fn cpu_to_block(block: u64) -> u32 {
    block as u32
}

pub const fn v2_minix_blocks(size: u64, block_size: u64, block_size_bits: u32) -> u64 {
    if block_size == 0 {
        return 0;
    }
    let ptrs = block_size / 4;
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
    fn minix_v2_itree_geometry_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/minix/itree_v2.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"minix.h\""));
        assert!(source.contains("enum {DIRECT = 7, DEPTH = 4};"));
        assert!(source.contains("typedef u32 block_t;"));
        assert!(source.contains("static inline unsigned long block_to_cpu(block_t n)"));
        assert!(source.contains("static inline block_t cpu_to_block(unsigned long n)"));
        assert!(source.contains("#define DIRCOUNT 7"));
        assert!(source.contains("#define INDIRCOUNT(sb) (1 << ((sb)->s_blocksize_bits - 2))"));
        assert!(source.contains("if (block < DIRCOUNT)"));
        assert!(source.contains("offsets[n++] = DIRCOUNT + 2;"));
        assert!(source.contains("offsets[n++] = (block / INDIRCOUNT(sb)) / INDIRCOUNT(sb);"));
        assert!(source.contains("#include \"itree_common.c\""));
        assert!(source.contains("V2_minix_get_block"));
        assert!(source.contains("V2_minix_truncate"));
        assert!(source.contains("V2_minix_blocks"));

        assert_eq!(indirect_block_entries(10), 256);
        assert_eq!(block_to_path(-1, 1024, 10, u64::MAX).depth, 0);
        assert_eq!(block_to_path(7, 1024, 10, 7 * 1024).depth, 0);
        assert_eq!(
            block_to_path(6, 1024, 10, u64::MAX),
            MinixV2BlockPath {
                depth: 1,
                offsets: [6, 0, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7, 1024, 10, u64::MAX),
            MinixV2BlockPath {
                depth: 2,
                offsets: [7, 0, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7 + 256, 1024, 10, u64::MAX),
            MinixV2BlockPath {
                depth: 3,
                offsets: [8, 0, 0, 0],
            }
        );
        assert_eq!(
            block_to_path(7 + 256 + 256 * 256, 1024, 10, u64::MAX),
            MinixV2BlockPath {
                depth: 4,
                offsets: [9, 0, 0, 0],
            }
        );
        assert_eq!(block_to_cpu(42), 42);
        assert_eq!(cpu_to_block(42), 42);
        assert_eq!(v2_minix_blocks(0, 1024, 10), 0);
        assert_eq!(v2_minix_blocks(7 * 1024, 1024, 10), 7);
        assert_eq!(v2_minix_blocks(8 * 1024, 1024, 10), 9);
    }
}
