//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/sha384_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/sha384_kunit.c
//! SHA-384 hash KUnit suite metadata.

pub const HASH: &str = "sha384";
pub const HASH_CTX: &str = "sha384_ctx";
pub const HASH_SIZE: &str = "SHA384_DIGEST_SIZE";
pub const HMAC: &str = "hmac_sha384";
pub const SUITE_NAME: &str = "sha384";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for SHA-384 and HMAC-SHA384";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha384_kunit_source_matches_linux_template_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha384_kunit.c"
        ));
        assert!(source.contains("#include <crypto/sha2.h>"));
        assert!(source.contains("#include \"sha384-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE, HMAC] {
            assert!(source.contains(token));
        }
        assert!(source.contains("HASH_KUNIT_CASES"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"sha384\""));
        assert!(source.contains("kunit_test_suite(hash_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert_eq!(SUITE_NAME, "sha384");
    }
}
