//! linux-parity: complete
//! linux-source: vendor/linux/kernel/entry/syscall-common.c
//! test-origin: linux:vendor/linux/kernel/entry/syscall-common.c
//! Syscall tracepoint enter/exit helpers.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/kernel/entry/syscall-common.c",
    headers: &[
        "#include <linux/entry-common.h>",
        "#include <trace/events/syscalls.h>",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[],
    module_description: None,
};

pub const fn trace_syscall_enter_reread_syscall(reread_syscall: isize) -> isize {
    reread_syscall
}

pub const fn trace_syscall_exit_returns_void(_ret: isize) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_common_trace_source_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/entry/syscall-common.c"
        ));
        crate::kernel::trace::assert_trace_compile_unit(source, SOURCE);
        assert!(source.contains("trace_sys_enter(regs, syscall);"));
        assert!(source.contains("return syscall_get_nr(current, regs);"));
        assert!(source.contains("trace_sys_exit(regs, ret);"));
        assert_eq!(trace_syscall_enter_reread_syscall(42), 42);
        trace_syscall_exit_returns_void(-1);
    }
}
