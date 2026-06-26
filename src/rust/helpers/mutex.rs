//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/mutex.c
//! test-origin: linux:vendor/linux/rust/helpers/mutex.c
//! Rust helper shims for mutex operations.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/mutex.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mutex.h>",
        helper_symbol: "rust_helper_mutex_lock",
        forwards_to: "mutex_lock(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mutex.h>",
        helper_symbol: "rust_helper_mutex_trylock",
        forwards_to: "mutex_trylock(lock)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mutex.h>",
        helper_symbol: "rust_helper___mutex_init",
        forwards_to: "__mutex_init(mutex, name, key)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mutex.h>",
        helper_symbol: "rust_helper_mutex_assert_is_held",
        forwards_to: "lockdep_assert_held(mutex)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mutex.h>",
        helper_symbol: "rust_helper_mutex_destroy",
        forwards_to: "mutex_destroy(lock)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_mutex_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/mutex.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
