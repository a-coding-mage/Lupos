//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/fs.c
//! test-origin: linux:vendor/linux/rust/helpers/fs.c
//! Rust helper shim for file reference acquisition.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/fs.c",
    include_line: "#include <linux/fs.h>",
    helper_symbol: "rust_helper_get_file",
    forwards_to: "get_file(f)",
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
                "/vendor/linux/rust/helpers/fs.c"
            )),
            SOURCE,
        );
    }
}
