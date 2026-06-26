//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/platform.c
//! test-origin: linux:vendor/linux/rust/helpers/platform.c
//! Rust helper shim for platform-device detection.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/platform.c",
    include_line: "#include <linux/platform_device.h>",
    helper_symbol: "rust_helper_dev_is_platform",
    forwards_to: "dev_is_platform(dev)",
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
                "/vendor/linux/rust/helpers/platform.c"
            )),
            SOURCE,
        );
    }
}
