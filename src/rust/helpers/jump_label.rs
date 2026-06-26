//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/jump_label.c
//! test-origin: linux:vendor/linux/rust/helpers/jump_label.c
//! Rust helper shim for static-key counts without jump labels.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/jump_label.c",
    include_line: "#include <linux/jump_label.h>",
    helper_symbol: "rust_helper_static_key_count",
    forwards_to: "static_key_count(key)",
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
                "/vendor/linux/rust/helpers/jump_label.c"
            )),
            SOURCE,
        );
    }
}
