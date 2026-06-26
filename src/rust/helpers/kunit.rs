//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/kunit.c
//! test-origin: linux:vendor/linux/rust/helpers/kunit.c
//! Rust helper shim for KUnit current-test lookup.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/kunit.c",
    include_line: "#include <kunit/test-bug.h>",
    helper_symbol: "rust_helper_kunit_get_current_test",
    forwards_to: "kunit_get_current_test()",
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
                "/vendor/linux/rust/helpers/kunit.c"
            )),
            SOURCE,
        );
    }
}
