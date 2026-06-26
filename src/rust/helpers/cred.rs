//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/cred.c
//! test-origin: linux:vendor/linux/rust/helpers/cred.c
//! Rust helper shims for Linux credential references.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cred.c",
        include_line: "#include <linux/cred.h>",
        helper_symbol: "rust_helper_get_cred",
        forwards_to: "get_cred(cred)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cred.c",
        include_line: "#include <linux/cred.h>",
        helper_symbol: "rust_helper_put_cred",
        forwards_to: "put_cred(cred)",
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
            "/vendor/linux/rust/helpers/cred.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
