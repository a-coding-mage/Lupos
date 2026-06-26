//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/err.c
//! test-origin: linux:vendor/linux/rust/helpers/err.c
//! Rust helper shims for Linux encoded error pointers.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/err.c",
        include_line: "#include <linux/err.h>",
        helper_symbol: "rust_helper_ERR_PTR",
        forwards_to: "ERR_PTR(err)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/err.c",
        include_line: "#include <linux/err.h>",
        helper_symbol: "rust_helper_IS_ERR",
        forwards_to: "IS_ERR(ptr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/err.c",
        include_line: "#include <linux/err.h>",
        helper_symbol: "rust_helper_PTR_ERR",
        forwards_to: "PTR_ERR(ptr)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/err.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
