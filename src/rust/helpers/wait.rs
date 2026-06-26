//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/wait.c
//! test-origin: linux:vendor/linux/rust/helpers/wait.c
//! Rust helper shim for wait-queue entry initialization.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/wait.c",
    include_line: "#include <linux/wait.h>",
    helper_symbol: "rust_helper_init_wait",
    forwards_to: "init_wait(wq_entry)",
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
                "/vendor/linux/rust/helpers/wait.c"
            )),
            SOURCE,
        );
    }
}
