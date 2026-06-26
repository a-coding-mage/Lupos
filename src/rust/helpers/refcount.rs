//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/refcount.c
//! test-origin: linux:vendor/linux/rust/helpers/refcount.c
//! Rust helper shims for refcount_t operations.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/refcount.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/refcount.h>",
        helper_symbol: "rust_helper_REFCOUNT_INIT",
        forwards_to: "REFCOUNT_INIT(n)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/refcount.h>",
        helper_symbol: "rust_helper_refcount_set",
        forwards_to: "refcount_set(r, n)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/refcount.h>",
        helper_symbol: "rust_helper_refcount_inc",
        forwards_to: "refcount_inc(r)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/refcount.h>",
        helper_symbol: "rust_helper_refcount_dec",
        forwards_to: "refcount_dec(r)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/refcount.h>",
        helper_symbol: "rust_helper_refcount_dec_and_test",
        forwards_to: "refcount_dec_and_test(r)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_refcount_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/refcount.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
