//! linux-parity: complete
//! linux-source: vendor/linux/lib/atomic64_test.c
//! test-origin: linux:vendor/linux/lib/atomic64_test.c
//! Atomic and atomic64 test-module operation semantics.

pub const V0: i64 = 0xaaa31337c001d00d_u64 as i64;
pub const V1: i64 = 0xdeadbeefdeafcafe_u64 as i64;
pub const V2: i64 = 0xfaceabadf00df001_u64 as i64;
pub const V3: i64 = 0x8000000000000000_u64 as i64;
pub const ONES_TWOS: i64 = 0x1111111122222222;
pub const MODULE_DESCRIPTION: &str = "Testsuite for atomic64_t functions";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicOp {
    Add,
    Sub,
    Or,
    And,
    Xor,
    AndNot,
    Inc,
    Dec,
    Xchg,
    CmpXchg,
    AddUnless,
    DecIfPositive,
    IncNotZero,
}

pub const ATOMIC64_OPS: [AtomicOp; 13] = [
    AtomicOp::Add,
    AtomicOp::Sub,
    AtomicOp::Or,
    AtomicOp::And,
    AtomicOp::Xor,
    AtomicOp::AndNot,
    AtomicOp::Inc,
    AtomicOp::Dec,
    AtomicOp::Xchg,
    AtomicOp::CmpXchg,
    AtomicOp::AddUnless,
    AtomicOp::DecIfPositive,
    AtomicOp::IncNotZero,
];

pub const fn add_unless(value: i64, add: i64, unless: i64) -> (bool, i64) {
    if value == unless {
        (false, value)
    } else {
        (true, value.wrapping_add(add))
    }
}

pub const fn dec_if_positive(value: i64) -> (i64, i64) {
    let next = value.wrapping_sub(1);
    if next >= 0 {
        (next, next)
    } else {
        (next, value)
    }
}

pub const fn inc_not_zero(value: i64) -> (bool, i64) {
    if value == 0 {
        (false, value)
    } else {
        (true, value.wrapping_add(1))
    }
}

pub const fn cmpxchg(value: i64, old: i64, new: i64) -> (i64, i64) {
    if value == old {
        (value, new)
    } else {
        (value, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic64_test_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/atomic64_test.c"
        ));

        assert!(source.contains("Testsuite for atomic64_t functions"));
        assert!(source.contains("#define FAMILY_TEST(test, bit, op, args...)"));
        assert!(source.contains("static __init void test_atomic64(void)"));
        for token in [
            "TEST(64, add, +=, onestwos)",
            "TEST(64, sub, -=, onestwos)",
            "TEST(64, or, |=, v1)",
            "TEST(64, and, &=, v1)",
            "TEST(64, xor, ^=, v1)",
            "TEST(64, andnot, &= ~, v1)",
            "RETURN_FAMILY_TEST(64, add_return, +=, onestwos)",
            "FETCH_FAMILY_TEST(64, fetch_xor, ^=, v1)",
            "XCHG_FAMILY_TEST(64, v0, v1)",
            "CMPXCHG_FAMILY_TEST(64, v0, v1, v2)",
            "atomic64_add_unless(&v, one, v0)",
            "atomic64_dec_if_positive(&v)",
            "atomic64_inc_not_zero(&v)",
        ] {
            assert!(source.contains(token));
        }
        assert!(source.contains("module_init(test_atomics_init);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert_eq!(ATOMIC64_OPS.len(), 13);

        assert_eq!(add_unless(V0, 1, V0), (false, V0));
        assert_eq!(add_unless(V0, 1, V1), (true, V0.wrapping_add(1)));
        assert_eq!(dec_if_positive(ONES_TWOS), (ONES_TWOS - 1, ONES_TWOS - 1));
        assert_eq!(dec_if_positive(0), (-1, 0));
        assert_eq!(dec_if_positive(-1), (-2, -1));
        assert_eq!(inc_not_zero(ONES_TWOS), (true, ONES_TWOS + 1));
        assert_eq!(inc_not_zero(0), (false, 0));
        assert_eq!(cmpxchg(V0, V0, V1), (V0, V1));
        assert_eq!(cmpxchg(V0, V2, V1), (V0, V0));
    }
}
