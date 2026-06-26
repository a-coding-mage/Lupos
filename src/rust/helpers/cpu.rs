//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/cpu.c
//! test-origin: linux:vendor/linux/rust/helpers/cpu.c
//! Rust helper shim for raw CPU-id lookup.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/cpu.c",
    include_line: "#include <linux/smp.h>",
    helper_symbol: "rust_helper_raw_smp_processor_id",
    forwards_to: "raw_smp_processor_id()",
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
                "/vendor/linux/rust/helpers/cpu.c"
            )),
            SOURCE,
        );
    }
}
