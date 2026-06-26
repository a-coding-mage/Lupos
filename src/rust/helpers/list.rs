//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/list.c
//! test-origin: linux:vendor/linux/rust/helpers/list.c
//! Rust helper shims for Linux list-head operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/list.c",
        include_line: "#include <linux/list.h>",
        helper_symbol: "rust_helper_INIT_LIST_HEAD",
        forwards_to: "INIT_LIST_HEAD(list)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/list.c",
        include_line: "#include <linux/list.h>",
        helper_symbol: "rust_helper_list_add_tail",
        forwards_to: "list_add_tail(new, head)",
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
            "/vendor/linux/rust/helpers/list.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
