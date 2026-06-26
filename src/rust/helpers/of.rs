//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/of.c
//! test-origin: linux:vendor/linux/rust/helpers/of.c
//! Rust helper shim for Open Firmware node detection.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/of.c",
    include_line: "#include <linux/of.h>",
    helper_symbol: "rust_helper_is_of_node",
    forwards_to: "is_of_node(fwnode)",
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
                "/vendor/linux/rust/helpers/of.c"
            )),
            SOURCE,
        );
    }
}
