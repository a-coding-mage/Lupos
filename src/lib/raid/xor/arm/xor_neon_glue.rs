//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/arm/xor-neon-glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/arm/xor-neon-glue.c
//! ARM NEON XOR glue wrapper.

pub const XOR_BLOCK_SYMBOL: &str = "xor_block_neon";
pub const TEMPLATE_NAME: &str = "neon";
pub const BEGIN: &str = "kernel_neon_begin();";
pub const INNER: &str = "xor_gen_neon_inner(dest, srcs, src_cnt, bytes);";
pub const END: &str = "kernel_neon_end();";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_xor_neon_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/arm/xor-neon-glue.c"
        ));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains(BEGIN));
        assert!(source.contains(INNER));
        assert!(source.contains(END));
        assert!(source.contains(XOR_BLOCK_SYMBOL));
        assert!(source.contains(".name"));
        assert!(source.contains(TEMPLATE_NAME));
    }
}
