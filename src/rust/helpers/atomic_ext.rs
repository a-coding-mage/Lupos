//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/atomic_ext.c
//! test-origin: linux:vendor/linux/rust/helpers/atomic_ext.c
//! Macro-generated Rust helper shim inventory for narrow and pointer atomics.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicExtFamily {
    pub type_name: &'static str,
    pub c_type: &'static str,
}

pub const FAMILIES: &[AtomicExtFamily] = &[
    AtomicExtFamily {
        type_name: "i8",
        c_type: "s8",
    },
    AtomicExtFamily {
        type_name: "i16",
        c_type: "s16",
    },
    AtomicExtFamily {
        type_name: "ptr",
        c_type: "const void *",
    },
];

pub const READ_SET_FORWARDS: &[&str] = &[
    "READ_ONCE(*ptr)",
    "WRITE_ONCE(*ptr, val)",
    "smp_load_acquire(ptr)",
    "smp_store_release(ptr, val)",
];

pub const ORDERING_SUFFIXES: &[&str] = &["", "_acquire", "_release", "_relaxed"];

pub const EXPECTED_HELPER_COUNT: usize = 36;

pub fn families() -> &'static [AtomicExtFamily] {
    FAMILIES
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn macro_generated_helper_inventory_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/atomic_ext.c"
        ));

        assert_eq!(
            source.lines().next(),
            Some("// SPDX-License-Identifier: GPL-2.0")
        );
        assert!(source.contains("#include <asm/barrier.h>"));
        assert!(source.contains("#include <asm/rwonce.h>"));
        assert!(source.contains("#include <linux/atomic.h>"));

        for forward in READ_SET_FORWARDS {
            assert!(
                source.contains(forward),
                "vendor/linux/rust/helpers/atomic_ext.c missing {}",
                forward
            );
        }

        assert!(source.contains("rust_helper_atomic_##tname##_read"));
        assert!(source.contains("rust_helper_atomic_##tname##_set"));
        assert!(source.contains("rust_helper_atomic_##tname##_read_acquire"));
        assert!(source.contains("rust_helper_atomic_##tname##_set_release"));
        assert!(source.contains("rust_helper_atomic_##tname##_xchg##suffix"));
        assert!(source.contains("rust_helper_atomic_##tname##_try_cmpxchg##suffix"));
        assert!(source.contains("xchg##suffix(ptr, new)"));
        assert!(source.contains("try_cmpxchg##suffix(ptr, old, new)"));

        for family in FAMILIES {
            assert!(source.contains(&format!(
                "GEN_READ_SET_HELPERS({}, {})",
                family.type_name, family.c_type
            )));
            assert!(source.contains(&format!(
                "GEN_XCHG_HELPERS({}, {})",
                family.type_name, family.c_type
            )));
            assert!(source.contains(&format!(
                "GEN_TRY_CMPXCHG_HELPERS({}, {})",
                family.type_name, family.c_type
            )));
        }

        assert_eq!(
            FAMILIES.len() * (READ_SET_FORWARDS.len() + ORDERING_SUFFIXES.len() * 2),
            EXPECTED_HELPER_COUNT
        );
    }
}
