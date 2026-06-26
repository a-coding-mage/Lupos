//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/md5_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/md5_kunit.c
//! MD5 hash KUnit suite metadata.

pub const HASH: &str = "md5";
pub const HASH_CTX: &str = "md5_ctx";
pub const HASH_SIZE: &str = "MD5_DIGEST_SIZE";
pub const HMAC: &str = "hmac_md5";
pub const SUITE_NAME: &str = "md5";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for MD5 and HMAC-MD5";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_kunit_source_matches_linux_template_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/md5_kunit.c"
        ));
        assert!(source.contains("#include <crypto/md5.h>"));
        assert!(source.contains("#include \"md5-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE, HMAC] {
            assert!(source.contains(token));
        }
        assert!(source.contains("HASH_KUNIT_CASES"));
        assert!(source.contains("KUNIT_CASE(benchmark_hash)"));
        assert!(source.contains(".name = \"md5\""));
        assert!(source.contains("kunit_test_suite(hash_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert_eq!(SUITE_NAME, "md5");
    }
}
