//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/uaccess.c
//! test-origin: linux:vendor/linux/rust/helpers/uaccess.c
//! Rust helper shims for user-copy accessors.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/uaccess.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/uaccess.h>",
        helper_symbol: "rust_helper_copy_from_user",
        forwards_to: "copy_from_user(to, from, n)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/uaccess.h>",
        helper_symbol: "rust_helper_copy_to_user",
        forwards_to: "copy_to_user(to, from, n)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/uaccess.h>",
        helper_symbol: "rust_helper__copy_from_user",
        forwards_to: "_inline_copy_from_user(to, from, n)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/uaccess.h>",
        helper_symbol: "rust_helper__copy_to_user",
        forwards_to: "_inline_copy_to_user(to, from, n)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_uaccess_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/uaccess.c"
        ));
        assert!(source.contains("#ifdef INLINE_COPY_USER"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
