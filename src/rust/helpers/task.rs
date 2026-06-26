//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/task.c
//! test-origin: linux:vendor/linux/rust/helpers/task.c
//! Rust helper shims for task references and IDs.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/task.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/kernel.h>",
        helper_symbol: "rust_helper_might_resched",
        forwards_to: "might_resched()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_get_current",
        forwards_to: "current",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_get_task_struct",
        forwards_to: "get_task_struct(t)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_put_task_struct",
        forwards_to: "put_task_struct(t)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_task_uid",
        forwards_to: "task_uid(task)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_task_euid",
        forwards_to: "task_euid(task)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_from_kuid",
        forwards_to: "from_kuid(to, uid)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_uid_eq",
        forwards_to: "uid_eq(left, right)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_current_euid",
        forwards_to: "current_euid()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_current_user_ns",
        forwards_to: "current_user_ns()",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/sched/task.h>",
        helper_symbol: "rust_helper_task_tgid_nr_ns",
        forwards_to: "task_tgid_nr_ns(tsk, ns)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_task_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/task.c"
        ));
        assert!(source.contains("#ifndef CONFIG_USER_NS"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
