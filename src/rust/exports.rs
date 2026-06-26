//! linux-parity: complete
//! linux-source: vendor/linux/rust/exports.c
//! test-origin: linux:vendor/linux/rust/exports.c
//! Rust exported-symbol C bridge metadata.

pub const EXPORT_MACRO: &str =
    "#define EXPORT_SYMBOL_RUST_GPL(sym) extern int sym; EXPORT_SYMBOL_GPL(sym)";
pub const GENERATED_EXPORTS: &[&str] = &[
    "#include \"exports_core_generated.h\"",
    "#include \"exports_bindings_generated.h\"",
    "#include \"exports_kernel_generated.h\"",
    "#include \"exports_helpers_generated.h\"",
];
pub const BUILD_ERROR_SYMBOL: &str = "rust_build_error";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_exports_bridge_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/exports.c"
        ));
        assert!(source.contains("#include <linux/export.h>"));
        assert!(source.contains(EXPORT_MACRO));
        assert!(source.contains("#ifndef CONFIG_RUST_INLINE_HELPERS"));
        for include in GENERATED_EXPORTS {
            assert!(source.contains(include));
        }
        assert!(source.contains("#ifdef CONFIG_RUST_BUILD_ASSERT_ALLOW"));
        assert!(source.contains(BUILD_ERROR_SYMBOL));
    }
}
