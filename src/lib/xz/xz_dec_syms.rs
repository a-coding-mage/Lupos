//! linux-parity: complete
//! linux-source: vendor/linux/lib/xz/xz_dec_syms.c
//! test-origin: linux:vendor/linux/lib/xz/xz_dec_syms.c
//! XZ decoder export metadata.

pub const EXPORTED_SYMBOLS: &[&str] = &["xz_dec_init", "xz_dec_reset", "xz_dec_run", "xz_dec_end"];
pub const MICROLZMA_EXPORTED_SYMBOLS: &[&str] = &[
    "xz_dec_microlzma_alloc",
    "xz_dec_microlzma_reset",
    "xz_dec_microlzma_run",
    "xz_dec_microlzma_end",
];
pub const MODULE_DESCRIPTION: &str = "XZ decompressor";
pub const MODULE_VERSION: &str = "1.2";
pub const MODULE_AUTHOR: &str = "Lasse Collin <lasse.collin@tukaani.org> and Igor Pavlov";
pub const MODULE_LICENSE: &str = "Dual BSD/GPL";

pub fn exported_symbols(microlzma: bool) -> &'static [&'static str] {
    if microlzma {
        MICROLZMA_EXPORTED_SYMBOLS
    } else {
        EXPORTED_SYMBOLS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xz_decoder_symbol_exports_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/xz/xz_dec_syms.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/xz.h>"));
        for symbol in EXPORTED_SYMBOLS {
            assert!(source.contains(symbol));
        }
        assert!(source.contains("#ifdef CONFIG_XZ_DEC_MICROLZMA"));
        for symbol in MICROLZMA_EXPORTED_SYMBOLS {
            assert!(source.contains(symbol));
        }
        assert!(source.contains("MODULE_DESCRIPTION(\"XZ decompressor\")"));
        assert!(source.contains("MODULE_VERSION(\"1.2\")"));
        assert!(source.contains("MODULE_LICENSE(\"Dual BSD/GPL\")"));
        assert_eq!(exported_symbols(false), EXPORTED_SYMBOLS);
        assert_eq!(exported_symbols(true), MICROLZMA_EXPORTED_SYMBOLS);
        assert_eq!(MODULE_DESCRIPTION, "XZ decompressor");
        assert_eq!(MODULE_VERSION, "1.2");
        assert_eq!(
            MODULE_AUTHOR,
            "Lasse Collin <lasse.collin@tukaani.org> and Igor Pavlov"
        );
        assert_eq!(MODULE_LICENSE, "Dual BSD/GPL");
    }
}
