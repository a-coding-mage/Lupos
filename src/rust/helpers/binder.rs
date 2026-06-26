//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/binder.c
//! test-origin: linux:vendor/linux/rust/helpers/binder.c
//! Rust helper shims used by binder-facing code.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/binder.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/list_lru.h>",
        helper_symbol: "rust_helper_list_lru_count",
        forwards_to: "list_lru_count(lru)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/list_lru.h>",
        helper_symbol: "rust_helper_list_lru_walk",
        forwards_to: "list_lru_walk(lru, isolate, cb_arg, nr_to_walk)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/task_work.h>",
        helper_symbol: "rust_helper_init_task_work",
        forwards_to: "init_task_work(twork, func)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_binder_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/binder.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
