//! linux-parity: complete
//! linux-source: vendor/linux/lib/zstd/zstd_common_module.c
//! test-origin: linux:vendor/linux/lib/zstd/zstd_common_module.c
//! Zstd common module export metadata.

pub const GPL_EXPORTED_SYMBOLS: &[&str] = &[
    "FSE_readNCount",
    "HUF_readStats",
    "HUF_readStats_wksp",
    "ZSTD_isError",
    "ZSTD_getErrorName",
    "ZSTD_getErrorCode",
];
pub const MODULE_DESCRIPTION: &str = "Zstd Common";
pub const MODULE_LICENSE: &str = "Dual BSD/GPL";

pub fn gpl_exported_symbols() -> &'static [&'static str] {
    GPL_EXPORTED_SYMBOLS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstd_common_symbol_exports_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zstd/zstd_common_module.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include \"common/huf.h\""));
        assert!(source.contains("#include \"common/fse.h\""));
        assert!(source.contains("#include \"common/zstd_internal.h\""));
        assert!(source.contains("#undef ZSTD_isError"));
        for symbol in GPL_EXPORTED_SYMBOLS {
            assert!(source.contains(&alloc::format!("EXPORT_SYMBOL_GPL({symbol});")));
        }
        assert!(source.contains("MODULE_LICENSE(\"Dual BSD/GPL\")"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Zstd Common\")"));
        assert_eq!(gpl_exported_symbols().len(), 6);
        assert_eq!(MODULE_DESCRIPTION, "Zstd Common");
        assert_eq!(MODULE_LICENSE, "Dual BSD/GPL");
    }
}
