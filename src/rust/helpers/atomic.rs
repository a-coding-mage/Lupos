//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/atomic.c
//! test-origin: linux:vendor/linux/rust/helpers/atomic.c
//! Generated Rust helper shim inventory for Linux atomic operations.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicFamily {
    pub helper_prefix: &'static str,
    pub intrinsic_prefix: &'static str,
    pub value_type: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicOperation {
    pub suffix: &'static str,
    pub args: &'static str,
}

pub const FAMILIES: &[AtomicFamily] = &[
    AtomicFamily {
        helper_prefix: "atomic",
        intrinsic_prefix: "atomic",
        value_type: "atomic_t",
    },
    AtomicFamily {
        helper_prefix: "atomic64",
        intrinsic_prefix: "atomic64",
        value_type: "atomic64_t",
    },
];

pub const OPERATIONS: &[AtomicOperation] = &[
    AtomicOperation {
        suffix: "read",
        args: "v",
    },
    AtomicOperation {
        suffix: "read_acquire",
        args: "v",
    },
    AtomicOperation {
        suffix: "set",
        args: "v, i",
    },
    AtomicOperation {
        suffix: "set_release",
        args: "v, i",
    },
    AtomicOperation {
        suffix: "add",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_return",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_return_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_return_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_return_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_add",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_add_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_add_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_add_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "sub",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "sub_return",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "sub_return_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "sub_return_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "sub_return_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_sub",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_sub_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_sub_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_sub_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "inc",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_return",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_return_acquire",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_return_release",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_return_relaxed",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_inc",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_inc_acquire",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_inc_release",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_inc_relaxed",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_return",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_return_acquire",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_return_release",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_return_relaxed",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_dec",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_dec_acquire",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_dec_release",
        args: "v",
    },
    AtomicOperation {
        suffix: "fetch_dec_relaxed",
        args: "v",
    },
    AtomicOperation {
        suffix: "and",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_and",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_and_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_and_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_and_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "andnot",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_andnot",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_andnot_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_andnot_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_andnot_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "or",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_or",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_or_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_or_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_or_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "xor",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_xor",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_xor_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_xor_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_xor_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "xchg",
        args: "v, new",
    },
    AtomicOperation {
        suffix: "xchg_acquire",
        args: "v, new",
    },
    AtomicOperation {
        suffix: "xchg_release",
        args: "v, new",
    },
    AtomicOperation {
        suffix: "xchg_relaxed",
        args: "v, new",
    },
    AtomicOperation {
        suffix: "cmpxchg",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "cmpxchg_acquire",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "cmpxchg_release",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "cmpxchg_relaxed",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "try_cmpxchg",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "try_cmpxchg_acquire",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "try_cmpxchg_release",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "try_cmpxchg_relaxed",
        args: "v, old, new",
    },
    AtomicOperation {
        suffix: "sub_and_test",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "dec_and_test",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_and_test",
        args: "v",
    },
    AtomicOperation {
        suffix: "add_negative",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_negative_acquire",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_negative_release",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "add_negative_relaxed",
        args: "i, v",
    },
    AtomicOperation {
        suffix: "fetch_add_unless",
        args: "v, a, u",
    },
    AtomicOperation {
        suffix: "add_unless",
        args: "v, a, u",
    },
    AtomicOperation {
        suffix: "inc_not_zero",
        args: "v",
    },
    AtomicOperation {
        suffix: "inc_unless_negative",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_unless_positive",
        args: "v",
    },
    AtomicOperation {
        suffix: "dec_if_positive",
        args: "v",
    },
];

pub const EXPECTED_HELPER_COUNT: usize = 170;

pub fn families() -> &'static [AtomicFamily] {
    FAMILIES
}

pub fn operations() -> &'static [AtomicOperation] {
    OPERATIONS
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn generated_helper_inventory_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/atomic.c"
        ));

        assert_eq!(
            source.lines().next(),
            Some("// SPDX-License-Identifier: GPL-2.0")
        );
        assert!(source.contains("Generated by scripts/atomic/gen-rust-atomic-helpers.sh"));
        assert!(source.contains("#include <linux/atomic.h>"));
        assert_eq!(
            source.matches("__rust_helper").count(),
            EXPECTED_HELPER_COUNT
        );
        assert_eq!(FAMILIES.len() * OPERATIONS.len(), EXPECTED_HELPER_COUNT);

        for family in FAMILIES {
            for operation in OPERATIONS {
                let helper_symbol =
                    format!("rust_helper_{}_{}", family.helper_prefix, operation.suffix);
                let forwards_to = format!(
                    "{}_{}({})",
                    family.intrinsic_prefix, operation.suffix, operation.args
                );

                assert!(
                    source.contains(&helper_symbol),
                    "vendor/linux/rust/helpers/atomic.c missing {}",
                    helper_symbol
                );
                assert!(
                    source.contains(&forwards_to),
                    "vendor/linux/rust/helpers/atomic.c missing {}",
                    forwards_to
                );
            }
        }
    }
}
