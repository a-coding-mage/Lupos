//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/trace.c
//! test-origin: linux:vendor/linux/net/mac80211/trace.c
//! mac80211 tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/net/mac80211/trace.c",
    headers: &[
        "#include <linux/module.h>",
        "#include <net/cfg80211.h>",
        "#include \"driver-ops.h\"",
        "#include \"debug.h\"",
        "#include \"trace.h\"",
        "#include \"trace_msg.h\"",
    ],
    create_trace_points: true,
    checker_gated: true,
    exported_tracepoints: &[],
    module_description: None,
};

pub const MESSAGE_TRACE_HELPERS: [&str; 4] =
    ["__sdata_info", "__sdata_dbg", "__sdata_err", "__wiphy_dbg"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_compile_unit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/trace.c"
        ));
        crate::kernel::trace::assert_trace_compile_unit(source, SOURCE);
        assert!(source.contains("#ifdef CONFIG_MAC80211_MESSAGE_TRACING"));
        assert!(source.contains("void __sdata_info(const char *fmt, ...)"));
        assert!(source.contains("void __sdata_dbg(bool print, const char *fmt, ...)"));
        assert!(source.contains("void __sdata_err(const char *fmt, ...)"));
        assert!(source.contains("void __wiphy_dbg(struct wiphy *wiphy, bool print"));
        assert!(source.contains("trace_mac80211_info(&vaf);"));
        assert!(source.contains("trace_mac80211_dbg(&vaf);"));
        assert!(source.contains("trace_mac80211_err(&vaf);"));
        assert_eq!(
            MESSAGE_TRACE_HELPERS,
            ["__sdata_info", "__sdata_dbg", "__sdata_err", "__wiphy_dbg"]
        );
    }
}
