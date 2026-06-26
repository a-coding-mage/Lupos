//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests/int_sqrt_kunit.c
//! test-origin: linux:vendor/linux/lib/math/tests/int_sqrt_kunit.c
//! KUnit parameter coverage for int_sqrt().

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntSqrtCase {
    pub x: usize,
    pub expected_result: usize,
    pub name: &'static str,
}

pub const INT_SQRT_CASES: &[IntSqrtCase] = &[
    IntSqrtCase {
        x: 0,
        expected_result: 0,
        name: "edge case: square root of 0",
    },
    IntSqrtCase {
        x: 1,
        expected_result: 1,
        name: "perfect square: square root of 1",
    },
    IntSqrtCase {
        x: 2,
        expected_result: 1,
        name: "non-perfect square: square root of 2",
    },
    IntSqrtCase {
        x: 3,
        expected_result: 1,
        name: "non-perfect square: square root of 3",
    },
    IntSqrtCase {
        x: 4,
        expected_result: 2,
        name: "perfect square: square root of 4",
    },
    IntSqrtCase {
        x: 5,
        expected_result: 2,
        name: "non-perfect square: square root of 5",
    },
    IntSqrtCase {
        x: 6,
        expected_result: 2,
        name: "non-perfect square: square root of 6",
    },
    IntSqrtCase {
        x: 7,
        expected_result: 2,
        name: "non-perfect square: square root of 7",
    },
    IntSqrtCase {
        x: 8,
        expected_result: 2,
        name: "non-perfect square: square root of 8",
    },
    IntSqrtCase {
        x: 9,
        expected_result: 3,
        name: "perfect square: square root of 9",
    },
    IntSqrtCase {
        x: 15,
        expected_result: 3,
        name: "non-perfect square: square root of 15 (N-1 from 16)",
    },
    IntSqrtCase {
        x: 16,
        expected_result: 4,
        name: "perfect square: square root of 16",
    },
    IntSqrtCase {
        x: 17,
        expected_result: 4,
        name: "non-perfect square: square root of 17 (N+1 from 16)",
    },
    IntSqrtCase {
        x: 80,
        expected_result: 8,
        name: "non-perfect square: square root of 80 (N-1 from 81)",
    },
    IntSqrtCase {
        x: 81,
        expected_result: 9,
        name: "perfect square: square root of 81",
    },
    IntSqrtCase {
        x: 82,
        expected_result: 9,
        name: "non-perfect square: square root of 82 (N+1 from 81)",
    },
    IntSqrtCase {
        x: 255,
        expected_result: 15,
        name: "non-perfect square: square root of 255 (N-1 from 256)",
    },
    IntSqrtCase {
        x: 256,
        expected_result: 16,
        name: "perfect square: square root of 256",
    },
    IntSqrtCase {
        x: 257,
        expected_result: 16,
        name: "non-perfect square: square root of 257 (N+1 from 256)",
    },
    IntSqrtCase {
        x: 2_147_483_648,
        expected_result: 46_340,
        name: "large input: square root of 2147483648",
    },
    IntSqrtCase {
        x: 4_294_967_295,
        expected_result: 65_535,
        name: "edge case: ULONG_MAX for 32-bit",
    },
];

pub const fn int_sqrt(mut x: usize) -> usize {
    if x <= 1 {
        return x;
    }

    let mut bit = 1usize << (((usize::BITS as usize) - 2) & !1usize);
    while bit > x {
        bit >>= 2;
    }

    let mut result = 0usize;
    while bit != 0 {
        if x >= result + bit {
            x -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}

pub fn int_sqrt_kunit_results() -> impl Iterator<Item = (&'static str, bool)> {
    INT_SQRT_CASES
        .iter()
        .map(|case| (case.name, int_sqrt(case.x) == case.expected_result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_sqrt_kunit_params_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/int_sqrt_kunit.c"
        ));
        assert!(source.contains("{ 0, 0, \"edge case: square root of 0\" }"));
        assert!(source.contains("{ 1, 1, \"perfect square: square root of 1\" }"));
        assert!(source.contains("{ 2, 1, \"non-perfect square: square root of 2\" }"));
        assert!(source.contains("{ 16, 4, \"perfect square: square root of 16\" }"));
        assert!(source.contains("{ 2147483648, 46340"));
        assert!(source.contains("{ 4294967295, 65535"));
        assert!(source.contains("KUNIT_ARRAY_PARAM(int_sqrt, params, get_desc);"));
        assert!(source.contains("KUNIT_CASE_PARAM(int_sqrt_test, int_sqrt_gen_params)"));
        assert!(source.contains(".name = \"math-int_sqrt\""));

        assert_eq!(INT_SQRT_CASES.len(), 21);
        assert!(int_sqrt_kunit_results().all(|(_, passed)| passed));
    }
}
