//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/barrier.c
//! test-origin: linux:vendor/linux/rust/helpers/barrier.c
//! Rust helper shims for SMP memory barriers.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/barrier.c",
        include_line: "#include <asm/barrier.h>",
        helper_symbol: "rust_helper_smp_mb",
        forwards_to: "smp_mb()",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/barrier.c",
        include_line: "#include <asm/barrier.h>",
        helper_symbol: "rust_helper_smp_wmb",
        forwards_to: "smp_wmb()",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/barrier.c",
        include_line: "#include <asm/barrier.h>",
        helper_symbol: "rust_helper_smp_rmb",
        forwards_to: "smp_rmb()",
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
            "/vendor/linux/rust/helpers/barrier.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
