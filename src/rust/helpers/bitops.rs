//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/bitops.c
//! test-origin: linux:vendor/linux/rust/helpers/bitops.c
//! Rust helper shims for bit mutation and bit search operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/bitops.h>",
        helper_symbol: "rust_helper___set_bit",
        forwards_to: "__set_bit(nr, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/bitops.h>",
        helper_symbol: "rust_helper___clear_bit",
        forwards_to: "__clear_bit(nr, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/bitops.h>",
        helper_symbol: "rust_helper_set_bit",
        forwards_to: "set_bit(nr, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/bitops.h>",
        helper_symbol: "rust_helper_clear_bit",
        forwards_to: "clear_bit(nr, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/find.h>",
        helper_symbol: "_find_first_zero_bit",
        forwards_to: "find_first_zero_bit(p, size)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/find.h>",
        helper_symbol: "_find_next_zero_bit",
        forwards_to: "find_next_zero_bit(addr, size, offset)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/find.h>",
        helper_symbol: "_find_first_bit",
        forwards_to: "find_first_bit(addr, size)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/bitops.c",
        include_line: "#include <linux/find.h>",
        helper_symbol: "_find_next_bit",
        forwards_to: "find_next_bit(addr, size, offset)",
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
            "/vendor/linux/rust/helpers/bitops.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
