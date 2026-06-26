//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/sm3_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/sm3_kunit.c
//! SM3 hash KUnit suite metadata.

pub const HASH: &str = "sm3";
pub const HASH_CTX: &str = "sm3_ctx";
pub const HASH_SIZE: &str = "SM3_DIGEST_SIZE";
pub const HASH_INIT: &str = "sm3_init";
pub const HASH_UPDATE: &str = "sm3_update";
pub const HASH_FINAL: &str = "sm3_final";
pub const SUITE_NAME: &str = "sm3";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for SM3";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm3_kunit_source_matches_linux_template_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sm3_kunit.c"
        ));
        assert!(source.contains("#include <crypto/sm3.h>"));
        assert!(source.contains("#include \"sm3-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [
            HASH,
            HASH_CTX,
            HASH_SIZE,
            HASH_INIT,
            HASH_UPDATE,
            HASH_FINAL,
        ] {
            assert!(source.contains(token));
        }
        assert!(source.contains("HASH_KUNIT_CASES"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"sm3\""));
        assert!(source.contains("kunit_test_suite(sm3_test_suite);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"KUnit tests and benchmark for SM3\")"));
        assert_eq!(SUITE_NAME, "sm3");
        assert_eq!(MODULE_DESCRIPTION, "KUnit tests and benchmark for SM3");
    }
}
