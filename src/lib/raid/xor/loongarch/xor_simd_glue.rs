//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/loongarch/xor_simd_glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/loongarch/xor_simd_glue.c
//! LoongArch SIMD XOR glue templates.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoongarchXorTemplate {
    pub flavor: &'static str,
    pub wraps_kernel_fpu: bool,
}

pub const LSX_TEMPLATE: LoongarchXorTemplate = LoongarchXorTemplate {
    flavor: "lsx",
    wraps_kernel_fpu: true,
};

pub const LASX_TEMPLATE: LoongarchXorTemplate = LoongarchXorTemplate {
    flavor: "lasx",
    wraps_kernel_fpu: true,
};

pub fn enabled_templates(cpu_has_lsx: bool, cpu_has_lasx: bool) -> &'static [LoongarchXorTemplate] {
    match (cpu_has_lsx, cpu_has_lasx) {
        (true, true) => &[LSX_TEMPLATE, LASX_TEMPLATE],
        (true, false) => &[LSX_TEMPLATE],
        (false, true) => &[LASX_TEMPLATE],
        (false, false) => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loongarch_xor_glue_matches_linux_macro_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/loongarch/xor_simd_glue.c"
        ));
        assert!(source.contains("#include <asm/fpu.h>"));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains("#include \"xor_simd.h\""));
        assert!(source.contains("#define MAKE_XOR_GLUES(flavor)"));
        assert!(source.contains("kernel_fpu_begin();"));
        assert!(source.contains("kernel_fpu_end();"));
        assert!(source.contains(".name\t\t= __stringify(flavor)"));
        assert!(source.contains("MAKE_XOR_GLUES(lsx);"));
        assert!(source.contains("MAKE_XOR_GLUES(lasx);"));
        assert_eq!(enabled_templates(true, true), [LSX_TEMPLATE, LASX_TEMPLATE]);
        assert_eq!(enabled_templates(false, false).len(), 0);
    }
}
