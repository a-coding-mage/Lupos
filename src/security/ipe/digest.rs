//! linux-parity: complete
//! linux-source: vendor/linux/security/ipe/digest.c
//! test-origin: linux:vendor/linux/security/ipe/digest.c
//! Integrity Policy Enforcement digest parsing, comparison, and audit format.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EBADMSG, EINVAL};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestInfo {
    pub alg: String,
    pub digest: Vec<u8>,
}

impl DigestInfo {
    pub fn digest_len(&self) -> usize {
        self.digest.len()
    }
}

pub fn ipe_digest_parse(valstr: &str) -> Result<DigestInfo, i32> {
    let sep = valstr.find(':').ok_or(-EBADMSG)?;
    let alg = String::from(&valstr[..sep]);
    let raw_digest = &valstr[sep + 1..];

    let digest_len = (raw_digest.len() + 1) / 2;
    let digest = hex2bin(raw_digest.as_bytes(), digest_len)?;

    Ok(DigestInfo { alg, digest })
}

pub fn ipe_digest_eval(expected: &DigestInfo, digest: &DigestInfo) -> bool {
    expected.digest_len() == digest.digest_len()
        && expected.alg == digest.alg
        && expected.digest == digest.digest
}

pub fn ipe_digest_audit(info: &DigestInfo) -> String {
    let mut hex = String::with_capacity(info.digest.len() * 2);
    for byte in &info.digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("{}:{hex}", info.alg)
}

pub fn ipe_digest_free(_info: Option<DigestInfo>) {}

fn hex2bin(src: &[u8], count: usize) -> Result<Vec<u8>, i32> {
    let mut digest = Vec::with_capacity(count);
    for index in 0..count {
        let hi = hex_value(*src.get(index * 2).unwrap_or(&0)).ok_or(-EINVAL)?;
        let lo = hex_value(*src.get(index * 2 + 1).unwrap_or(&0)).ok_or(-EINVAL)?;
        digest.push((hi << 4) | lo);
    }
    Ok(digest)
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipe_digest_helpers_match_linux_source_contract() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/ipe/digest.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/ipe/digest.h"
        ));
        let policy_tests = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/ipe/policy_tests.c"
        ));

        assert!(source.contains("sep = strchr(valstr, ':');"));
        assert!(source.contains("info->digest_len = (raw_digest_len + 1) / 2;"));
        assert!(source.contains("rc = hex2bin(digest, raw_digest, info->digest_len);"));
        assert!(source.contains("kfree(alg);"));
        assert!(source.contains("kfree(digest);"));
        assert!(source.contains("kfree(info);"));
        assert!(source.contains("return (expected->digest_len == digest->digest_len) &&"));
        assert!(source.contains("audit_log_untrustedstring(ab, info->alg);"));
        assert!(source.contains("audit_log_n_hex(ab, info->digest, info->digest_len);"));
        assert!(header.contains("struct digest_info"));
        assert!(policy_tests.contains("\"old-style digest\""));

        let parsed = ipe_digest_parse("sha256:0A10ff").expect("digest parses");
        assert_eq!(parsed.alg, "sha256");
        assert_eq!(parsed.digest, [0x0a, 0x10, 0xff]);
        assert_eq!(parsed.digest_len(), 3);
        assert_eq!(ipe_digest_audit(&parsed), "sha256:0a10ff");
        assert!(ipe_digest_eval(
            &parsed,
            &DigestInfo {
                alg: String::from("sha256"),
                digest: alloc::vec![0x0a, 0x10, 0xff],
            }
        ));
        ipe_digest_free(Some(parsed));

        assert_eq!(ipe_digest_parse("sha256").unwrap_err(), -EBADMSG);
        assert_eq!(
            ipe_digest_parse("1c0d7ee1f8343b7fbe418378e8eb22c061d7dec7").unwrap_err(),
            -EBADMSG
        );
        assert_eq!(ipe_digest_parse("sha256:abc").unwrap_err(), -EINVAL);
        assert_eq!(ipe_digest_parse("sha256:xx").unwrap_err(), -EINVAL);
    }

    #[test]
    fn ipe_digest_parser_uses_linux_hex2bin_count() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let hexdump = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/hexdump.c"
        ));

        assert!(hexdump.contains("int hex2bin(u8 *dst, const char *src, size_t count)"));
        assert!(hexdump.contains("while (count--)"));
        assert!(hexdump.contains("hi = hex_to_bin(*src++);"));
        assert!(hexdump.contains("lo = hex_to_bin(*src++);"));
        assert!(hexdump.contains("*dst++ = (hi << 4) | lo;"));

        assert_eq!(ipe_digest_parse("sha256:").unwrap().digest, []);
        assert_eq!(ipe_digest_parse("sha256:f").unwrap_err(), -EINVAL);
        assert_eq!(ipe_digest_parse("sha256:ff0").unwrap_err(), -EINVAL);
    }
}
