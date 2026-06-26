//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/test_div64.c
//! test-origin: linux:vendor/linux/lib/math/test_div64.c
//! 64-bit dividend by 32-bit divisor Linux test-module tables.

pub const TEST_DIV64_N_ITER: usize = 1024;
pub const TEST_DIV64_DIVIDENDS: [u64; 12] = [
    0x00000000ab275080,
    0x0000000fe73c1959,
    0x000000e54c0a74b1,
    0x00000d4398ff1ef9,
    0x0000a18c2ee1c097,
    0x00079fb80b072e4a,
    0x0072db27380dd689,
    0x0842f488162e2284,
    0xf66745411d8ab063,
    0xfffffffffffffffb,
    0xfffffffffffffffc,
    0xffffffffffffffff,
];
pub const TEST_DIV64_DIVISORS: [u32; 12] = [
    0x00000009, 0x0000007c, 0x00000204, 0x0000cb5b, 0x00010000, 0x0008a880, 0x003fd3ae, 0x0b658fac,
    0x80000001, 0xdc08b349, 0xfffffffe, 0xffffffff,
];

pub const fn div64_one(dividend: u64, divisor: u32) -> (u64, u32) {
    let divisor64 = divisor as u64;
    ((dividend / divisor64), (dividend % divisor64) as u32)
}

pub fn all_div64_pairs() -> impl Iterator<Item = (usize, usize, u64, u32)> {
    TEST_DIV64_DIVIDENDS
        .iter()
        .copied()
        .enumerate()
        .flat_map(|(i, dividend)| {
            TEST_DIV64_DIVISORS
                .iter()
                .copied()
                .enumerate()
                .map(move |(j, divisor)| {
                    let (quotient, remainder) = div64_one(dividend, divisor);
                    (i, j, quotient, remainder)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div64_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/test_div64.c"
        ));

        assert!(source.contains("#define TEST_DIV64_N_ITER 1024"));
        assert!(source.contains("static const u64 test_div64_dividends[]"));
        assert!(source.contains("static const u32 test_div64_divisors[]"));
        assert!(source.contains("test_div64_results[SIZE_DIV64_DIVIDENDS][SIZE_DIV64_DIVISORS]"));
        assert!(source.contains("#define test_div64_one(dividend, divisor, i, j)"));
        assert!(source.contains("remainder = do_div(quotient, divisor);"));
        assert!(source.contains("module_init(test_div64_init);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert_eq!(TEST_DIV64_N_ITER, 1024);
        assert_eq!(TEST_DIV64_DIVIDENDS.len(), 12);
        assert_eq!(TEST_DIV64_DIVISORS.len(), 12);

        assert_eq!(div64_one(0x00000000ab275080, 0x00000009), (0x13045e47, 1));
        assert_eq!(
            div64_one(0xffffffffffffffff, 0xffffffff),
            (0x0000000100000001, 0)
        );
        assert_eq!(all_div64_pairs().count(), 144);
    }

    const MODULE_DESCRIPTION: &str = "64bit/32bit division and modulo test module";
}
