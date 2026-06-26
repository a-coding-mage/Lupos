//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/test_mul_u64_u64_div_u64.c
//! test-origin: linux:vendor/linux/lib/math/test_mul_u64_u64_div_u64.c
//! mul_u64_u64_div_u64 Linux test-module vector table.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestParams {
    pub a: u64,
    pub b: u64,
    pub d: u64,
    pub result: u64,
    pub round_up: bool,
}

pub const TEST_VALUES: [TestParams; 28] = [
    TestParams {
        a: 0xb,
        b: 0x7,
        d: 0x3,
        result: 0x19,
        round_up: true,
    },
    TestParams {
        a: 0xffff0000,
        b: 0xffff0000,
        d: 0xf,
        result: 0x1110eeef00000000,
        round_up: false,
    },
    TestParams {
        a: 0xffffffff,
        b: 0xffffffff,
        d: 0x1,
        result: 0xfffffffe00000001,
        round_up: false,
    },
    TestParams {
        a: 0xffffffff,
        b: 0xffffffff,
        d: 0x2,
        result: 0x7fffffff00000000,
        round_up: true,
    },
    TestParams {
        a: 0x1ffffffff,
        b: 0xffffffff,
        d: 0x2,
        result: 0xfffffffe80000000,
        round_up: true,
    },
    TestParams {
        a: 0x1ffffffff,
        b: 0xffffffff,
        d: 0x3,
        result: 0xaaaaaaa9aaaaaaab,
        round_up: false,
    },
    TestParams {
        a: 0x1ffffffff,
        b: 0x1ffffffff,
        d: 0x4,
        result: 0xffffffff00000000,
        round_up: true,
    },
    TestParams {
        a: 0xffff000000000000,
        b: 0xffff000000000000,
        d: 0xffff000000000001,
        result: 0xfffeffffffffffff,
        round_up: true,
    },
    TestParams {
        a: 0x3333333333333333,
        b: 0x3333333333333333,
        d: 0x5555555555555555,
        result: 0x1eb851eb851eb851,
        round_up: true,
    },
    TestParams {
        a: 0x7fffffffffffffff,
        b: 0x2,
        d: 0x3,
        result: 0x5555555555555554,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x2,
        d: 0x8000000000000000,
        result: 0x3,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x2,
        d: 0xc000000000000000,
        result: 0x2,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x4000000000000004,
        d: 0x8000000000000000,
        result: 0x8000000000000007,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x4000000000000001,
        d: 0x8000000000000000,
        result: 0x8000000000000001,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x8000000000000001,
        d: 0xffffffffffffffff,
        result: 0x8000000000000001,
        round_up: false,
    },
    TestParams {
        a: 0xfffffffffffffffe,
        b: 0x8000000000000001,
        d: 0xffffffffffffffff,
        result: 0x8000000000000000,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x8000000000000001,
        d: 0xfffffffffffffffe,
        result: 0x8000000000000001,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x8000000000000001,
        d: 0xfffffffffffffffd,
        result: 0x8000000000000002,
        round_up: true,
    },
    TestParams {
        a: 0x7fffffffffffffff,
        b: 0xffffffffffffffff,
        d: 0xc000000000000000,
        result: 0xaaaaaaaaaaaaaaa8,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x7fffffffffffffff,
        d: 0xa000000000000000,
        result: 0xccccccccccccccca,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0x7fffffffffffffff,
        d: 0x9000000000000000,
        result: 0xe38e38e38e38e38b,
        round_up: true,
    },
    TestParams {
        a: 0x7fffffffffffffff,
        b: 0x7fffffffffffffff,
        d: 0x5000000000000000,
        result: 0xccccccccccccccc9,
        round_up: true,
    },
    TestParams {
        a: 0xffffffffffffffff,
        b: 0xfffffffffffffffe,
        d: 0xffffffffffffffff,
        result: 0xfffffffffffffffe,
        round_up: false,
    },
    TestParams {
        a: 0xe6102d256d7ea3ae,
        b: 0x70a77d0be4c31201,
        d: 0xd63ec35ab3220357,
        result: 0x78f8bf8cc86c6e18,
        round_up: true,
    },
    TestParams {
        a: 0xf53bae05cb86c6e1,
        b: 0x3847b32d2f8d32e0,
        d: 0xcfd4f55a647f403c,
        result: 0x42687f79d8998d35,
        round_up: true,
    },
    TestParams {
        a: 0x9951c5498f941092,
        b: 0x1f8c8bfdf287a251,
        d: 0xa3c8dc5f81ea3fe2,
        result: 0x1d887cb25900091f,
        round_up: true,
    },
    TestParams {
        a: 0x374fee9daa1bb2bb,
        b: 0x0d0bfbff7b8ae3ef,
        d: 0xc169337bd42d5179,
        result: 0x03bb2dbaffcbb961,
        round_up: true,
    },
    TestParams {
        a: 0xeac0d03ac10eeaf0,
        b: 0x89be05dfa162ed9b,
        d: 0x92bb1679a41f0e4b,
        result: 0xdc5f5cc9e270d216,
        round_up: true,
    },
];

pub const fn mul_u64_u64_div_u64(a: u64, b: u64, d: u64) -> u64 {
    ((a as u128 * b as u128) / d as u128) as u64
}

pub const fn mul_u64_u64_div_u64_roundup(a: u64, b: u64, d: u64) -> u64 {
    ((a as u128 * b as u128 + (d - 1) as u128) / d as u128) as u64
}

pub fn test_vector_failures() -> usize {
    TEST_VALUES
        .iter()
        .filter(|params| {
            let rounded = params.result + u64::from(params.round_up);
            mul_u64_u64_div_u64(params.a, params.b, params.d) != params.result
                || mul_u64_u64_div_u64_roundup(params.a, params.b, params.d) != rounded
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_u64_u64_div_u64_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/test_mul_u64_u64_div_u64.c"
        ));
        let div64_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/div64.c"
        ));
        let math64_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/math64.h"
        ));

        assert!(source.contains("static test_params test_values[]"));
        assert!(source.contains("mul_u64_u64_div_u64(a, b, d);"));
        assert!(source.contains("mul_u64_u64_div_u64_roundup(a, b, d);"));
        assert!(source.contains("#include \"div64.c\""));
        assert!(source.contains("module_init(test_init);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"mul_u64_u64_div_u64() test module\")"));
        assert!(div64_source.contains("u64 mul_u64_add_u64_div_u64(u64 a, u64 b, u64 c, u64 d)"));
        assert!(math64_source.contains("#define mul_u64_u64_div_u64(a, b, d)"));
        assert!(math64_source.contains("#define mul_u64_u64_div_u64_roundup(a, b, d)"));
        assert_eq!(TEST_VALUES.len(), 28);
        assert_eq!(test_vector_failures(), 0);
    }
}
