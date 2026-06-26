//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/pid_namespace.c
//! test-origin: linux:vendor/linux/rust/helpers/pid_namespace.c
//! Rust helper shims for pid namespace references.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/pid_namespace.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pid_namespace.h>",
        helper_symbol: "rust_helper_get_pid_ns",
        forwards_to: "get_pid_ns(ns)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pid_namespace.h>",
        helper_symbol: "rust_helper_put_pid_ns",
        forwards_to: "put_pid_ns(ns)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pid_namespace.h>",
        helper_symbol: "rust_helper_task_get_pid_ns",
        forwards_to: "task_active_pid_ns(task)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_pid_namespace_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/pid_namespace.c"
        ));
        assert!(source.contains("#include <linux/cleanup.h>"));
        assert!(source.contains("guard(rcu)();"));
        assert!(source.contains("if (pid_ns)"));
        assert!(source.contains("get_pid_ns(pid_ns);"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
