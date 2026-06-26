//! linux-parity: complete
//! linux-source: vendor/linux/lib/zstd/common/debug.c
//! test-origin: linux:vendor/linux/lib/zstd/common/debug.c
//! Zstd debug-level global.

pub const DEBUGLEVEL_GATE: &str = "#if (DEBUGLEVEL>=2)";
pub const GLOBAL_SYMBOL: &str = "g_debuglevel";

pub const fn debuglevel_global(debuglevel: i32) -> Option<i32> {
    if debuglevel >= 2 {
        Some(debuglevel)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstd_common_debug_source_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zstd/common/debug.c"
        ));
        assert!(source.contains("#include \"debug.h\""));
        assert!(source.contains(DEBUGLEVEL_GATE));
        assert!(source.contains("int g_debuglevel = DEBUGLEVEL;"));
        assert_eq!(debuglevel_global(1), None);
        assert_eq!(debuglevel_global(2), Some(2));
    }
}
