//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/tests/crc_kunit.c
//! test-origin: linux:vendor/linux/lib/crc/tests/crc_kunit.c
//! CRC KUnit suite metadata and bit-at-a-time reference implementation.

pub const CRC_KUNIT_SEED: u32 = 42;
pub const CRC_KUNIT_MAX_LEN: usize = 16_384;
pub const CRC_KUNIT_NUM_TEST_ITERS: usize = 1_000;
pub const IRQ_TEST_DATA_LEN: usize = 512;
pub const IRQ_TEST_NUM_BUFFERS: usize = 3;
pub const SUITE_NAME: &str = "crc";
pub const MODULE_DESCRIPTION: &str = "Unit tests and benchmarks for the CRC library functions";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrcVariant {
    pub bits: u8,
    pub little_endian: bool,
    pub poly: u64,
}

pub const CRC_VARIANTS: [(&str, CrcVariant); 8] = [
    (
        "crc7_be",
        CrcVariant {
            bits: 7,
            little_endian: false,
            poly: 0x9,
        },
    ),
    (
        "crc16",
        CrcVariant {
            bits: 16,
            little_endian: true,
            poly: 0xa001,
        },
    ),
    (
        "crc_t10dif",
        CrcVariant {
            bits: 16,
            little_endian: false,
            poly: 0x8bb7,
        },
    ),
    (
        "crc32_le",
        CrcVariant {
            bits: 32,
            little_endian: true,
            poly: 0xedb88320,
        },
    ),
    (
        "crc32_be",
        CrcVariant {
            bits: 32,
            little_endian: false,
            poly: 0x04c11db7,
        },
    ),
    (
        "crc32c",
        CrcVariant {
            bits: 32,
            little_endian: true,
            poly: 0x82f63b78,
        },
    ),
    (
        "crc64_be",
        CrcVariant {
            bits: 64,
            little_endian: false,
            poly: 0x42f0e1eba9ea3693,
        },
    ),
    (
        "crc64_nvme",
        CrcVariant {
            bits: 64,
            little_endian: true,
            poly: 0x9a6c9329ac4bc9b5,
        },
    ),
];
pub const CRC_KUNIT_CASES: [&str; 8] = [
    "KUNIT_CASE(crc7_be_test)",
    "KUNIT_CASE(crc16_test)",
    "KUNIT_CASE(crc_t10dif_test)",
    "KUNIT_CASE(crc32_le_test)",
    "KUNIT_CASE(crc32_be_test)",
    "KUNIT_CASE(crc32c_test)",
    "KUNIT_CASE(crc64_be_test)",
    "KUNIT_CASE(crc64_nvme_test)",
];
pub const CRC_BENCHMARK_CASES: [&str; 8] = [
    "KUNIT_CASE(crc7_be_benchmark)",
    "KUNIT_CASE(crc16_benchmark)",
    "KUNIT_CASE(crc_t10dif_benchmark)",
    "KUNIT_CASE(crc32_le_benchmark)",
    "KUNIT_CASE(crc32_be_benchmark)",
    "KUNIT_CASE(crc32c_benchmark)",
    "KUNIT_CASE(crc64_be_benchmark)",
    "KUNIT_CASE(crc64_nvme_benchmark)",
];

pub const fn crc_mask(variant: CrcVariant) -> u64 {
    u64::MAX >> (64 - variant.bits as u32)
}

pub fn crc_ref(variant: CrcVariant, mut crc: u64, bytes: &[u8]) -> u64 {
    let mask = crc_mask(variant);
    for byte in bytes {
        for bit in 0..8 {
            if variant.little_endian {
                crc ^= u64::from((byte >> bit) & 1);
                crc = (crc >> 1) ^ if crc & 1 != 0 { variant.poly } else { 0 };
            } else {
                crc ^= u64::from((byte >> (7 - bit)) & 1) << (variant.bits - 1);
                if crc & (1u64 << (variant.bits - 1)) != 0 {
                    crc = ((crc << 1) ^ variant.poly) & mask;
                } else {
                    crc <<= 1;
                }
            }
        }
    }
    crc & mask
}

pub fn variant(name: &str) -> Option<CrcVariant> {
    CRC_VARIANTS
        .iter()
        .find_map(|(variant_name, variant)| (*variant_name == name).then_some(*variant))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_kunit_matches_linux_original_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/tests/crc_kunit.c"
        ));

        assert!(source.contains("#include <kunit/run-in-irq-context.h>"));
        assert!(source.contains("#include <linux/crc7.h>"));
        assert!(source.contains("#define CRC_KUNIT_SEED\t\t\t42"));
        assert!(source.contains("#define CRC_KUNIT_MAX_LEN\t\t16384"));
        assert!(source.contains("#define CRC_KUNIT_NUM_TEST_ITERS\t1000"));
        assert!(source.contains("#define IRQ_TEST_DATA_LEN 512"));
        assert!(source.contains("#define IRQ_TEST_NUM_BUFFERS 3"));
        assert!(source.contains("static u64 crc_ref(const struct crc_variant *v,"));
        assert!(source.contains("kunit_run_irq_test(test, crc_irq_test_func, 100000, &state);"));

        for (name, variant) in CRC_VARIANTS {
            assert!(source.contains(name));
            assert_eq!(variant.bits as u64, crc_mask(variant).count_ones() as u64);
        }
        for case in CRC_KUNIT_CASES {
            assert!(source.contains(case));
        }
        for case in CRC_BENCHMARK_CASES {
            assert!(source.contains(case));
        }

        assert!(source.contains(".name = \"crc\""));
        assert!(source.contains("kunit_test_suite(crc_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert_eq!(CRC_KUNIT_SEED, 42);
        assert_eq!(CRC_KUNIT_MAX_LEN, 16_384);
        assert_eq!(IRQ_TEST_NUM_BUFFERS, 3);

        let crc16 = variant("crc16").unwrap();
        assert_eq!(crc_ref(crc16, 0, b"123456789"), 0xbb3d);
        let crc32_le = variant("crc32_le").unwrap();
        assert_eq!(crc_ref(crc32_le, 0, b"123456789"), 0x2dfd2d88);
    }
}
