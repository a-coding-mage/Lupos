//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/workqueue.c
//! test-origin: linux:vendor/linux/rust/helpers/workqueue.c
//! Rust helper shim for initialized work items.

use super::RustHelperSource;

pub const SOURCE: RustHelperSource = RustHelperSource {
    linux_source: "vendor/linux/rust/helpers/workqueue.c",
    include_line: "#include <linux/workqueue.h>",
    helper_symbol: "rust_helper_init_work_with_key",
    forwards_to: "lockdep_init_map(&work->lockdep_map, name, key, 0)",
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
                "/vendor/linux/rust/helpers/workqueue.c"
            )),
            SOURCE,
        );
    }
}
