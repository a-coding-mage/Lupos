//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/slab.c
//! test-origin: linux:vendor/linux/rust/helpers/slab.c
//! Rust helper shims for slab/vmalloc reallocation.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/slab.c",
        include_line: "#include <linux/slab.h>",
        helper_symbol: "rust_helper_krealloc_node_align",
        forwards_to: "krealloc_node_align(objp, new_size, align, flags, node)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/slab.c",
        include_line: "#include <linux/slab.h>",
        helper_symbol: "rust_helper_kvrealloc_node_align",
        forwards_to: "kvrealloc_node_align(p, size, align, flags, node)",
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
            "/vendor/linux/rust/helpers/slab.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
