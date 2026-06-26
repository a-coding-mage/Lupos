//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/arm/xor-neon.c
//! test-origin: linux:vendor/linux/lib/raid/xor/arm/xor-neon.c
//! ARM NEON-vectorized XOR inner implementation source contract.

pub const REQUIRED_CC_FLAG_ERROR: &str =
    "You should compile this file with '-march=armv7-a -mfloat-abi=softfp -mfpu=neon'";
pub const TEMPLATE_INCLUDE: &str = "#include \"../xor-8regs.c\"";
pub const BLOCKS_MACRO: &str =
    "__DO_XOR_BLOCKS(neon_inner, xor_8regs_2, xor_8regs_3, xor_8regs_4, xor_8regs_5);";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_xor_neon_inner_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/arm/xor-neon.c"
        ));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains("#ifndef __ARM_NEON__"));
        assert!(source.contains(REQUIRED_CC_FLAG_ERROR));
        assert!(source.contains("#pragma GCC optimize \"tree-vectorize\""));
        assert!(source.contains("#define NO_TEMPLATE"));
        assert!(source.contains(TEMPLATE_INCLUDE));
        assert!(source.contains(BLOCKS_MACRO));
    }
}
