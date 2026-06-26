//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/xarray.c
//! test-origin: linux:vendor/linux/rust/helpers/xarray.c
//! Rust helper shims for xarray operations.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/xarray.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/xarray.h>",
        helper_symbol: "rust_helper_xa_err",
        forwards_to: "xa_err(entry)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/xarray.h>",
        helper_symbol: "rust_helper_xa_init_flags",
        forwards_to: "xa_init_flags(xa, flags)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/xarray.h>",
        helper_symbol: "rust_helper_xa_trylock",
        forwards_to: "xa_trylock(xa)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/xarray.h>",
        helper_symbol: "rust_helper_xa_lock",
        forwards_to: "xa_lock(xa)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/xarray.h>",
        helper_symbol: "rust_helper_xa_unlock",
        forwards_to: "xa_unlock(xa)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_xarray_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/xarray.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
