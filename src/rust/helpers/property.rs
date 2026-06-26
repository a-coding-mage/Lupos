//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/property.c
//! test-origin: linux:vendor/linux/rust/helpers/property.c
//! Rust helper shim for firmware-node reference release.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/property.c",
    include_line: "#include <linux/property.h>",
    helper_symbol: "rust_helper_fwnode_handle_put",
    forwards_to: "fwnode_handle_put(fwnode)",
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
                "/vendor/linux/rust/helpers/property.c"
            )),
            SOURCE,
        );
    }
}
