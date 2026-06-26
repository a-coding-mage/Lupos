//! linux-parity: complete
//! linux-source: vendor/linux/net/handshake/trace.c
//! test-origin: linux:vendor/linux/net/handshake/trace.c
//! Transport security handshake tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/handshake/trace.c",
    headers: &[
        "#include <linux/types.h>",
        "#include <linux/ipv6.h>",
        "#include <net/sock.h>",
        "#include <net/inet_sock.h>",
        "#include <net/netlink.h>",
        "#include <net/genetlink.h>",
        "#include \"handshake.h\"",
        "#include <trace/events/handshake.h>",
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
                "/vendor/linux/net/handshake/trace.c"
            )),
            SOURCE,
        );
    }
}
