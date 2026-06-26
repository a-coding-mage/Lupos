//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/simd.c
//! test-origin: linux:vendor/linux/lib/crypto/simd.c
//! Crypto SIMD test per-CPU disable flag.

pub const LINUX_SOURCE: &str = "vendor/linux/lib/crypto/simd.c";
pub const PER_CPU_SYMBOL: &str = "crypto_simd_disabled_for_test";

pub fn exported_per_cpu_symbol() -> &'static str {
    PER_CPU_SYMBOL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simd_source_exports_per_cpu_test_flag() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/simd.c"
        ));
        assert!(source.contains("#include <crypto/internal/simd.h>"));
        assert!(source.contains("DEFINE_PER_CPU(bool, crypto_simd_disabled_for_test);"));
        assert!(source.contains("EXPORT_PER_CPU_SYMBOL_GPL(crypto_simd_disabled_for_test);"));
        assert_eq!(exported_per_cpu_symbol(), PER_CPU_SYMBOL);
    }
}
