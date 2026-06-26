//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/rbtree.c
//! test-origin: linux:vendor/linux/rust/helpers/rbtree.c
//! Rust helper shims for red-black tree operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/rbtree.c",
        include_line: "#include <linux/rbtree.h>",
        helper_symbol: "rust_helper_rb_link_node",
        forwards_to: "rb_link_node(node, parent, rb_link)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/rbtree.c",
        include_line: "#include <linux/rbtree.h>",
        helper_symbol: "rust_helper_rb_first",
        forwards_to: "rb_first(root)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/rbtree.c",
        include_line: "#include <linux/rbtree.h>",
        helper_symbol: "rust_helper_rb_last",
        forwards_to: "rb_last(root)",
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
            "/vendor/linux/rust/helpers/rbtree.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
