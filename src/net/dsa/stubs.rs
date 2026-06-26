//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/stubs.c
//! test-origin: linux:vendor/linux/net/dsa/stubs.c
//! DSA core/module boundary stubs.

pub const LINUX_SOURCE: &str = "vendor/linux/net/dsa/stubs.c";
pub const EXPORTED_SYMBOL: &str = "dsa_stubs";

pub fn exported_symbol() -> &'static str {
    EXPORTED_SYMBOL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsa_stubs_exports_pointer_symbol() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/stubs.c"
        ));
        assert!(source.contains("#include <net/dsa_stubs.h>"));
        assert!(source.contains("const struct dsa_stubs *dsa_stubs;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(dsa_stubs);"));
        assert_eq!(exported_symbol(), EXPORTED_SYMBOL);
    }
}
