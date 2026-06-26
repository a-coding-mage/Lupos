//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/libxfs/xfs_bit.c
//! test-origin: linux:vendor/linux/fs/xfs/libxfs/xfs_bit.c
//! XFS non-realtime bitmap helpers.

pub const NBWORD: u32 = 32;
pub const BIT_TO_WORD_SHIFT: u32 = 5;

pub fn xfs_bitmap_empty(map: &[u32], size: usize) -> bool {
    map.iter().take(size).all(|word| *word == 0)
}

pub fn xfs_contig_bits(map: &[u32], size: u32, mut start_bit: u32) -> i32 {
    let mut p = (start_bit >> BIT_TO_WORD_SHIFT) as usize;
    let mut result = 0u32;
    let mut size_bits = size << BIT_TO_WORD_SHIFT;

    assert!(start_bit < size_bits);
    size_bits -= start_bit & !(NBWORD - 1);
    start_bit &= NBWORD - 1;
    if start_bit != 0 {
        let mut tmp = map[p];
        p += 1;
        tmp |= u32::MAX >> (NBWORD - start_bit);
        if tmp != u32::MAX {
            return (result + (!tmp).trailing_zeros() - start_bit) as i32;
        }
        result += NBWORD;
        size_bits -= NBWORD;
    }
    while size_bits != 0 {
        let tmp = map[p];
        p += 1;
        if tmp != u32::MAX {
            return (result + (!tmp).trailing_zeros() - start_bit) as i32;
        }
        result += NBWORD;
        size_bits -= NBWORD;
    }
    (result - start_bit) as i32
}

pub fn xfs_next_bit(map: &[u32], size: u32, mut start_bit: u32) -> i32 {
    let mut p = (start_bit >> BIT_TO_WORD_SHIFT) as usize;
    let mut result = start_bit & !(NBWORD - 1);
    let mut size_bits = size << BIT_TO_WORD_SHIFT;

    if start_bit >= size_bits {
        return -1;
    }
    size_bits -= result;
    start_bit &= NBWORD - 1;
    if start_bit != 0 {
        let tmp = map[p] & (u32::MAX << start_bit);
        p += 1;
        if tmp != 0 {
            return (result + tmp.trailing_zeros()) as i32;
        }
        result += NBWORD;
        size_bits -= NBWORD;
    }
    while size_bits != 0 {
        let tmp = map[p];
        p += 1;
        if tmp != 0 {
            return (result + tmp.trailing_zeros()) as i32;
        }
        result += NBWORD;
        size_bits -= NBWORD;
    }
    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_bit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/libxfs/xfs_bit.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"xfs_log_format.h\""));
        assert!(source.contains("#include \"xfs_bit.h\""));
        assert!(source.contains("xfs_bitmap_empty(uint *map, uint size)"));
        assert!(source.contains("if (map[i] != 0)"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("xfs_contig_bits(uint *map, uint\tsize, uint start_bit)"));
        assert!(source.contains("size <<= BIT_TO_WORD_SHIFT;"));
        assert!(source.contains("ASSERT(start_bit < size);"));
        assert!(source.contains("tmp |= (~0U >> (NBWORD-start_bit));"));
        assert!(source.contains("return result + ffz(tmp) - start_bit;"));
        assert!(source.contains("int xfs_next_bit(uint *map, uint size, uint start_bit)"));
        assert!(source.contains("if (start_bit >= size)"));
        assert!(source.contains("return -1;"));
        assert!(source.contains("tmp &= (~0U << start_bit);"));
        assert!(source.contains("return result + ffs(tmp) - 1;"));

        assert!(xfs_bitmap_empty(&[0, 0, 7], 2));
        assert!(!xfs_bitmap_empty(&[0, 1], 2));
        assert_eq!(xfs_contig_bits(&[0b1111, 0], 2, 0), 4);
        assert_eq!(xfs_contig_bits(&[u32::MAX, 0b11], 2, 0), 34);
        assert_eq!(xfs_contig_bits(&[0b1111_0000], 1, 4), 4);
        assert_eq!(xfs_next_bit(&[0, 0b1000], 2, 0), 35);
        assert_eq!(xfs_next_bit(&[0b1000_0000], 1, 4), 7);
        assert_eq!(xfs_next_bit(&[0], 1, 32), -1);
        assert_eq!(xfs_next_bit(&[0], 1, 0), -1);
    }
}
