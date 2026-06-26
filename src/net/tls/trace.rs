//! linux-parity: complete
//! linux-source: vendor/linux/net/tls/trace.c
//! test-origin: linux:vendor/linux/net/tls/trace.c
//! TLS tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/tls/trace.c",
    headers: &["#include <linux/module.h>", "#include \"trace.h\""],
    create_trace_points: true,
    checker_gated: true,
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
                "/vendor/linux/net/tls/trace.c"
            )),
            SOURCE,
        );
    }
}
