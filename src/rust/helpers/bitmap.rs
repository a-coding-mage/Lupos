//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/bitmap.c
//! test-origin: linux:vendor/linux/rust/helpers/bitmap.c
//! Rust helper shim for bitmap copy-and-extend.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/bitmap.c",
    include_line: "#include <linux/bitmap.h>",
    helper_symbol: "rust_helper_bitmap_copy_and_extend",
    forwards_to: "bitmap_copy_and_extend(to, from, count, size)",
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
                "/vendor/linux/rust/helpers/bitmap.c"
            )),
            SOURCE,
        );
    }
}
