//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/encrypted-keys/ecryptfs_format.c
//! test-origin: linux:vendor/linux/security/keys/encrypted-keys/ecryptfs_format.c
//! eCryptfs auth-token helpers for encrypted keys.

pub const ECRYPTFS_VERSION_MAJOR: i32 = 0x00;
pub const ECRYPTFS_VERSION_MINOR: i32 = 0x04;
pub const ECRYPTFS_SUPPORTED_FILE_VERSION: i32 = 0x03;
pub const ECRYPTFS_PASSWORD: u16 = 0;
pub const ECRYPTFS_PASSWORD_SIG_SIZE: usize = 16;
pub const ECRYPTFS_MAX_KEY_BYTES: usize = 64;
pub const ECRYPTFS_MAX_ENCRYPTED_KEY_BYTES: usize = 512;
pub const ECRYPTFS_PERSISTENT_PASSWORD: u32 = 0x01;
pub const ECRYPTFS_SESSION_KEY_ENCRYPTION_KEY_SET: u32 = 0x02;
pub const PGP_DIGEST_ALGO_SHA512: i32 = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcryptfsSessionKey {
    pub encrypted_key: [u8; ECRYPTFS_MAX_ENCRYPTED_KEY_BYTES],
    pub encrypted_key_size: u32,
}

impl Default for EcryptfsSessionKey {
    fn default() -> Self {
        Self {
            encrypted_key: [0; ECRYPTFS_MAX_ENCRYPTED_KEY_BYTES],
            encrypted_key_size: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcryptfsPassword {
    pub signature: [u8; ECRYPTFS_PASSWORD_SIG_SIZE + 1],
    pub session_key_encryption_key: [u8; ECRYPTFS_MAX_KEY_BYTES],
    pub session_key_encryption_key_bytes: u32,
    pub hash_algo: i32,
    pub flags: u32,
}

impl Default for EcryptfsPassword {
    fn default() -> Self {
        Self {
            signature: [0; ECRYPTFS_PASSWORD_SIG_SIZE + 1],
            session_key_encryption_key: [0; ECRYPTFS_MAX_KEY_BYTES],
            session_key_encryption_key_bytes: 0,
            hash_algo: 0,
            flags: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EcryptfsAuthTok {
    pub version: u16,
    pub token_type: u16,
    pub session_key: EcryptfsSessionKey,
    pub password: EcryptfsPassword,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcryptfsVersions {
    pub major: i32,
    pub minor: i32,
    pub file_version: i32,
}

pub fn ecryptfs_get_auth_tok_key(
    auth_tok: &mut EcryptfsAuthTok,
) -> &mut [u8; ECRYPTFS_MAX_KEY_BYTES] {
    &mut auth_tok.password.session_key_encryption_key
}

pub const fn ecryptfs_get_versions() -> EcryptfsVersions {
    EcryptfsVersions {
        major: ECRYPTFS_VERSION_MAJOR,
        minor: ECRYPTFS_VERSION_MINOR,
        file_version: ECRYPTFS_SUPPORTED_FILE_VERSION,
    }
}

pub fn ecryptfs_fill_auth_tok(auth_tok: &mut EcryptfsAuthTok, key_desc: &str) -> Result<(), i32> {
    let versions = ecryptfs_get_versions();
    auth_tok.version = (((versions.major as u16) << 8) & 0xff00) | (versions.minor as u16 & 0x00ff);
    auth_tok.token_type = ECRYPTFS_PASSWORD;
    strscpy_pad_signature(&mut auth_tok.password.signature, key_desc);
    auth_tok.password.session_key_encryption_key_bytes = ECRYPTFS_MAX_KEY_BYTES as u32;
    auth_tok.password.flags |= ECRYPTFS_SESSION_KEY_ENCRYPTION_KEY_SET;
    auth_tok.session_key.encrypted_key[0] = 0;
    auth_tok.session_key.encrypted_key_size = 0;
    auth_tok.password.hash_algo = PGP_DIGEST_ALGO_SHA512;
    auth_tok.password.flags &= !ECRYPTFS_PERSISTENT_PASSWORD;
    Ok(())
}

fn strscpy_pad_signature(dst: &mut [u8; ECRYPTFS_PASSWORD_SIG_SIZE + 1], src: &str) {
    dst.fill(0);
    let bytes = src.as_bytes();
    let copy_len = bytes.len().min(ECRYPTFS_PASSWORD_SIG_SIZE);
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecryptfs_fill_auth_tok_matches_linux_defaults() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/encrypted-keys/ecryptfs_format.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/encrypted-keys/ecryptfs_format.h"
        ));
        assert!(source.contains("ecryptfs_get_auth_tok_key"));
        assert!(source.contains("ecryptfs_get_versions(&major, &minor, NULL)"));
        assert!(source.contains("ECRYPTFS_SESSION_KEY_ENCRYPTION_KEY_SET"));
        assert!(source.contains("PGP_DIGEST_ALGO_SHA512"));
        assert!(header.contains("#define PGP_DIGEST_ALGO_SHA512   10"));

        let mut auth_tok = EcryptfsAuthTok::default();
        auth_tok.password.flags = ECRYPTFS_PERSISTENT_PASSWORD;
        ecryptfs_fill_auth_tok(&mut auth_tok, "0123456789abcdef-long").expect("fill");

        assert_eq!(ecryptfs_get_versions().major, 0);
        assert_eq!(ecryptfs_get_versions().minor, 4);
        assert_eq!(ecryptfs_get_versions().file_version, 3);
        assert_eq!(auth_tok.version, 0x0004);
        assert_eq!(auth_tok.token_type, ECRYPTFS_PASSWORD);
        assert_eq!(&auth_tok.password.signature[..16], b"0123456789abcdef");
        assert_eq!(auth_tok.password.signature[16], 0);
        assert_eq!(
            auth_tok.password.session_key_encryption_key_bytes,
            ECRYPTFS_MAX_KEY_BYTES as u32
        );
        assert_eq!(
            auth_tok.password.flags & ECRYPTFS_SESSION_KEY_ENCRYPTION_KEY_SET,
            ECRYPTFS_SESSION_KEY_ENCRYPTION_KEY_SET
        );
        assert_eq!(auth_tok.password.flags & ECRYPTFS_PERSISTENT_PASSWORD, 0);
        assert_eq!(auth_tok.session_key.encrypted_key[0], 0);
        assert_eq!(auth_tok.session_key.encrypted_key_size, 0);
        assert_eq!(auth_tok.password.hash_algo, PGP_DIGEST_ALGO_SHA512);

        ecryptfs_get_auth_tok_key(&mut auth_tok)[0] = 0xaa;
        assert_eq!(auth_tok.password.session_key_encryption_key[0], 0xaa);
    }
}
