//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/gpu.c
//! test-origin: linux:vendor/linux/rust/helpers/gpu.c
//! Rust helper shims for GPU buddy blocks.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/gpu.c",
        include_line: "#include <linux/gpu_buddy.h>",
        helper_symbol: "rust_helper_gpu_buddy_block_offset",
        forwards_to: "gpu_buddy_block_offset(block)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/gpu.c",
        include_line: "#include <linux/gpu_buddy.h>",
        helper_symbol: "rust_helper_gpu_buddy_block_order",
        forwards_to: "gpu_buddy_block_order(block)",
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
            "/vendor/linux/rust/helpers/gpu.c"
        ));
        assert!(source.contains("#ifdef CONFIG_GPU_BUDDY"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
