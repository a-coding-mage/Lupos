//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/clk.c
//! test-origin: linux:vendor/linux/rust/helpers/clk.c
//! Rust helper shims for Linux clock framework operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_get",
        forwards_to: "clk_get(dev, id)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_put",
        forwards_to: "clk_put(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_enable",
        forwards_to: "clk_enable(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_disable",
        forwards_to: "clk_disable(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_get_rate",
        forwards_to: "clk_get_rate(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_set_rate",
        forwards_to: "clk_set_rate(clk, rate)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_prepare",
        forwards_to: "clk_prepare(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_unprepare",
        forwards_to: "clk_unprepare(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_get_optional",
        forwards_to: "clk_get_optional(dev, id)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_prepare_enable",
        forwards_to: "clk_prepare_enable(clk)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/clk.c",
        include_line: "#include <linux/clk.h>",
        helper_symbol: "rust_helper_clk_disable_unprepare",
        forwards_to: "clk_disable_unprepare(clk)",
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
            "/vendor/linux/rust/helpers/clk.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
