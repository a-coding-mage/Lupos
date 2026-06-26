//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/powerpc/xor_vmx_glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/powerpc/xor_vmx_glue.c
//! PowerPC Altivec XOR glue wrapper.

pub const XOR_BLOCK_SYMBOL: &str = "xor_block_altivec";
pub const TEMPLATE_NAME: &str = "altivec";
pub const BEGIN: &str = "preempt_disable();";
pub const ENABLE: &str = "enable_kernel_altivec();";
pub const INNER: &str = "xor_gen_altivec_inner(dest, srcs, src_cnt, bytes);";
pub const DISABLE: &str = "disable_kernel_altivec();";
pub const END: &str = "preempt_enable();";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerpc_xor_vmx_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/powerpc/xor_vmx_glue.c"
        ));
        for include in [
            "#include <linux/preempt.h>",
            "#include <linux/sched.h>",
            "#include <asm/switch_to.h>",
            "#include \"xor_impl.h\"",
            "#include \"xor_arch.h\"",
            "#include \"xor_vmx.h\"",
        ] {
            assert!(source.contains(include));
        }
        for token in [
            BEGIN,
            ENABLE,
            INNER,
            DISABLE,
            END,
            XOR_BLOCK_SYMBOL,
            TEMPLATE_NAME,
        ] {
            assert!(source.contains(token));
        }
    }
}
