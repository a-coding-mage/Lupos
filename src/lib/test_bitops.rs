//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_bitops.c
//! test-origin: linux:vendor/linux/lib/test_bitops.c
//! Bit operation Linux test-module constants and count-order checks.

pub const BITOPS_SET_BITS: [usize; 5] = [4, 7, 11, 31, 88];
pub const BITOPS_LAST: usize = 255;
pub const BITOPS_LENGTH: usize = 256;
pub const ORDER_COMB: [(u32, u32); 7] = [
    (0x00000003, 2),
    (0x00000004, 2),
    (0x00001fff, 13),
    (0x00002000, 13),
    (0x50000000, 31),
    (0x80000000, 31),
    (0x80003000, 32),
];
pub const ORDER_COMB_LONG: [(u64, u32); 7] = [
    (0x0000000300000000, 34),
    (0x0000000400000000, 34),
    (0x00001fff00000000, 45),
    (0x0000200000000000, 45),
    (0x5000000000000000, 63),
    (0x8000000000000000, 63),
    (0x8000300000000000, 64),
];
pub const MODULE_DESCRIPTION: &str = "Bit testing module";

pub const fn get_count_order(value: u64) -> u32 {
    if value <= 1 {
        0
    } else {
        u64::BITS - (value - 1).leading_zeros()
    }
}

pub fn set_and_clear_bitmap() -> Option<usize> {
    let mut bitmap = [false; BITOPS_LENGTH];
    for bit in BITOPS_SET_BITS {
        bitmap[bit] = true;
    }
    for bit in BITOPS_SET_BITS {
        bitmap[bit] = false;
    }
    bitmap
        .iter()
        .take(BITOPS_LAST)
        .position(|bit| *bit)
        .or(Some(BITOPS_LAST))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitops_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_bitops.c"
        ));

        assert!(source.contains("enum bitops_fun"));
        for token in [
            "BITOPS_4 = 4",
            "BITOPS_7 = 7",
            "BITOPS_11 = 11",
            "BITOPS_31 = 31",
            "BITOPS_88 = 88",
        ] {
            assert!(source.contains(token));
        }
        assert!(source.contains("static DECLARE_BITMAP(g_bitmap, BITOPS_LENGTH);"));
        assert!(source.contains("static unsigned int order_comb[][2]"));
        assert!(source.contains("static unsigned long order_comb_long[][2]"));
        assert!(source.contains("get_count_order(order_comb[i][0])"));
        assert!(source.contains("get_count_order_long(order_comb_long[i][0])"));
        assert!(source.contains("set_bit(BITOPS_4, g_bitmap);"));
        assert!(source.contains("clear_bit(BITOPS_88, g_bitmap);"));
        assert!(source.contains("find_first_bit(g_bitmap, BITOPS_LAST);"));
        assert!(source.contains("module_init(test_bitops_startup);"));
        assert!(source.contains(MODULE_DESCRIPTION));

        for (value, order) in ORDER_COMB {
            assert_eq!(get_count_order(value as u64), order);
        }
        for (value, order) in ORDER_COMB_LONG {
            assert_eq!(get_count_order(value), order);
        }
        assert_eq!(set_and_clear_bitmap(), Some(BITOPS_LAST));
    }
}
