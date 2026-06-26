//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/usb.c
//! test-origin: linux:vendor/linux/rust/helpers/usb.c
//! Rust helper shim for USB interface-to-device conversion.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/usb.c",
    include_line: "#include <linux/usb.h>",
    helper_symbol: "rust_helper_interface_to_usbdev",
    forwards_to: "interface_to_usbdev(intf)",
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
                "/vendor/linux/rust/helpers/usb.c"
            )),
            SOURCE,
        );
    }
}
