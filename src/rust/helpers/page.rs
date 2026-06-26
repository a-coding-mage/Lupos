//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/page.c
//! test-origin: linux:vendor/linux/rust/helpers/page.c
//! Rust helper shims for page allocation and temporary mapping.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/page.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/gfp.h>",
        helper_symbol: "rust_helper_alloc_pages",
        forwards_to: "alloc_pages(gfp_mask, order)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/highmem.h>",
        helper_symbol: "rust_helper_kmap_local_page",
        forwards_to: "kmap_local_page(page)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/highmem.h>",
        helper_symbol: "rust_helper_kunmap_local",
        forwards_to: "kunmap_local(addr)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_page_to_nid",
        forwards_to: "page_to_nid(page)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_page_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/page.c"
        ));
        assert!(source.contains("#ifndef NODE_NOT_IN_PAGE_FLAGS"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
