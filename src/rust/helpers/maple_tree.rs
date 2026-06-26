//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/maple_tree.c
//! test-origin: linux:vendor/linux/rust/helpers/maple_tree.c
//! Rust helper shim for maple-tree initialization flags.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/maple_tree.c",
    include_line: "#include <linux/maple_tree.h>",
    helper_symbol: "rust_helper_mt_init_flags",
    forwards_to: "mt_init_flags(mt, flags)",
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
                "/vendor/linux/rust/helpers/maple_tree.c"
            )),
            SOURCE,
        );
    }
}
