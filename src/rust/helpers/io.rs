//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/io.c
//! test-origin: linux:vendor/linux/rust/helpers/io.c
//! Rust helper shims for MMIO and resource region operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_ioremap",
        forwards_to: "ioremap(offset, size)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_ioremap_np",
        forwards_to: "ioremap_np(offset, size)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_iounmap",
        forwards_to: "iounmap(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readb",
        forwards_to: "readb(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readw",
        forwards_to: "readw(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readl",
        forwards_to: "readl(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readq",
        forwards_to: "readq(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writeb",
        forwards_to: "writeb(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writew",
        forwards_to: "writew(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writel",
        forwards_to: "writel(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writeq",
        forwards_to: "writeq(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readb_relaxed",
        forwards_to: "readb_relaxed(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readw_relaxed",
        forwards_to: "readw_relaxed(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readl_relaxed",
        forwards_to: "readl_relaxed(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_readq_relaxed",
        forwards_to: "readq_relaxed(addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writeb_relaxed",
        forwards_to: "writeb_relaxed(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writew_relaxed",
        forwards_to: "writew_relaxed(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writel_relaxed",
        forwards_to: "writel_relaxed(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/io.h>",
        helper_symbol: "rust_helper_writeq_relaxed",
        forwards_to: "writeq_relaxed(value, addr)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_resource_size",
        forwards_to: "resource_size(res)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_request_mem_region",
        forwards_to: "request_mem_region(start, n, name)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_release_mem_region",
        forwards_to: "release_mem_region(start, n)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_request_region",
        forwards_to: "request_region(start, n, name)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_request_muxed_region",
        forwards_to: "request_muxed_region(start, n, name)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/io.c",
        include_line: "#include <linux/ioport.h>",
        helper_symbol: "rust_helper_release_region",
        forwards_to: "release_region(start, n)",
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
            "/vendor/linux/rust/helpers/io.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
