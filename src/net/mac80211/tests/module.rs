//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/tests/module.c
//! test-origin: linux:vendor/linux/net/mac80211/tests/module.c
//! mac80211 KUnit module metadata.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/mac80211/tests/module.c",
    headers: &["#include <linux/module.h>", "MODULE_LICENSE(\"GPL\");"],
    create_trace_points: false,
    checker_gated: false,
    exported_tracepoints: &[],
    module_description: Some("MODULE_DESCRIPTION(\"tests for mac80211\");"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_compile_unit_matches_linux_source() {
        crate::kernel::trace::assert_trace_compile_unit(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/net/mac80211/tests/module.c"
            )),
            SOURCE,
        );
    }
}
