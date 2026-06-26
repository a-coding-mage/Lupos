//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/dma.c
//! test-origin: linux:vendor/linux/rust/helpers/dma.c
//! Rust helper shims for DMA mapping.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/dma.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_alloc_attrs",
        forwards_to: "dma_alloc_attrs(dev, size, dma_handle, flag, attrs)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_free_attrs",
        forwards_to: "dma_free_attrs(dev, size, cpu_addr, dma_handle, attrs)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_set_mask_and_coherent",
        forwards_to: "dma_set_mask_and_coherent(dev, mask)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_set_mask",
        forwards_to: "dma_set_mask(dev, mask)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_set_coherent_mask",
        forwards_to: "dma_set_coherent_mask(dev, mask)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_map_sgtable",
        forwards_to: "dma_map_sgtable(dev, sgt, dir, attrs)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_max_mapping_size",
        forwards_to: "dma_max_mapping_size(dev)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/dma-mapping.h>",
        helper_symbol: "rust_helper_dma_set_max_seg_size",
        forwards_to: "dma_set_max_seg_size(dev, size)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_dma_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/dma.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
