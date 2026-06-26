//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/device.c
//! test-origin: linux:vendor/linux/rust/helpers/device.c
//! Rust helper shims for device-managed actions and driver data.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/device.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/device.h>",
        helper_symbol: "rust_helper_devm_add_action",
        forwards_to: "devm_add_action(dev, action, data)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/device.h>",
        helper_symbol: "rust_helper_devm_add_action_or_reset",
        forwards_to: "devm_add_action_or_reset(dev, action, data)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/device.h>",
        helper_symbol: "rust_helper_dev_get_drvdata",
        forwards_to: "dev_get_drvdata(dev)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/device.h>",
        helper_symbol: "rust_helper_dev_set_drvdata",
        forwards_to: "dev_set_drvdata(dev, data)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/device.h>",
        helper_symbol: "rust_helper_dev_name",
        forwards_to: "dev_name(dev)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_device_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/device.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
