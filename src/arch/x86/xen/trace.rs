//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/trace.c
//! test-origin: linux:vendor/linux/arch/x86/xen/trace.c
//! Xen hypercall tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/arch/x86/xen/trace.c",
    headers: &[
        "#include <linux/ftrace.h>",
        "#include <xen/interface/xen.h>",
        "#include <xen/interface/xen-mca.h>",
        "#include <asm/xen-hypercalls.h>",
        "#include <trace/events/xen.h>",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[],
    module_description: None,
};

pub fn xen_hypercall_name(op: usize, names: &[Option<&'static str>]) -> &'static str {
    names.get(op).and_then(|name| *name).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_compile_unit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/trace.c"
        ));
        crate::kernel::trace::assert_trace_compile_unit(source, SOURCE);
        assert!(source.contains("#define HYPERCALL(x)"));
        assert!(source.contains("xen_hypercall_names[op] != NULL"));
    }

    #[test]
    fn xen_hypercall_name_matches_linux_fallbacks() {
        let names = [Some("(set_trap_table)"), None, Some("(sched_op)")];
        assert_eq!(xen_hypercall_name(0, &names), "(set_trap_table)");
        assert_eq!(xen_hypercall_name(1, &names), "");
        assert_eq!(xen_hypercall_name(99, &names), "");
    }
}
