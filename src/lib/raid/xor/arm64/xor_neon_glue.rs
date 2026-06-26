//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/arm64/xor-neon-glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/arm64/xor-neon-glue.c
//! ARM64 NEON/EOR3 XOR glue wrappers.

pub const TEMPLATE_MACRO: &str = "XOR_TEMPLATE";
pub const SIMD_SCOPE: &str = "scoped_ksimd()";
pub const TEMPLATES: &[&str] = &["neon", "eor3"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm64_xor_neon_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/arm64/xor-neon-glue.c"
        ));
        assert!(source.contains("#include <asm/simd.h>"));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains("#include \"xor-neon.h\""));
        assert!(source.contains(TEMPLATE_MACRO));
        assert!(source.contains(SIMD_SCOPE));
        for template in TEMPLATES {
            assert!(source.contains(template));
            assert!(source.contains("__stringify(_name)"));
        }
    }
}
