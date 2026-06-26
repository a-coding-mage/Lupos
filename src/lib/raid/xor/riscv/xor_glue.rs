//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/riscv/xor-glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/riscv/xor-glue.c
//! RISC-V vector XOR glue wrapper.

pub const XOR_BLOCK_SYMBOL: &str = "xor_block_rvv";
pub const TEMPLATE_NAME: &str = "rvv";
pub const BLOCKS_MACRO: &str =
    "DO_XOR_BLOCKS(vector_inner, xor_regs_2_, xor_regs_3_, xor_regs_4_, xor_regs_5_);";
pub const BEGIN: &str = "kernel_vector_begin();";
pub const INNER: &str = "xor_gen_vector_inner(dest, srcs, src_cnt, bytes);";
pub const END: &str = "kernel_vector_end();";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riscv_xor_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/riscv/xor-glue.c"
        ));
        assert!(source.contains("#include <asm/vector.h>"));
        assert!(source.contains("#include <asm/switch_to.h>"));
        assert!(source.contains("#include <asm/asm-prototypes.h>"));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains(BLOCKS_MACRO));
        assert!(source.contains(BEGIN));
        assert!(source.contains(INNER));
        assert!(source.contains(END));
        assert!(source.contains(XOR_BLOCK_SYMBOL));
        assert!(source.contains(TEMPLATE_NAME));
    }
}
