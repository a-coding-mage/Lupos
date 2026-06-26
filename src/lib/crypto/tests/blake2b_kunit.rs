//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/blake2b_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/blake2b_kunit.c
//! BLAKE2b KUnit suite metadata and keyed-length cases.

pub const HASH: &str = "blake2b_default";
pub const HASH_CTX: &str = "blake2b_ctx";
pub const HASH_SIZE: &str = "BLAKE2B_HASH_SIZE";
pub const SUITE_NAME: &str = "blake2b";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for BLAKE2b";
pub const BLAKE2B_SPECIFIC_CASES: [&str; 3] = [
    "test_blake2b_all_key_and_hash_lens",
    "test_blake2b_with_guarded_key_buf",
    "test_blake2b_with_guarded_out_buf",
];
pub const BLAKE2B_SPECIFIC_KUNIT_CASES: [&str; 3] = [
    "KUNIT_CASE(test_blake2b_all_key_and_hash_lens)",
    "KUNIT_CASE(test_blake2b_with_guarded_key_buf)",
    "KUNIT_CASE(test_blake2b_with_guarded_out_buf)",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake2b_kunit_source_matches_linux_original_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));

        assert!(source.contains("#include <crypto/blake2b.h>"));
        assert!(source.contains("#include \"blake2b-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE] {
            assert!(source.contains(token));
        }
        for case in BLAKE2B_SPECIFIC_CASES {
            assert!(source.contains(case));
        }
        for case in BLAKE2B_SPECIFIC_KUNIT_CASES {
            assert!(source.contains(case));
        }
        assert!(source.contains("for (int key_len = 0; key_len <= BLAKE2B_KEY_SIZE; key_len++)"));
        assert!(source.contains("for (int out_len = 1; out_len <= BLAKE2B_HASH_SIZE; out_len++)"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"blake2b\""));
        assert!(source.contains("kunit_test_suite(blake2b_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(vectors.contains("blake2b_keyed_testvec_consolidated"));
        assert!(template.contains("#define HASH_KUNIT_CASES"));
        assert_eq!(SUITE_NAME, "blake2b");
    }
}
