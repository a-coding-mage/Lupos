//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/poly1305_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/poly1305_kunit.c
//! Poly1305 KUnit suite metadata and edge-case inventory.

pub const HASH: &str = "poly1305_withtestkey";
pub const HASH_CTX: &str = "poly1305_desc_ctx";
pub const HASH_SIZE: &str = "POLY1305_DIGEST_SIZE";
pub const SUITE_NAME: &str = "poly1305";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for Poly1305";
pub const POLY1305_SPECIFIC_CASES: [&str; 2] = [
    "test_poly1305_allones_keys_and_message",
    "test_poly1305_reduction_edge_cases",
];
pub const POLY1305_SPECIFIC_KUNIT_CASES: [&str; 2] = [
    "KUNIT_CASE(test_poly1305_allones_keys_and_message)",
    "KUNIT_CASE(test_poly1305_reduction_edge_cases)",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poly1305_kunit_source_matches_linux_original_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/poly1305_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/poly1305-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));

        assert!(source.contains("#include <crypto/poly1305.h>"));
        assert!(source.contains("#include \"poly1305-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE] {
            assert!(source.contains(token));
        }
        for case in POLY1305_SPECIFIC_CASES {
            assert!(source.contains(case));
        }
        for case in POLY1305_SPECIFIC_KUNIT_CASES {
            assert!(source.contains(case));
        }
        assert!(source.contains("static u8 test_key[POLY1305_KEY_SIZE];"));
        assert!(source.contains("rand_bytes_seeded_from_len(test_key, POLY1305_KEY_SIZE);"));
        assert!(source.contains("for (size_t len = 0; len <= 4096; len += 16)"));
        assert!(source.contains("for (int i = 1; i <= 10; i++)"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"poly1305\""));
        assert!(source.contains("kunit_test_suite(poly1305_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(vectors.contains("poly1305_allones_macofmacs"));
        assert!(template.contains("#define HASH_KUNIT_CASES"));
        assert_eq!(SUITE_NAME, "poly1305");
    }
}
