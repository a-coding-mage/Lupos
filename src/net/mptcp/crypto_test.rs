//! linux-parity: complete
//! linux-source: vendor/linux/net/mptcp/crypto_test.c
//! test-origin: linux:vendor/linux/net/mptcp/crypto_test.c
//! KUnit-style MPTCP crypto test vectors.

use super::crypto::mptcp_crypto_hmac_sha;

pub const MODULE_DESCRIPTION: &str = "KUnit tests for MPTCP Crypto";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoTestCase {
    pub key: &'static [u8; 16],
    pub msg: &'static [u8; 8],
    pub result_hex: &'static str,
}

pub const TESTS: [CryptoTestCase; 3] = [
    CryptoTestCase {
        key: b"0b0b0b0b0b0b0b0b",
        msg: b"48692054",
        result_hex: "8385e24fb4235ac37556b6b886db106284a1da671699f46db1f235ec622dcafa",
    },
    CryptoTestCase {
        key: b"aaaaaaaaaaaaaaaa",
        msg: b"dddddddd",
        result_hex: "2c5e219164ff1dca1c4a92318d847bb6b9d44492984e1eb71aff9022f71046e9",
    },
    CryptoTestCase {
        key: b"0102030405060708",
        msg: b"cdcdcdcd",
        result_hex: "e73b9ba9969969cefb04aa0d6df18ec2fcc075b6f23b4d8c4da736a5dbbc6e7d",
    },
];

pub fn mptcp_crypto_test_hmac(case: CryptoTestCase) -> [u8; 32] {
    let key1 = u64::from_be_bytes([
        case.key[0],
        case.key[1],
        case.key[2],
        case.key[3],
        case.key[4],
        case.key[5],
        case.key[6],
        case.key[7],
    ]);
    let key2 = u64::from_be_bytes([
        case.key[8],
        case.key[9],
        case.key[10],
        case.key[11],
        case.key[12],
        case.key[13],
        case.key[14],
        case.key[15],
    ]);
    mptcp_crypto_hmac_sha(key1, key2, case.msg)
}

pub fn bytes_to_lower_hex(bytes: &[u8; 32]) -> [u8; 64] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 32 {
        out[i * 2] = HEX[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(bytes[i] & 0x0f) as usize];
        i += 1;
    }
    out
}

pub fn mptcp_crypto_test_basic() -> bool {
    let mut i = 0;
    while i < TESTS.len() {
        let hmac = mptcp_crypto_test_hmac(TESTS[i]);
        if bytes_to_lower_hex(&hmac) != TESTS[i].result_hex.as_bytes() {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mptcp_crypto_test_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mptcp/crypto_test.c"
        ));
        assert!(source.contains("struct test_case"));
        assert!(source.contains(".key = \"0b0b0b0b0b0b0b0b\""));
        assert!(source.contains(".msg = \"48692054\""));
        assert!(
            source.contains("8385e24fb4235ac37556b6b886db106284a1da671699f46db1f235ec622dcafa")
        );
        assert!(source.contains("mptcp_crypto_test_basic"));
        assert!(source.contains("key1 = be64_to_cpu(*((__be64 *)&tests[i].key[0]));"));
        assert!(source.contains("key2 = be64_to_cpu(*((__be64 *)&tests[i].key[8]));"));
        assert!(source.contains("put_unaligned_be32(nonce1, &msg[0]);"));
        assert!(source.contains("mptcp_crypto_hmac_sha(key1, key2, msg, 8, hmac);"));
        assert!(source.contains("sprintf(&hmac_hex[j << 1], \"%02x\", hmac[j] & 0xff);"));
        assert!(source.contains("KUNIT_EXPECT_STREQ(test, &hmac_hex[0], tests[i].result);"));
        assert!(source.contains(".name = \"mptcp-crypto\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"KUnit tests for MPTCP Crypto\");"));

        assert_eq!(TESTS.len(), 3);
    }

    #[test]
    fn mptcp_crypto_vectors_match_linux_expected_hex() {
        assert!(mptcp_crypto_test_basic());
        for case in TESTS {
            let hmac = mptcp_crypto_test_hmac(case);
            assert_eq!(
                bytes_to_lower_hex(&hmac).as_slice(),
                case.result_hex.as_bytes()
            );
        }
    }
}
