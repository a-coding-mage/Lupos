//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/time.c
//! test-origin: linux:vendor/linux/rust/helpers/time.c
//! Rust helper shims for Linux timekeeping.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/time.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/delay.h>",
        helper_symbol: "rust_helper_fsleep",
        forwards_to: "fsleep(usecs)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/ktime.h>",
        helper_symbol: "rust_helper_ktime_get_real",
        forwards_to: "ktime_get_real()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/ktime.h>",
        helper_symbol: "rust_helper_ktime_get_boottime",
        forwards_to: "ktime_get_boottime()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/ktime.h>",
        helper_symbol: "rust_helper_ktime_get_clocktai",
        forwards_to: "ktime_get_clocktai()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/ktime.h>",
        helper_symbol: "rust_helper_ktime_to_us",
        forwards_to: "ktime_to_us(kt)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/ktime.h>",
        helper_symbol: "rust_helper_ktime_to_ms",
        forwards_to: "ktime_to_ms(kt)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/delay.h>",
        helper_symbol: "rust_helper_udelay",
        forwards_to: "udelay(usec)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_time_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/time.c"
        ));
        assert!(source.contains("#include <linux/timekeeping.h>"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
