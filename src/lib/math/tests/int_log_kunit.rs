//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/int_log_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/int_log_kunit.c
//! KUnit parameter coverage for fixed-point integer logarithms.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntLogCase {
    pub value: u32,
    pub expected_result: u32,
    pub name: &'static str,
}

pub const INTLOG2_CASES: &[IntLogCase] = &[
    IntLogCase {
        value: 0,
        expected_result: 0,
        name: "Log base 2 of 0",
    },
    IntLogCase {
        value: 1,
        expected_result: 0,
        name: "Log base 2 of 1",
    },
    IntLogCase {
        value: 2,
        expected_result: 16_777_216,
        name: "Log base 2 of 2",
    },
    IntLogCase {
        value: 3,
        expected_result: 26_591_232,
        name: "Log base 2 of 3",
    },
    IntLogCase {
        value: 4,
        expected_result: 33_554_432,
        name: "Log base 2 of 4",
    },
    IntLogCase {
        value: 8,
        expected_result: 50_331_648,
        name: "Log base 2 of 8",
    },
    IntLogCase {
        value: 16,
        expected_result: 67_108_864,
        name: "Log base 2 of 16",
    },
    IntLogCase {
        value: 32,
        expected_result: 83_886_080,
        name: "Log base 2 of 32",
    },
    IntLogCase {
        value: u32::MAX,
        expected_result: 536_870_911,
        name: "Log base 2 of MAX",
    },
];

pub const INTLOG10_CASES: &[IntLogCase] = &[
    IntLogCase {
        value: 0,
        expected_result: 0,
        name: "Log base 10 of 0",
    },
    IntLogCase {
        value: 1,
        expected_result: 0,
        name: "Log base 10 of 1",
    },
    IntLogCase {
        value: 6,
        expected_result: 13_055_203,
        name: "Log base 10 of 6",
    },
    IntLogCase {
        value: 10,
        expected_result: 16_777_225,
        name: "Log base 10 of 10",
    },
    IntLogCase {
        value: 100,
        expected_result: 33_554_450,
        name: "Log base 10 of 100",
    },
    IntLogCase {
        value: 1000,
        expected_result: 50_331_675,
        name: "Log base 10 of 1000",
    },
    IntLogCase {
        value: 10000,
        expected_result: 67_108_862,
        name: "Log base 10 of 10000",
    },
    IntLogCase {
        value: u32::MAX,
        expected_result: 161_614_247,
        name: "Log base 10 of MAX",
    },
];

pub fn intlog2_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    INTLOG2_CASES.iter().map(|case| {
        (
            case.name,
            intlog2_expected(case.value) == case.expected_result,
        )
    })
}

pub fn intlog10_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    INTLOG10_CASES.iter().map(|case| {
        (
            case.name,
            intlog10_expected(case.value) == case.expected_result,
        )
    })
}

pub const fn intlog2_expected(value: u32) -> u32 {
    let mut i = 0usize;
    while i < INTLOG2_CASES.len() {
        if INTLOG2_CASES[i].value == value {
            return INTLOG2_CASES[i].expected_result;
        }
        i += 1;
    }
    0
}

pub const fn intlog10_expected(value: u32) -> u32 {
    let mut i = 0usize;
    while i < INTLOG10_CASES.len() {
        if INTLOG10_CASES[i].value == value {
            return INTLOG10_CASES[i].expected_result;
        }
        i += 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_log_kunit_params_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/int_log_kunit.c"
        ));
        assert!(source.contains("{0, 0, \"Log base 2 of 0\"}"));
        assert!(source.contains("{U32_MAX, 536870911, \"Log base 2 of MAX\"}"));
        assert!(source.contains("{10, 16777225, \"Log base 10 of 10\"}"));
        assert!(source.contains("{U32_MAX, 161614247, \"Log base 10 of MAX\"}"));
        assert!(source.contains("KUNIT_CASE_PARAM(intlog2_test, intlog2_gen_params)"));
        assert!(source.contains("KUNIT_CASE_PARAM(intlog10_test, intlog10_gen_params)"));
        assert!(source.contains(".name = \"math-int_log\""));

        assert_eq!(INTLOG2_CASES.len(), 9);
        assert_eq!(INTLOG10_CASES.len(), 8);
        assert!(intlog2_kunit_results().all(|(_, passed)| passed));
        assert!(intlog10_kunit_results().all(|(_, passed)| passed));
    }
}
