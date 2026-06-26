//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/cpumask.c
//! test-origin: linux:vendor/linux/rust/helpers/cpumask.c
//! Rust helper shims for Linux CPU mask operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_set_cpu",
        forwards_to: "cpumask_set_cpu(cpu, dstp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper___cpumask_set_cpu",
        forwards_to: "__cpumask_set_cpu(cpu, dstp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_clear_cpu",
        forwards_to: "cpumask_clear_cpu(cpu, dstp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper___cpumask_clear_cpu",
        forwards_to: "__cpumask_clear_cpu(cpu, dstp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_test_cpu",
        forwards_to: "cpumask_test_cpu(cpu, srcp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_setall",
        forwards_to: "cpumask_setall(dstp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_empty",
        forwards_to: "cpumask_empty(srcp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_full",
        forwards_to: "cpumask_full(srcp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_weight",
        forwards_to: "cpumask_weight(srcp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_cpumask_copy",
        forwards_to: "cpumask_copy(dstp, srcp)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_alloc_cpumask_var",
        forwards_to: "alloc_cpumask_var(mask, flags)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_zalloc_cpumask_var",
        forwards_to: "zalloc_cpumask_var(mask, flags)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
        include_line: "#include <linux/cpumask.h>",
        helper_symbol: "rust_helper_free_cpumask_var",
        forwards_to: "free_cpumask_var(mask)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/cpumask.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
