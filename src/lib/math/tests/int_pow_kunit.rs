//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/int_pow_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/int_pow_kunit.c
//! KUnit parameter coverage for int_pow().

use crate::lib::math::int_pow::int_pow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntPowCase {
    pub base: u64,
    pub exponent: u32,
    pub expected_result: u64,
    pub name: &'static str,
}

pub const INT_POW_CASES: &[IntPowCase] = &[
    IntPowCase {
        base: 64,
        exponent: 0,
        expected_result: 1,
        name: "Power of zero",
    },
    IntPowCase {
        base: 64,
        exponent: 1,
        expected_result: 64,
        name: "Power of one",
    },
    IntPowCase {
        base: 0,
        exponent: 5,
        expected_result: 0,
        name: "Base zero",
    },
    IntPowCase {
        base: 1,
        exponent: 64,
        expected_result: 1,
        name: "Base one",
    },
    IntPowCase {
        base: 2,
        exponent: 2,
        expected_result: 4,
        name: "Two squared",
    },
    IntPowCase {
        base: 2,
        exponent: 3,
        expected_result: 8,
        name: "Two cubed",
    },
    IntPowCase {
        base: 5,
        exponent: 5,
        expected_result: 3125,
        name: "Five raised to the fifth power",
    },
    IntPowCase {
        base: u64::MAX,
        exponent: 1,
        expected_result: u64::MAX,
        name: "Max base",
    },
    IntPowCase {
        base: 2,
        exponent: 63,
        expected_result: 9_223_372_036_854_775_808,
        name: "Large result",
    },
];

pub fn int_pow_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    INT_POW_CASES.iter().map(|case| {
        (
            case.name,
            int_pow(case.base, case.exponent) == case.expected_result,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_pow_kunit_params_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/int_pow_kunit.c"
        ));
        assert!(source.contains("{ 64, 0, 1, \"Power of zero\" }"));
        assert!(source.contains("{ 64, 1, 64, \"Power of one\"}"));
        assert!(source.contains("{ 0, 5, 0, \"Base zero\" }"));
        assert!(source.contains("{ 1, 64, 1, \"Base one\" }"));
        assert!(source.contains("{ 2, 2, 4, \"Two squared\"}"));
        assert!(source.contains("{ 2, 3, 8, \"Two cubed\"}"));
        assert!(source.contains("{ 5, 5, 3125, \"Five raised to the fifth power\" }"));
        assert!(source.contains("{ U64_MAX, 1, U64_MAX, \"Max base\" }"));
        assert!(source.contains("{ 2, 63, 9223372036854775808ULL, \"Large result\"}"));
        assert!(source.contains("KUNIT_ARRAY_PARAM(int_pow, params, get_desc);"));
        assert!(source.contains("KUNIT_CASE_PARAM(int_pow_test, int_pow_gen_params)"));
        assert!(source.contains(".name = \"math-int_pow\""));

        assert_eq!(INT_POW_CASES.len(), 9);
        assert!(int_pow_kunit_results().all(|(_, passed)| passed));
    }
}
