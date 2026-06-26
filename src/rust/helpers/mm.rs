//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/mm.c
//! test-origin: linux:vendor/linux/rust/helpers/mm.c
//! Rust helper shims for memory-management references and mmap locking.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/mm.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_mmgrab",
        forwards_to: "mmgrab(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_mmdrop",
        forwards_to: "mmdrop(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_mmget",
        forwards_to: "mmget(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_mmget_not_zero",
        forwards_to: "mmget_not_zero(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mmap_lock.h>",
        helper_symbol: "rust_helper_mmap_read_lock",
        forwards_to: "mmap_read_lock(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mmap_lock.h>",
        helper_symbol: "rust_helper_mmap_read_trylock",
        forwards_to: "mmap_read_trylock(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mmap_lock.h>",
        helper_symbol: "rust_helper_mmap_read_unlock",
        forwards_to: "mmap_read_unlock(mm)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_vma_lookup",
        forwards_to: "vma_lookup(mm, addr)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/mm.h>",
        helper_symbol: "rust_helper_vma_end_read",
        forwards_to: "vma_end_read(vma)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_mm_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/mm.c"
        ));
        assert!(source.contains("#include <linux/sched/mm.h>"));
        for contract in SOURCES {
            assert!(source.contains(contract.helper_symbol));
            assert!(source.contains(contract.forwards_to));
        }
    }
}
