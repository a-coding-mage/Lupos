//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/rational_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/rational_kunit.c
//! KUnit parameter coverage for rational_best_approximation().

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RationalCase {
    pub num: usize,
    pub den: usize,
    pub max_num: usize,
    pub max_den: usize,
    pub exp_num: usize,
    pub exp_den: usize,
    pub name: &'static str,
}

pub const RATIONAL_CASES: &[RationalCase] = &[
    RationalCase {
        num: 1230,
        den: 10,
        max_num: 100,
        max_den: 20,
        exp_num: 100,
        exp_den: 1,
        name: "Exceeds bounds, semi-convergent term > 1/2 last term",
    },
    RationalCase {
        num: 34567,
        den: 100,
        max_num: 120,
        max_den: 20,
        exp_num: 120,
        exp_den: 1,
        name: "Exceeds bounds, semi-convergent term < 1/2 last term",
    },
    RationalCase {
        num: 1,
        den: 30,
        max_num: 100,
        max_den: 10,
        exp_num: 0,
        exp_den: 1,
        name: "Closest to zero",
    },
    RationalCase {
        num: 1,
        den: 19,
        max_num: 100,
        max_den: 10,
        exp_num: 1,
        exp_den: 10,
        name: "Closest to smallest non-zero",
    },
    RationalCase {
        num: 27,
        den: 32,
        max_num: 16,
        max_den: 16,
        exp_num: 11,
        exp_den: 13,
        name: "Use convergent",
    },
    RationalCase {
        num: 1155,
        den: 7735,
        max_num: 255,
        max_den: 255,
        exp_num: 33,
        exp_den: 221,
        name: "Exact answer",
    },
    RationalCase {
        num: 87,
        den: 32,
        max_num: 70,
        max_den: 32,
        exp_num: 68,
        exp_den: 25,
        name: "Semiconvergent, numerator limit",
    },
    RationalCase {
        num: 14533,
        den: 4626,
        max_num: 15000,
        max_den: 2400,
        exp_num: 7433,
        exp_den: 2366,
        name: "Semiconvergent, denominator limit",
    },
];

pub fn rational_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    RATIONAL_CASES.iter().map(|case| {
        (
            case.name,
            rational_best_approximation(case.num, case.den, case.max_num, case.max_den)
                == (case.exp_num, case.exp_den),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_kunit_params_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/rational_kunit.c"
        ));
        assert!(source.contains("#include <linux/rational.h>"));
        assert!(source.contains("{ 1230,\t10,\t100, 20,\t100, 1"));
        assert!(source.contains("{ 34567,100, \t120, 20,\t120, 1"));
        assert!(source.contains("{ 1, 30,\t100, 10,\t0, 1"));
        assert!(source.contains("{ 1, 19,\t100, 10,\t1, 10"));
        assert!(source.contains("{ 27,32,\t16, 16,\t\t11, 13"));
        assert!(source.contains("{ 1155, 7735,\t255, 255,\t33, 221"));
        assert!(source.contains("{ 87, 32,\t70, 32,\t\t68, 25"));
        assert!(source.contains("{ 14533, 4626,\t15000, 2400,\t7433, 2366"));
        assert!(source.contains("rational_best_approximation("));
        assert!(source.contains("KUNIT_ARRAY_PARAM(rational, test_parameters, get_desc);"));
        assert!(source.contains(".name = \"rational\""));

        assert_eq!(RATIONAL_CASES.len(), 8);
        assert!(rational_kunit_results().all(|(_, passed)| passed));
    }
}
use crate::lib::math::rational::rational_best_approximation;
