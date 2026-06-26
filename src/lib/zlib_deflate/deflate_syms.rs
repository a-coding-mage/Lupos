//! linux-parity: complete
//! linux-source: vendor/linux/lib/zlib_deflate/deflate_syms.c
//! test-origin: linux:vendor/linux/lib/zlib_deflate/deflate_syms.c
//! Exported symbols for zlib deflate.

pub const EXPORTED_SYMBOLS: &[&str] = &[
    "zlib_deflate_workspacesize",
    "zlib_deflate_dfltcc_enabled",
    "zlib_deflate",
    "zlib_deflateInit2",
    "zlib_deflateEnd",
    "zlib_deflateReset",
];
pub const MODULE_DESCRIPTION: &str = "Data compression using the deflation algorithm";
pub const MODULE_LICENSE: &str = "GPL";

pub fn exported_symbols() -> &'static [&'static str] {
    EXPORTED_SYMBOLS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlib_deflate_symbol_exports_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zlib_deflate/deflate_syms.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include <linux/zlib.h>"));
        for symbol in EXPORTED_SYMBOLS {
            assert!(source.contains(symbol));
        }
        assert!(
            source
                .contains("MODULE_DESCRIPTION(\"Data compression using the deflation algorithm\")")
        );
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert_eq!(exported_symbols().len(), 6);
        assert_eq!(
            MODULE_DESCRIPTION,
            "Data compression using the deflation algorithm"
        );
        assert_eq!(MODULE_LICENSE, "GPL");
    }
}
