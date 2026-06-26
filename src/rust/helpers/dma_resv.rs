//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/dma-resv.c
//! test-origin: linux:vendor/linux/rust/helpers/dma-resv.c
//! Rust helper shims for DMA reservation locking.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/dma-resv.c",
        include_line: "#include <linux/dma-resv.h>",
        helper_symbol: "rust_helper_dma_resv_lock",
        forwards_to: "dma_resv_lock(obj, ctx)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/dma-resv.c",
        include_line: "#include <linux/dma-resv.h>",
        helper_symbol: "rust_helper_dma_resv_unlock",
        forwards_to: "dma_resv_unlock(obj)",
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
            "/vendor/linux/rust/helpers/dma-resv.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
