//! linux-parity: complete
//! linux-source: vendor/linux/fs/qnx4/bitmap.c
//! test-origin: linux:vendor/linux/fs/qnx4/bitmap.c
//! QNX4 free-block bitmap counting.

use crate::lib::memweight::memweight_bytes;

pub const QNX4_BLOCK_SIZE: usize = 0x200;
pub const BITS_PER_BYTE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qnx4BitmapBlock<'a> {
    pub block: i64,
    pub data: &'a [u8],
}

pub fn qnx4_count_free_blocks(
    bitmap_first_xtnt_blk: u32,
    bitmap_size: usize,
    blocks: &[Qnx4BitmapBlock<'_>],
) -> usize {
    let start = bitmap_first_xtnt_blk as i64 - 1;
    let mut total = 0usize;
    let mut total_free = 0usize;
    let mut offset = 0i64;

    while total < bitmap_size {
        let bytes = core::cmp::min(bitmap_size - total, QNX4_BLOCK_SIZE);
        let block_no = start + offset;
        let Some(block) = blocks.iter().find(|block| block.block == block_no) else {
            break;
        };
        let bytes = core::cmp::min(bytes, block.data.len());
        total_free += bytes * BITS_PER_BYTE - memweight_bytes(&block.data[..bytes]);
        total += bytes;
        offset += 1;
    }

    total_free
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qnx4_bitmap_count_matches_linux_block_scan() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/qnx4/bitmap.c"
        ));
        assert!(source.contains("qnx4_count_free_blocks"));
        assert!(source.contains("di_first_xtnt.xtnt_blk) - 1"));
        assert!(source.contains("bytes = min(size - total, QNX4_BLOCK_SIZE);"));
        assert!(source.contains("sb_bread(sb, start + offset)"));
        assert!(source.contains("total_free += bytes * BITS_PER_BYTE -"));
        assert!(source.contains("memweight(bh->b_data, bytes);"));
        assert!(source.contains("brelse(bh);"));

        let block = Qnx4BitmapBlock {
            block: 9,
            data: &[0xff, 0x00, 0b0000_1111],
        };
        assert_eq!(qnx4_count_free_blocks(10, 3, &[block]), 12);
        assert_eq!(qnx4_count_free_blocks(10, 2, &[block]), 8);

        let full = [0xff; QNX4_BLOCK_SIZE];
        let blocks = [Qnx4BitmapBlock {
            block: 1,
            data: &full,
        }];
        assert_eq!(qnx4_count_free_blocks(2, QNX4_BLOCK_SIZE + 1, &blocks), 0);
    }
}
