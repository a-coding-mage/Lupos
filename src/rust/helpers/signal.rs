//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/signal.c
//! test-origin: linux:vendor/linux/rust/helpers/signal.c
//! Rust helper shim for task signal-pending checks.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/signal.c",
    include_line: "#include <linux/sched/signal.h>",
    helper_symbol: "rust_helper_signal_pending",
    forwards_to: "signal_pending(t)",
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
                "/vendor/linux/rust/helpers/signal.c"
            )),
            SOURCE,
        );
    }
}
