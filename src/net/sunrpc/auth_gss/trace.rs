//! linux-parity: complete
//! linux-source: vendor/linux/net/sunrpc/auth_gss/trace.c
//! test-origin: linux:vendor/linux/net/sunrpc/auth_gss/trace.c
//! SUNRPC GSS tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/sunrpc/auth_gss/trace.c",
    headers: &[
        "#include <linux/sunrpc/clnt.h>",
        "#include <linux/sunrpc/sched.h>",
        "#include <linux/sunrpc/svc.h>",
        "#include <linux/sunrpc/svc_xprt.h>",
        "#include <linux/sunrpc/auth_gss.h>",
        "#include <linux/sunrpc/gss_err.h>",
        "#include <trace/events/rpcgss.h>",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[],
    module_description: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_compile_unit_matches_linux_source() {
        crate::kernel::trace::assert_trace_compile_unit(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/net/sunrpc/auth_gss/trace.c"
            )),
            SOURCE,
        );
    }
}
