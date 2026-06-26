//! linux-parity: complete
//! linux-source: vendor/linux/kernel/static_call.c
//! test-origin: linux:vendor/linux/kernel/static_call.c
//! Generic static-call fallback helpers.

pub const LINUX_SOURCE: &str = "vendor/linux/kernel/static_call.c";
pub const EXPORTED_SYMBOL: &str = "__static_call_return0";

pub const fn __static_call_return0() -> isize {
    0
}

pub fn exported_symbol() -> &'static str {
    EXPORTED_SYMBOL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_call_return0_matches_linux_fallback() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/static_call.c"
        ));
        assert!(source.contains("#include <linux/static_call.h>"));
        assert!(source.contains("long __static_call_return0(void)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__static_call_return0);"));
        assert_eq!(__static_call_return0(), 0);
        assert_eq!(exported_symbol(), EXPORTED_SYMBOL);
    }
}
