//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/loongarch/xor_simd.c
//! test-origin: linux:vendor/linux/lib/raid/xor/loongarch/xor_simd.c
//! LoongArch LSX/LASX SIMD XOR template metadata.

pub const LINE_WIDTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoongarchXorSimdFlavor {
    pub name: &'static str,
    pub config: &'static str,
    pub register_prefix: &'static str,
    pub input_load_offsets: &'static [usize],
    pub xor_load_offsets: &'static [usize],
}

pub const LSX_FLAVOR: LoongarchXorSimdFlavor = LoongarchXorSimdFlavor {
    name: "lsx",
    config: "CONFIG_CPU_HAS_LSX",
    register_prefix: "$vr",
    input_load_offsets: &[0, 16, 32, 48],
    xor_load_offsets: &[0, 16, 32, 48],
};

pub const LASX_FLAVOR: LoongarchXorSimdFlavor = LoongarchXorSimdFlavor {
    name: "lasx",
    config: "CONFIG_CPU_HAS_LASX",
    register_prefix: "$xr",
    input_load_offsets: &[0, 32],
    xor_load_offsets: &[0, 32],
};

pub const XOR_SIMD_FLAVORS: &[LoongarchXorSimdFlavor] = &[LSX_FLAVOR, LASX_FLAVOR];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loongarch_xor_simd_matches_linux_templates() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/loongarch/xor_simd.c"
        ));
        assert!(source.contains("#include \"xor_simd.h\""));
        assert!(source.contains("#define LINE_WIDTH 64"));
        assert!(source.contains("#ifdef CONFIG_CPU_HAS_LSX"));
        assert!(source.contains("\"vld $vr\" #reg"));
        assert!(source.contains("\"vxor.v $vr\" #dj"));
        assert!(source.contains("LD(0, base, 0)"));
        assert!(source.contains("LD(3, base, 48)"));
        assert!(source.contains("#define XOR_FUNC_NAME(nr) __xor_lsx_##nr"));
        assert!(source.contains("#ifdef CONFIG_CPU_HAS_LASX"));
        assert!(source.contains("\"xvld $xr\" #reg"));
        assert!(source.contains("\"xvxor.v $xr\" #dj"));
        assert!(source.contains("LD(1, base, 32)"));
        assert!(source.contains("#define XOR_FUNC_NAME(nr) __xor_lasx_##nr"));
        assert!(source.matches("#include \"xor_template.c\"").count() >= 2);

        assert_eq!(LINE_WIDTH, 64);
        assert_eq!(XOR_SIMD_FLAVORS, [LSX_FLAVOR, LASX_FLAVOR]);
        assert_eq!(LSX_FLAVOR.input_load_offsets, [0, 16, 32, 48]);
        assert_eq!(LASX_FLAVOR.input_load_offsets, [0, 32]);
    }
}
