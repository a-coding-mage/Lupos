//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/sha3_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/sha3_kunit.c
//! SHA3 and SHAKE KUnit suite metadata and original test cases.

pub const HASH: &str = "sha3_256";
pub const HASH_CTX: &str = "sha3_ctx";
pub const HASH_SIZE: &str = "SHA3_256_DIGEST_SIZE";
pub const SUITE_NAME: &str = "sha3";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for SHA3";
pub const SHA3_BASIC_CASES: [&str; 6] = [
    "test_sha3_224_basic",
    "test_sha3_256_basic",
    "test_sha3_384_basic",
    "test_sha3_512_basic",
    "test_shake128_basic",
    "test_shake256_basic",
];
pub const SHAKE_CASES: [&str; 5] = [
    "test_shake128_nist",
    "test_shake256_nist",
    "test_shake_all_lens_up_to_4096",
    "test_shake_multiple_squeezes",
    "test_shake_with_guarded_bufs",
];
pub const SHA3_BASIC_KUNIT_CASES: [&str; 6] = [
    "KUNIT_CASE(test_sha3_224_basic)",
    "KUNIT_CASE(test_sha3_256_basic)",
    "KUNIT_CASE(test_sha3_384_basic)",
    "KUNIT_CASE(test_sha3_512_basic)",
    "KUNIT_CASE(test_shake128_basic)",
    "KUNIT_CASE(test_shake256_basic)",
];
pub const SHAKE_KUNIT_CASES: [&str; 5] = [
    "KUNIT_CASE(test_shake128_nist)",
    "KUNIT_CASE(test_shake256_nist)",
    "KUNIT_CASE(test_shake_all_lens_up_to_4096)",
    "KUNIT_CASE(test_shake_multiple_squeezes)",
    "KUNIT_CASE(test_shake_with_guarded_bufs)",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha3_kunit_source_matches_linux_original_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha3_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha3-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));

        assert!(source.contains("#include <crypto/sha3.h>"));
        assert!(source.contains("#include \"sha3-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE] {
            assert!(source.contains(token));
        }
        for case in SHA3_BASIC_CASES {
            assert!(source.contains(case));
        }
        for case in SHAKE_CASES {
            assert!(source.contains(case));
        }
        for case in SHA3_BASIC_KUNIT_CASES {
            assert!(source.contains(case));
        }
        for case in SHAKE_KUNIT_CASES {
            assert!(source.contains(case));
        }
        assert!(source.contains("static const u8 test_sha3_sample[]"));
        assert!(source.contains("test_nist_1600_sample"));
        assert!(source.contains("shake128_testvec_consolidated"));
        assert!(source.contains("shake256_testvec_consolidated"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"sha3\""));
        assert!(source.contains("kunit_test_suite(sha3_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(vectors.contains("hash_testvecs"));
        assert!(vectors.contains("shake128_testvec_consolidated"));
        assert!(vectors.contains("shake256_testvec_consolidated"));
        assert!(template.contains("#define HASH_KUNIT_CASES"));
        assert_eq!(SUITE_NAME, "sha3");
    }
}
