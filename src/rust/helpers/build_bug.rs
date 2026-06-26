//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/build_bug.c
//! test-origin: linux:vendor/linux/rust/helpers/build_bug.c
//! Rust helper shim for Linux error-name lookup.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/build_bug.c",
    include_line: "#include <linux/errname.h>",
    helper_symbol: "rust_helper_errname",
    forwards_to: "errname(err)",
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
                "/vendor/linux/rust/helpers/build_bug.c"
            )),
            SOURCE,
        );
    }
}
