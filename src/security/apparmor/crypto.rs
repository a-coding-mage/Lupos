//! linux-parity: complete
//! linux-source: vendor/linux/security/apparmor/crypto.c
//! test-origin: linux:vendor/linux/security/apparmor/crypto.c
//! AppArmor SHA-256 policy hashing helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::security::integrity::ima::{IMA_SHA256_DIGEST_SIZE, sha256_digest};

pub const AA_HASH_ALGORITHM: &str = "sha256";
pub const SHA256_DIGEST_SIZE: usize = IMA_SHA256_DIGEST_SIZE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AaProfileHash {
    pub version_le: [u8; 4],
    pub digest: [u8; SHA256_DIGEST_SIZE],
}

pub fn aa_hash_size() -> usize {
    SHA256_DIGEST_SIZE
}

pub fn aa_calc_hash(data: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    sha256_digest(data)
}

pub fn aa_calc_profile_hash(
    hash_policy_enabled: bool,
    version: u32,
    start: &[u8],
) -> Option<AaProfileHash> {
    if !hash_policy_enabled {
        return None;
    }

    let version_le = version.to_le_bytes();
    let mut material = Vec::with_capacity(version_le.len() + start.len());
    material.extend_from_slice(&version_le);
    material.extend_from_slice(start);

    Some(AaProfileHash {
        version_le,
        digest: sha256_digest(&material),
    })
}

pub fn init_profile_hash(apparmor_initialized: bool) -> bool {
    if apparmor_initialized {
        crate::kernel::printk::log_info!("AppArmor", "AppArmor sha256 policy hashing enabled");
    }
    apparmor_initialized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apparmor_crypto_hashes_policy_bytes_like_linux_sha256_flow() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/apparmor/crypto.c"
        ));
        assert!(source.contains("return SHA256_DIGEST_SIZE;"));
        assert!(source.contains("sha256(data, len, hash);"));
        assert!(source.contains("__le32 le32_version = cpu_to_le32(version);"));
        assert!(source.contains("sha256_update(&sctx, (u8 *)&le32_version, 4);"));
        assert!(source.contains("sha256_update(&sctx, (u8 *)start, len);"));
        assert!(source.contains("init_profile_hash"));

        assert_eq!(aa_hash_size(), 32);
        assert_eq!(
            aa_calc_hash(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad
            ]
        );

        assert_eq!(aa_calc_profile_hash(false, 9, b"profile"), None);
        let profile = aa_calc_profile_hash(true, 9, b"profile").expect("profile hash");
        assert_eq!(profile.version_le, [9, 0, 0, 0]);

        let mut expected_material = Vec::new();
        expected_material.extend_from_slice(&9u32.to_le_bytes());
        expected_material.extend_from_slice(b"profile");
        assert_eq!(profile.digest, sha256_digest(&expected_material));
        assert!(init_profile_hash(true));
        assert!(!init_profile_hash(false));
    }
}
