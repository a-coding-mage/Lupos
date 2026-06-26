//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/pwm.c
//! test-origin: linux:vendor/linux/rust/helpers/pwm.c
//! Rust helper shims for PWM chip parent and driver data.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/pwm.c",
        include_line: "#include <linux/pwm.h>",
        helper_symbol: "rust_helper_pwmchip_parent",
        forwards_to: "pwmchip_parent(chip)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/pwm.c",
        include_line: "#include <linux/pwm.h>",
        helper_symbol: "rust_helper_pwmchip_get_drvdata",
        forwards_to: "pwmchip_get_drvdata(chip)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/pwm.c",
        include_line: "#include <linux/pwm.h>",
        helper_symbol: "rust_helper_pwmchip_set_drvdata",
        forwards_to: "pwmchip_set_drvdata(chip, data)",
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
            "/vendor/linux/rust/helpers/pwm.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
