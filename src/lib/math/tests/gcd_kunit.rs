//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/gcd_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/gcd_kunit.c
//! KUnit parameter coverage for gcd().

use crate::lib::math::gcd::gcd;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GcdCase {
    pub val1: usize,
    pub val2: usize,
    pub expected_result: usize,
    pub name: &'static str,
}

pub const GCD_CASES: &[GcdCase] = &[
    GcdCase {
        val1: 48,
        val2: 18,
        expected_result: 6,
        name: "GCD of 48 and 18",
    },
    GcdCase {
        val1: 18,
        val2: 48,
        expected_result: 6,
        name: "GCD of 18 and 48",
    },
    GcdCase {
        val1: 56,
        val2: 98,
        expected_result: 14,
        name: "GCD of 56 and 98",
    },
    GcdCase {
        val1: 17,
        val2: 13,
        expected_result: 1,
        name: "Coprime numbers",
    },
    GcdCase {
        val1: 101,
        val2: 103,
        expected_result: 1,
        name: "Coprime numbers",
    },
    GcdCase {
        val1: 270,
        val2: 192,
        expected_result: 6,
        name: "GCD of 270 and 192",
    },
    GcdCase {
        val1: 0,
        val2: 5,
        expected_result: 5,
        name: "GCD with zero",
    },
    GcdCase {
        val1: 7,
        val2: 0,
        expected_result: 7,
        name: "GCD with zero reversed",
    },
    GcdCase {
        val1: 36,
        val2: 36,
        expected_result: 36,
        name: "GCD of identical numbers",
    },
    GcdCase {
        val1: usize::MAX,
        val2: 1,
        expected_result: 1,
        name: "GCD of max ulong and 1",
    },
    GcdCase {
        val1: usize::MAX,
        val2: usize::MAX,
        expected_result: usize::MAX,
        name: "GCD of max ulong values",
    },
];

pub fn gcd_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    GCD_CASES
        .iter()
        .map(|case| (case.name, gcd(case.val1, case.val2) == case.expected_result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_kunit_params_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/gcd_kunit.c"
        ));
        assert!(source.contains("{ 48, 18, 6, \"GCD of 48 and 18\" }"));
        assert!(source.contains("{ 18, 48, 6, \"GCD of 18 and 48\" }"));
        assert!(source.contains("{ 56, 98, 14, \"GCD of 56 and 98\" }"));
        assert!(source.contains("{ 17, 13, 1, \"Coprime numbers\" }"));
        assert!(source.contains("{ 101, 103, 1, \"Coprime numbers\" }"));
        assert!(source.contains("{ 270, 192, 6, \"GCD of 270 and 192\" }"));
        assert!(source.contains("{ 0, 5, 5, \"GCD with zero\" }"));
        assert!(source.contains("{ 7, 0, 7, \"GCD with zero reversed\" }"));
        assert!(source.contains("{ 36, 36, 36, \"GCD of identical numbers\" }"));
        assert!(source.contains("{ ULONG_MAX, 1, 1, \"GCD of max ulong and 1\" }"));
        assert!(source.contains("{ ULONG_MAX, ULONG_MAX, ULONG_MAX"));
        assert!(source.contains("KUNIT_ARRAY_PARAM(gcd, params, get_desc);"));
        assert!(source.contains("KUNIT_CASE_PARAM(gcd_test, gcd_gen_params)"));
        assert!(source.contains(".name = \"math-gcd\""));

        assert_eq!(GCD_CASES.len(), 11);
        assert!(gcd_kunit_results().all(|(_, passed)| passed));
    }
}
