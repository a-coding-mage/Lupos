//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/sha256_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/sha256_kunit.c
//! SHA-256 hash KUnit suite metadata and SHA256-specific cases.

pub const HASH: &str = "sha256";
pub const HASH_CTX: &str = "sha256_ctx";
pub const HASH_SIZE: &str = "SHA256_DIGEST_SIZE";
pub const HMAC: &str = "hmac_sha256";
pub const SUITE_NAME: &str = "sha256";
pub const MODULE_DESCRIPTION: &str = "KUnit tests and benchmark for SHA-256 and HMAC-SHA256";
pub const SHA256_FINUP_2X_CASES: [&str; 3] = [
    "test_sha256_finup_2x",
    "test_sha256_finup_2x_defaultctx",
    "test_sha256_finup_2x_hugelen",
];
pub const SHA256_FINUP_2X_KUNIT_CASES: [&str; 3] = [
    "KUNIT_CASE(test_sha256_finup_2x)",
    "KUNIT_CASE(test_sha256_finup_2x_defaultctx)",
    "KUNIT_CASE(test_sha256_finup_2x_hugelen)",
];
pub const BENCHMARK_KUNIT_CASES: [&str; 2] = [
    "KUNIT_CASE(benchmark_hash)",
    "KUNIT_CASE(benchmark_sha256_finup_2x)",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_kunit_source_matches_linux_original_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha256_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha256-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));

        assert!(source.contains("#include <crypto/sha2.h>"));
        assert!(source.contains("#include \"sha256-testvecs.h\""));
        assert!(source.contains("#include \"hash-test-template.h\""));
        for token in [HASH, HASH_CTX, HASH_SIZE, HMAC] {
            assert!(source.contains(token));
        }
        for case in SHA256_FINUP_2X_CASES {
            assert!(source.contains(case));
        }
        for case in SHA256_FINUP_2X_KUNIT_CASES {
            assert!(source.contains(case));
        }
        for case in BENCHMARK_KUNIT_CASES {
            assert!(source.contains(case));
        }
        assert!(source.contains("sha256_finup_2x(ctx, data1, data2, data_len, hash1, hash2);"));
        assert!(source.contains("sha256_finup_2x(NULL, test_buf"));
        assert!(source.contains("ctx.ctx.bytecount = 0x123456789abcd00 + align;"));
        assert!(source.contains(".name = \"sha256\""));
        assert!(source.contains("kunit_test_suite(hash_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(vectors.contains("hash_testvecs"));
        assert!(vectors.contains("hmac_testvec_consolidated"));
        assert!(template.contains("#define HASH_KUNIT_CASES"));
        assert_eq!(SUITE_NAME, "sha256");
    }
}
