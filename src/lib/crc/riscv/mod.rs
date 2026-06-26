//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/riscv
//! test-origin: linux:vendor/linux/lib/crc/riscv
//! RISC-V CRC carry-less-multiply source contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrcClmulSource {
    pub linux_source: &'static str,
    pub crc_type: &'static str,
    pub lsb_crc: bool,
    pub symbol: &'static str,
    pub return_type: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrcClmulModule {
    pub rust_module: &'static str,
    pub linux_source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrcRiscvHeader {
    pub linux_source: &'static str,
    pub required_markers: &'static [&'static str],
}

pub mod crc16_msb;
pub mod crc32_lsb;
pub mod crc32_msb;
pub mod crc64_lsb;
pub mod crc64_msb;

pub const CRC_CLMUL_MODULES: &[CrcClmulModule] = &[
    CrcClmulModule {
        rust_module: "crc16_msb",
        linux_source: "vendor/linux/lib/crc/riscv/crc16_msb.c",
    },
    CrcClmulModule {
        rust_module: "crc32_lsb",
        linux_source: "vendor/linux/lib/crc/riscv/crc32_lsb.c",
    },
    CrcClmulModule {
        rust_module: "crc32_msb",
        linux_source: "vendor/linux/lib/crc/riscv/crc32_msb.c",
    },
    CrcClmulModule {
        rust_module: "crc64_lsb",
        linux_source: "vendor/linux/lib/crc/riscv/crc64_lsb.c",
    },
    CrcClmulModule {
        rust_module: "crc64_msb",
        linux_source: "vendor/linux/lib/crc/riscv/crc64_msb.c",
    },
];

pub const CRC_CLMUL_SOURCES: &[CrcClmulSource] = &[
    crc16_msb::SOURCE,
    crc32_lsb::SOURCE,
    crc32_msb::SOURCE,
    crc64_lsb::SOURCE,
    crc64_msb::SOURCE,
];

pub const CRC_RISCV_HEADERS: &[CrcRiscvHeader] = &[
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc-clmul.h",
        required_markers: &[
            "crc16_msb_clmul",
            "crc32_msb_clmul",
            "crc32_lsb_clmul",
            "crc64_msb_clmul",
            "crc64_lsb_clmul",
            "const struct crc_clmul_consts *consts",
        ],
    },
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc-clmul-consts.h",
        required_markers: &[
            "struct crc_clmul_consts",
            "crc16_msb_0x8bb7_consts",
            "crc32_msb_0x04c11db7_consts",
            "crc32_lsb_0xedb88320_consts",
            "crc32_lsb_0x82f63b78_consts",
            "crc64_msb_0x42f0e1eba9ea3693_consts",
            "crc64_lsb_0x9a6c9329ac4bc9b5_consts",
        ],
    },
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc-clmul-template.h",
        required_markers: &[
            "crc_t",
            "LSB_CRC",
            "CRC_BITS",
            "clmul(",
            "clmulh(",
            "clmulr(",
            ".option arch,+zbc",
            "crc_clmul_prep",
            "crc_clmul_long",
            "crc_clmul(",
        ],
    },
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc-t10dif.h",
        required_markers: &[
            "crc_t10dif_arch",
            "riscv_has_extension_likely(RISCV_ISA_EXT_ZBC)",
            "crc16_msb_clmul",
            "crc16_msb_0x8bb7_consts",
            "crc_t10dif_generic",
        ],
    },
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc32.h",
        required_markers: &[
            "crc32_le_arch",
            "crc32_be_arch",
            "crc32c_arch",
            "crc32_optimizations_arch",
            "crc32_lsb_clmul",
            "crc32_msb_clmul",
            "CRC32_LE_OPTIMIZATION",
            "CRC32_BE_OPTIMIZATION",
            "CRC32C_OPTIMIZATION",
        ],
    },
    CrcRiscvHeader {
        linux_source: "vendor/linux/lib/crc/riscv/crc64.h",
        required_markers: &[
            "crc64_be_arch",
            "crc64_nvme_arch",
            "crc64_msb_clmul",
            "crc64_lsb_clmul",
            "crc64_be_generic",
            "crc64_nvme_generic",
        ],
    },
];

#[cfg(test)]
pub(crate) fn assert_crc_clmul_source(source: &str, contract: CrcClmulSource) {
    assert!(source.contains("SPDX-License-Identifier: GPL-2.0-or-later"));
    assert!(source.contains("#include \"crc-clmul.h\""));
    assert!(
        source.contains(contract.crc_type),
        "{} missing {}",
        contract.linux_source,
        contract.crc_type
    );
    let lsb = if contract.lsb_crc {
        "#define LSB_CRC 1"
    } else {
        "#define LSB_CRC 0"
    };
    assert!(
        source.contains(lsb),
        "{} missing {}",
        contract.linux_source,
        lsb
    );
    assert!(source.contains("#include \"crc-clmul-template.h\""));
    assert!(
        source.contains(contract.symbol),
        "{} missing {}",
        contract.linux_source,
        contract.symbol
    );
    assert!(
        source.contains(contract.return_type),
        "{} missing return type {}",
        contract.linux_source,
        contract.return_type
    );
    assert!(source.contains("return crc_clmul(crc, p, len, consts);"));
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_clmul_inventory {
        ($(($module:literal, $source:literal)),+ $(,)?) => {
            #[test]
            fn clmul_inventory_matches_complete_children_and_vendor_sources() {
                let mut idx = 0usize;
                $(
                    let rust = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/lib/crc/riscv/",
                        $module,
                        ".rs"
                    ));
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let declared = CRC_CLMUL_MODULES[idx];
                    let contract = CRC_CLMUL_SOURCES[idx];

                    assert_eq!(declared.rust_module, $module);
                    assert_eq!(declared.linux_source, $source);
                    assert_eq!(contract.linux_source, $source);
                    assert!(rust.contains("//! linux-parity: complete"), "{}", $module);
                    assert!(
                        rust.contains(concat!("//! linux-source: ", $source)),
                        "{} missing source tag {}",
                        $module,
                        $source
                    );
                    assert_crc_clmul_source(linux, contract);

                    idx += 1;
                )+
                assert_eq!(idx, CRC_CLMUL_MODULES.len());
                assert_eq!(idx, CRC_CLMUL_SOURCES.len());
            }
        };
    }

    macro_rules! assert_header_inventory {
        ($(($source:literal, [$($marker:literal),+ $(,)?])),+ $(,)?) => {
            #[test]
            fn shared_headers_match_riscv_crc_contracts() {
                let mut idx = 0usize;
                $(
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let header = CRC_RISCV_HEADERS[idx];

                    assert_eq!(header.linux_source, $source);
                    assert!(linux.contains("SPDX-License-Identifier: GPL-2.0-or-later"));
                    $(
                        assert!(linux.contains($marker), "{} missing {}", $source, $marker);
                    )+
                    for marker in header.required_markers {
                        assert!(linux.contains(marker), "{} missing {}", $source, marker);
                    }

                    idx += 1;
                )+
                assert_eq!(idx, CRC_RISCV_HEADERS.len());
            }
        };
    }

    assert_clmul_inventory!(
        ("crc16_msb", "vendor/linux/lib/crc/riscv/crc16_msb.c"),
        ("crc32_lsb", "vendor/linux/lib/crc/riscv/crc32_lsb.c"),
        ("crc32_msb", "vendor/linux/lib/crc/riscv/crc32_msb.c"),
        ("crc64_lsb", "vendor/linux/lib/crc/riscv/crc64_lsb.c"),
        ("crc64_msb", "vendor/linux/lib/crc/riscv/crc64_msb.c"),
    );

    assert_header_inventory!(
        (
            "vendor/linux/lib/crc/riscv/crc-clmul.h",
            [
                "crc16_msb_clmul",
                "crc32_msb_clmul",
                "crc32_lsb_clmul",
                "crc64_msb_clmul",
                "crc64_lsb_clmul"
            ]
        ),
        (
            "vendor/linux/lib/crc/riscv/crc-clmul-consts.h",
            [
                "crc16_msb_0x8bb7_consts",
                "crc32_msb_0x04c11db7_consts",
                "crc32_lsb_0xedb88320_consts",
                "crc32_lsb_0x82f63b78_consts",
                "crc64_msb_0x42f0e1eba9ea3693_consts",
                "crc64_lsb_0x9a6c9329ac4bc9b5_consts"
            ]
        ),
        (
            "vendor/linux/lib/crc/riscv/crc-clmul-template.h",
            [
                "clmul(",
                "clmulh(",
                "clmulr(",
                ".option arch,+zbc",
                "crc_clmul("
            ]
        ),
        (
            "vendor/linux/lib/crc/riscv/crc-t10dif.h",
            ["crc_t10dif_arch", "crc16_msb_clmul", "crc_t10dif_generic"]
        ),
        (
            "vendor/linux/lib/crc/riscv/crc32.h",
            [
                "crc32_le_arch",
                "crc32_be_arch",
                "crc32c_arch",
                "crc32_optimizations_arch"
            ]
        ),
        (
            "vendor/linux/lib/crc/riscv/crc64.h",
            ["crc64_be_arch", "crc64_nvme_arch", "crc64_be_generic"]
        ),
    );
}
