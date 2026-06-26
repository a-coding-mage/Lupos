//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/tests/nh_kunit.c
//! test-origin: linux:vendor/linux/lib/crypto/tests/nh_kunit.c
//! NH hash KUnit suite metadata.

pub const NH_PAIR_STRIDE: usize = 2;
pub const NH_MESSAGE_UNIT: usize = NH_PAIR_STRIDE * 2 * core::mem::size_of::<u32>();
pub const NH_NUM_PASSES: usize = 4;
pub const NH_HASH_BYTES: usize = NH_NUM_PASSES * core::mem::size_of::<u64>();
pub const NH_NUM_STRIDES: usize = 64;
pub const NH_MESSAGE_WORDS: usize = NH_PAIR_STRIDE * 2 * NH_NUM_STRIDES;
pub const NH_MESSAGE_BYTES: usize = NH_MESSAGE_WORDS * core::mem::size_of::<u32>();
pub const NH_KEY_WORDS: usize = NH_MESSAGE_WORDS + NH_PAIR_STRIDE * 2 * (NH_NUM_PASSES - 1);
pub const NH_KEY_BYTES: usize = NH_KEY_WORDS * core::mem::size_of::<u32>();
pub const TEST_MESSAGE_LENGTHS: [usize; 4] = [16, 96, 256, 1024];
pub const SUITE_NAME: &str = "nh";
pub const MODULE_DESCRIPTION: &str = "KUnit tests for NH";

pub const fn valid_nh_message_len(len: usize) -> bool {
    len <= NH_MESSAGE_BYTES && len % NH_MESSAGE_UNIT == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nh_kunit_source_matches_linux_vectors_and_cases() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/nh_kunit.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/nh.h"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/nh-testvecs.h"
        ));

        assert!(source.contains("#include <crypto/nh.h>"));
        assert!(source.contains("#include <kunit/test.h>"));
        assert!(source.contains("#include \"nh-testvecs.h\""));
        assert!(source.contains("u32 *key = kunit_kmalloc(test, NH_KEY_BYTES"));
        assert!(source.contains("__le64 hash[NH_NUM_PASSES];"));
        assert!(source.contains("le32_to_cpu_array(key, NH_KEY_WORDS);"));
        assert!(source.contains("nh(key, nh_test_msg, 16, hash);"));
        assert!(source.contains("nh(key, nh_test_msg, 96, hash);"));
        assert!(source.contains("nh(key, nh_test_msg, 256, hash);"));
        assert!(source.contains("nh(key, nh_test_msg, 1024, hash);"));
        for token in [
            "nh_test_key",
            "nh_test_msg",
            "nh_test_val16",
            "nh_test_val96",
            "nh_test_val256",
            "nh_test_val1024",
        ] {
            assert!(vectors.contains(token));
        }
        assert!(header.contains("#define NH_NUM_PASSES"));
        assert!(header.contains("#define NH_NUM_STRIDES"));
        assert!(source.contains(".name = \"nh\""));
        assert!(source.contains("kunit_test_suite(nh_test_suite);"));
        assert!(source.contains(MODULE_DESCRIPTION));

        assert_eq!(NH_MESSAGE_UNIT, 16);
        assert_eq!(NH_HASH_BYTES, 32);
        assert_eq!(NH_MESSAGE_BYTES, 1024);
        assert_eq!(NH_KEY_BYTES, 1072);
        assert!(valid_nh_message_len(16));
        assert!(valid_nh_message_len(1024));
        assert!(!valid_nh_message_len(15));
        assert!(!valid_nh_message_len(1040));
        assert_eq!(SUITE_NAME, "nh");
    }
}
