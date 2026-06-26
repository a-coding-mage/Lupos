//! linux-parity: complete
//! linux-source: vendor/linux/net/smc/smc_tracepoint.c
//! test-origin: linux:vendor/linux/net/smc/smc_tracepoint.c
//! SMC tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/smc/smc_tracepoint.c",
    headers: &["#include \"smc_tracepoint.h\""],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[
        "EXPORT_TRACEPOINT_SYMBOL(smc_switch_to_fallback);",
        "EXPORT_TRACEPOINT_SYMBOL(smc_tx_sendmsg);",
        "EXPORT_TRACEPOINT_SYMBOL(smc_rx_recvmsg);",
        "EXPORT_TRACEPOINT_SYMBOL(smcr_link_down);",
    ],
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
                "/vendor/linux/net/smc/smc_tracepoint.c"
            )),
            SOURCE,
        );
    }
}
