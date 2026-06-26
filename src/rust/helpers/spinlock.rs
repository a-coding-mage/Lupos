//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/spinlock.c
//! test-origin: linux:vendor/linux/rust/helpers/spinlock.c
//! Rust helper shims for spinlocks.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/spinlock.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/spinlock.h>",
        helper_symbol: "rust_helper___spin_lock_init",
        forwards_to: "spin_lock_init(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/spinlock.h>",
        helper_symbol: "rust_helper_spin_lock",
        forwards_to: "spin_lock(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/spinlock.h>",
        helper_symbol: "rust_helper_spin_unlock",
        forwards_to: "spin_unlock(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/spinlock.h>",
        helper_symbol: "rust_helper_spin_trylock",
        forwards_to: "spin_trylock(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/spinlock.h>",
        helper_symbol: "rust_helper_spin_assert_is_held",
        forwards_to: "lockdep_assert_held(lock)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_spinlock_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/spinlock.c"
        ));
        assert!(source.contains("#ifdef CONFIG_DEBUG_SPINLOCK"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
