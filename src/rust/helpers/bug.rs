//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/bug.c
//! test-origin: linux:vendor/linux/rust/helpers/bug.c
//! Rust helper shims for Linux BUG/WARN helpers.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bug.c",
        include_line: "#include <linux/bug.h>",
        helper_symbol: "rust_helper_BUG",
        forwards_to: "BUG()",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bug.c",
        include_line: "#include <linux/bug.h>",
        helper_symbol: "rust_helper_WARN_ON",
        forwards_to: "WARN_ON(cond)",
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
            "/vendor/linux/rust/helpers/bug.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
