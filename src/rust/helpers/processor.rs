//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/processor.c
//! test-origin: linux:vendor/linux/rust/helpers/processor.c
//! Rust helper shim for CPU relax.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/processor.c",
    include_line: "#include <linux/processor.h>",
    helper_symbol: "rust_helper_cpu_relax",
    forwards_to: "cpu_relax()",
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
                "/vendor/linux/rust/helpers/processor.c"
            )),
            SOURCE,
        );
    }
}
