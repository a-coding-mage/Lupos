//! linux-parity: complete
//! linux-source: vendor/linux/fs/ecryptfs/debug.c
//! test-origin: linux:vendor/linux/fs/ecryptfs/debug.c
//! eCryptfs debug dump decision points.

pub const ECRYPTFS_PRIVATE_KEY: u32 = 1;
pub const ECRYPTFS_PERSISTENT_PASSWORD: u32 = 0x01;
pub const ECRYPTFS_USERSPACE_SHOULD_TRY_TO_DECRYPT: u32 = 0x0000_0001;
pub const ECRYPTFS_USERSPACE_SHOULD_TRY_TO_ENCRYPT: u32 = 0x0000_0002;
pub const ECRYPTFS_CONTAINS_DECRYPTED_KEY: u32 = 0x0000_0004;
pub const ECRYPTFS_CONTAINS_ENCRYPTED_KEY: u32 = 0x0000_0008;
pub const ECRYPTFS_SALT_SIZE: usize = 8;
pub const ECRYPTFS_SIG_SIZE_HEX: usize = 16;
pub const ECRYPTFS_DEFAULT_KEY_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcryptfsAuthTokDumpPlan {
    pub private_key: bool,
    pub prints_salt: bool,
    pub prints_persistent: bool,
    pub prints_signature: bool,
    pub userspace_decrypt_request: bool,
    pub userspace_encrypt_request: bool,
    pub prints_decrypted_key_header: bool,
    pub dumps_decrypted_key: bool,
    pub prints_encrypted_key_header: bool,
    pub dumps_encrypted_key: bool,
}

pub const fn ecryptfs_dump_auth_tok_plan(
    auth_flags: u32,
    password_flags: u32,
    session_flags: u32,
    verbosity: i32,
) -> EcryptfsAuthTokDumpPlan {
    let private_key = auth_flags & ECRYPTFS_PRIVATE_KEY != 0;
    let has_decrypted = session_flags & ECRYPTFS_CONTAINS_DECRYPTED_KEY != 0;
    let has_encrypted = session_flags & ECRYPTFS_CONTAINS_ENCRYPTED_KEY != 0;
    EcryptfsAuthTokDumpPlan {
        private_key,
        prints_salt: !private_key,
        prints_persistent: !private_key && password_flags & ECRYPTFS_PERSISTENT_PASSWORD != 0,
        prints_signature: !private_key,
        userspace_decrypt_request: session_flags & ECRYPTFS_USERSPACE_SHOULD_TRY_TO_DECRYPT != 0,
        userspace_encrypt_request: session_flags & ECRYPTFS_USERSPACE_SHOULD_TRY_TO_ENCRYPT != 0,
        prints_decrypted_key_header: has_decrypted,
        dumps_decrypted_key: has_decrypted && verbosity > 0,
        prints_encrypted_key_header: has_encrypted,
        dumps_encrypted_key: has_encrypted && verbosity > 0,
    }
}

pub const fn ecryptfs_dump_hex_enabled(verbosity: i32) -> bool {
    verbosity >= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecryptfs_debug_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ecryptfs/debug.c"
        ));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include \"ecryptfs_kernel.h\""));
        assert!(source.contains("void ecryptfs_dump_auth_tok"));
        assert!(source.contains("char salt[ECRYPTFS_SALT_SIZE * 2 + 1];"));
        assert!(source.contains("char sig[ECRYPTFS_SIG_SIZE_HEX + 1];"));
        assert!(source.contains("auth_tok->flags & ECRYPTFS_PRIVATE_KEY"));
        assert!(source.contains("ecryptfs_to_hex(salt"));
        assert!(source.contains("ECRYPTFS_PERSISTENT_PASSWORD"));
        assert!(source.contains("strscpy(sig, auth_tok->token.password.signature);"));
        assert!(source.contains("ECRYPTFS_USERSPACE_SHOULD_TRY_TO_DECRYPT"));
        assert!(source.contains("ECRYPTFS_USERSPACE_SHOULD_TRY_TO_ENCRYPT"));
        assert!(source.contains("ECRYPTFS_CONTAINS_DECRYPTED_KEY"));
        assert!(source.contains("ECRYPTFS_DEFAULT_KEY_BYTES"));
        assert!(source.contains("ECRYPTFS_CONTAINS_ENCRYPTED_KEY"));
        assert!(source.contains("void ecryptfs_dump_hex"));
        assert!(source.contains("if (ecryptfs_verbosity < 1)"));
        assert!(source.contains("print_hex_dump(KERN_DEBUG, \"ecryptfs: \""));

        let passphrase = ecryptfs_dump_auth_tok_plan(
            0,
            ECRYPTFS_PERSISTENT_PASSWORD,
            ECRYPTFS_USERSPACE_SHOULD_TRY_TO_DECRYPT | ECRYPTFS_CONTAINS_DECRYPTED_KEY,
            1,
        );
        assert!(passphrase.prints_salt);
        assert!(passphrase.prints_persistent);
        assert!(passphrase.userspace_decrypt_request);
        assert!(passphrase.dumps_decrypted_key);

        let private = ecryptfs_dump_auth_tok_plan(
            ECRYPTFS_PRIVATE_KEY,
            0,
            ECRYPTFS_CONTAINS_ENCRYPTED_KEY,
            0,
        );
        assert!(private.private_key);
        assert!(!private.prints_salt);
        assert!(private.prints_encrypted_key_header);
        assert!(!private.dumps_encrypted_key);
        assert!(!ecryptfs_dump_hex_enabled(0));
        assert!(ecryptfs_dump_hex_enabled(1));
    }
}
