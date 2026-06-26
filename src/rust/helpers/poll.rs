//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/poll.c
//! test-origin: linux:vendor/linux/rust/helpers/poll.c
//! Rust helper shim for poll wait registration.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/poll.c",
    include_line: "#include <linux/poll.h>",
    helper_symbol: "rust_helper_poll_wait",
    forwards_to: "poll_wait(filp, wait_address, p)",
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
                "/vendor/linux/rust/helpers/poll.c"
            )),
            SOURCE,
        );
    }
}
