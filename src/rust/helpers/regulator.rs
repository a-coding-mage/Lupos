//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/regulator.c
//! test-origin: linux:vendor/linux/rust/helpers/regulator.c
//! Rust helper shims for regulator consumers.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/regulator.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_put",
        forwards_to: "regulator_put(regulator)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_set_voltage",
        forwards_to: "regulator_set_voltage(regulator, min_uV, max_uV)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_get_voltage",
        forwards_to: "regulator_get_voltage(regulator)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_get",
        forwards_to: "regulator_get(dev, id)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_enable",
        forwards_to: "regulator_enable(regulator)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_disable",
        forwards_to: "regulator_disable(regulator)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_regulator_is_enabled",
        forwards_to: "regulator_is_enabled(regulator)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_devm_regulator_get_enable",
        forwards_to: "devm_regulator_get_enable(dev, id)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/regulator/consumer.h>",
        helper_symbol: "rust_helper_devm_regulator_get_enable_optional",
        forwards_to: "devm_regulator_get_enable_optional(dev, id)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_regulator_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/regulator.c"
        ));
        assert!(source.contains("#ifndef CONFIG_REGULATOR"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
