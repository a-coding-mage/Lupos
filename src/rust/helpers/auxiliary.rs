//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/auxiliary.c
//! test-origin: linux:vendor/linux/rust/helpers/auxiliary.c
//! Rust helper shims for auxiliary devices.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/auxiliary.c",
        include_line: "#include <linux/auxiliary_bus.h>",
        helper_symbol: "rust_helper_auxiliary_device_uninit",
        forwards_to: "auxiliary_device_uninit(adev)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/auxiliary.c",
        include_line: "#include <linux/auxiliary_bus.h>",
        helper_symbol: "rust_helper_auxiliary_device_delete",
        forwards_to: "auxiliary_device_delete(adev)",
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
            "/vendor/linux/rust/helpers/auxiliary.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
