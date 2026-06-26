//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/loongarch
//! test-origin: linux:vendor/linux/lib/raid/xor/loongarch
//! LoongArch RAID XOR source coverage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoongarchXorFile {
    pub rust_module: Option<&'static str>,
    pub linux_source: &'static str,
    pub required_markers: &'static [&'static str],
}

pub mod xor_simd;
pub mod xor_simd_glue;
pub mod xor_template;

pub const LOONGARCH_XOR_IMPLEMENTATIONS: &[LoongarchXorFile] = &[
    LoongarchXorFile {
        rust_module: Some("xor_simd"),
        linux_source: "vendor/linux/lib/raid/xor/loongarch/xor_simd.c",
        required_markers: &[
            "#include \"xor_simd.h\"",
            "#define LINE_WIDTH 64",
            "#ifdef CONFIG_CPU_HAS_LSX",
            "#define XOR_FUNC_NAME(nr) __xor_lsx_##nr",
            "#ifdef CONFIG_CPU_HAS_LASX",
            "#define XOR_FUNC_NAME(nr) __xor_lasx_##nr",
            "#include \"xor_template.c\"",
        ],
    },
    LoongarchXorFile {
        rust_module: Some("xor_simd_glue"),
        linux_source: "vendor/linux/lib/raid/xor/loongarch/xor_simd_glue.c",
        required_markers: &[
            "#include <asm/fpu.h>",
            "#include \"xor_impl.h\"",
            "#include \"xor_arch.h\"",
            "#include \"xor_simd.h\"",
            "#define MAKE_XOR_GLUES(flavor)",
            "kernel_fpu_begin();",
            "kernel_fpu_end();",
            "MAKE_XOR_GLUES(lsx);",
            "MAKE_XOR_GLUES(lasx);",
        ],
    },
    LoongarchXorFile {
        rust_module: Some("xor_template"),
        linux_source: "vendor/linux/lib/raid/xor/loongarch/xor_template.c",
        required_markers: &[
            "LINE_WIDTH",
            "XOR_FUNC_NAME(nr)",
            "LD_INOUT_LINE(buf)",
            "LD_AND_XOR_LINE(buf)",
            "ST_LINE(buf)",
            "void XOR_FUNC_NAME(2)(unsigned long bytes",
            "void XOR_FUNC_NAME(5)(unsigned long bytes",
            "v1 += LINE_WIDTH / sizeof(unsigned long);",
            "} while (--lines > 0);",
        ],
    },
];

pub const LOONGARCH_XOR_HEADERS: &[LoongarchXorFile] = &[
    LoongarchXorFile {
        rust_module: None,
        linux_source: "vendor/linux/lib/raid/xor/loongarch/xor_arch.h",
        required_markers: &[
            "#include <asm/cpu-features.h>",
            "extern struct xor_block_template xor_block_lsx;",
            "extern struct xor_block_template xor_block_lasx;",
            "static __always_inline void __init arch_xor_init(void)",
            "xor_register(&xor_block_8regs);",
            "xor_register(&xor_block_8regs_p);",
            "xor_register(&xor_block_32regs);",
            "xor_register(&xor_block_32regs_p);",
            "if (cpu_has_lsx)",
            "if (cpu_has_lasx)",
        ],
    },
    LoongarchXorFile {
        rust_module: None,
        linux_source: "vendor/linux/lib/raid/xor/loongarch/xor_simd.h",
        required_markers: &[
            "#ifndef __LOONGARCH_LIB_XOR_SIMD_H",
            "#define __LOONGARCH_LIB_XOR_SIMD_H",
            "#ifdef CONFIG_CPU_HAS_LSX",
            "__xor_lsx_2",
            "__xor_lsx_3",
            "__xor_lsx_4",
            "__xor_lsx_5",
            "#ifdef CONFIG_CPU_HAS_LASX",
            "__xor_lasx_2",
            "__xor_lasx_3",
            "__xor_lasx_4",
            "__xor_lasx_5",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_impl_inventory {
        ($(($module:literal, $source:literal, [$($marker:literal),+ $(,)?])),+ $(,)?) => {
            #[test]
            fn implementation_inventory_matches_complete_children_and_linux_sources() {
                let mut idx = 0usize;
                $(
                    let rust = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/lib/raid/xor/loongarch/",
                        $module,
                        ".rs"
                    ));
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let entry = LOONGARCH_XOR_IMPLEMENTATIONS[idx];

                    assert_eq!(entry.rust_module, Some($module));
                    assert_eq!(entry.linux_source, $source);
                    assert!(rust.contains("//! linux-parity: complete"), "{}", $module);
                    assert!(
                        rust.contains(concat!("//! linux-source: ", $source)),
                        "{} missing source tag {}",
                        $module,
                        $source
                    );
                    assert!(linux.contains("SPDX-License-Identifier: GPL-2.0-or-later"));
                    $(
                        assert!(linux.contains($marker), "{} missing {}", $source, $marker);
                    )+
                    for marker in entry.required_markers {
                        assert!(linux.contains(marker), "{} missing {}", $source, marker);
                    }

                    idx += 1;
                )+
                assert_eq!(idx, LOONGARCH_XOR_IMPLEMENTATIONS.len());
            }
        };
    }

    macro_rules! assert_header_inventory {
        ($(($source:literal, [$($marker:literal),+ $(,)?])),+ $(,)?) => {
            #[test]
            fn header_inventory_matches_linux_sources() {
                let mut idx = 0usize;
                $(
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let entry = LOONGARCH_XOR_HEADERS[idx];

                    assert_eq!(entry.rust_module, None);
                    assert_eq!(entry.linux_source, $source);
                    assert!(linux.contains("SPDX-License-Identifier: GPL-2.0-or-later"));
                    $(
                        assert!(linux.contains($marker), "{} missing {}", $source, $marker);
                    )+
                    for marker in entry.required_markers {
                        assert!(linux.contains(marker), "{} missing {}", $source, marker);
                    }

                    idx += 1;
                )+
                assert_eq!(idx, LOONGARCH_XOR_HEADERS.len());
            }
        };
    }

    assert_impl_inventory!(
        (
            "xor_simd",
            "vendor/linux/lib/raid/xor/loongarch/xor_simd.c",
            [
                "#define LINE_WIDTH 64",
                "#define XOR_FUNC_NAME(nr) __xor_lsx_##nr",
                "#define XOR_FUNC_NAME(nr) __xor_lasx_##nr"
            ]
        ),
        (
            "xor_simd_glue",
            "vendor/linux/lib/raid/xor/loongarch/xor_simd_glue.c",
            [
                "#define MAKE_XOR_GLUES(flavor)",
                "kernel_fpu_begin();",
                "kernel_fpu_end();"
            ]
        ),
        (
            "xor_template",
            "vendor/linux/lib/raid/xor/loongarch/xor_template.c",
            [
                "void XOR_FUNC_NAME(2)(unsigned long bytes",
                "void XOR_FUNC_NAME(3)(unsigned long bytes",
                "void XOR_FUNC_NAME(4)(unsigned long bytes",
                "void XOR_FUNC_NAME(5)(unsigned long bytes"
            ]
        ),
    );

    assert_header_inventory!(
        (
            "vendor/linux/lib/raid/xor/loongarch/xor_arch.h",
            [
                "xor_register(&xor_block_8regs);",
                "xor_register(&xor_block_8regs_p);",
                "xor_register(&xor_block_32regs);",
                "xor_register(&xor_block_32regs_p);",
                "xor_register(&xor_block_lsx);",
                "xor_register(&xor_block_lasx);"
            ]
        ),
        (
            "vendor/linux/lib/raid/xor/loongarch/xor_simd.h",
            [
                "void __xor_lsx_2(unsigned long bytes",
                "void __xor_lsx_5(unsigned long bytes",
                "void __xor_lasx_2(unsigned long bytes",
                "void __xor_lasx_5(unsigned long bytes"
            ]
        ),
    );

    #[test]
    fn aggregate_exposes_child_contracts() {
        assert_eq!(xor_simd::LINE_WIDTH, 64);
        assert_eq!(
            xor_simd::XOR_SIMD_FLAVORS,
            [xor_simd::LSX_FLAVOR, xor_simd::LASX_FLAVOR]
        );
        assert_eq!(
            xor_simd_glue::enabled_templates(true, true),
            [xor_simd_glue::LSX_TEMPLATE, xor_simd_glue::LASX_TEMPLATE]
        );
        assert_eq!(xor_template::EXPECTED_DEFINES.len(), 5);
        assert_eq!(
            xor_template::words_per_line(xor_simd::LINE_WIDTH),
            xor_simd::LINE_WIDTH / core::mem::size_of::<usize>()
        );
    }
}
