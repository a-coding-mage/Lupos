//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/completion.c
//! test-origin: linux:vendor/linux/rust/helpers/completion.c
//! Rust helper shim for Linux completion initialization.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/completion.c",
    include_line: "#include <linux/completion.h>",
    helper_symbol: "rust_helper_init_completion",
    forwards_to: "init_completion(x)",
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
                "/vendor/linux/rust/helpers/completion.c"
            )),
            SOURCE,
        );
    }
}
