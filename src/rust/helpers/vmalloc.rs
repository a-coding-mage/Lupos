//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/vmalloc.c
//! test-origin: linux:vendor/linux/rust/helpers/vmalloc.c
//! Rust helper shim for vmalloc reallocation.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/vmalloc.c",
    include_line: "#include <linux/vmalloc.h>",
    helper_symbol: "rust_helper_vrealloc_node_align",
    forwards_to: "vrealloc_node_align(p, size, align, flags, node)",
};

pub fn source() -> RustHelperSource {
    SOURCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        super::super::assert_helper_source(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/rust/helpers/vmalloc.c"
            )),
            SOURCE,
        );
    }
}
