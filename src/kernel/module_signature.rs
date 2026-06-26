//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module_signature.c
//! test-origin: linux:vendor/linux/kernel/module_signature.c
//! Module PKCS#7 signature information block validation.

use crate::include::uapi::errno::{EBADMSG, ENOPKG};

pub const MODULE_SIGNATURE_MARKER: &str = "~Module signature appended~\n";
pub const MODULE_SIGNATURE_TYPE_PKCS7: u8 = 2;
pub const MODULE_SIGNATURE_ENCODED_SIZE: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSignature {
    pub algo: u8,
    pub hash: u8,
    pub id_type: u8,
    pub signer_len: u8,
    pub key_id_len: u8,
    pub pad: [u8; 3],
    pub sig_len_be: u32,
}

impl ModuleSignature {
    pub const fn sig_len(self) -> usize {
        u32::from_be(self.sig_len_be) as usize
    }
}

pub const fn mod_check_sig(ms: ModuleSignature, file_len: usize) -> Result<(), i32> {
    if file_len < MODULE_SIGNATURE_ENCODED_SIZE {
        return Err(-EBADMSG);
    }
    if ms.sig_len() >= file_len - MODULE_SIGNATURE_ENCODED_SIZE {
        return Err(-EBADMSG);
    }
    if ms.id_type != MODULE_SIGNATURE_TYPE_PKCS7 {
        return Err(-ENOPKG);
    }
    if ms.algo != 0
        || ms.hash != 0
        || ms.signer_len != 0
        || ms.key_id_len != 0
        || ms.pad[0] != 0
        || ms.pad[1] != 0
        || ms.pad[2] != 0
    {
        return Err(-EBADMSG);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: ModuleSignature = ModuleSignature {
        algo: 0,
        hash: 0,
        id_type: MODULE_SIGNATURE_TYPE_PKCS7,
        signer_len: 0,
        key_id_len: 0,
        pad: [0; 3],
        sig_len_be: 16u32.to_be(),
    };

    #[test]
    fn module_signature_validation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module_signature.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/module_signature.h"
        ));
        assert!(source.contains("be32_to_cpu(ms->sig_len) >= file_len - sizeof(*ms)"));
        assert!(source.contains("ms->id_type != MODULE_SIGNATURE_TYPE_PKCS7"));
        assert!(source.contains("return -ENOPKG;"));
        assert!(source.contains("ms->algo != 0"));
        assert!(source.contains("return -EBADMSG;"));
        assert!(header.contains("MODULE_SIGNATURE_MARKER"));
        assert!(header.contains("__be32\tsig_len"));

        assert_eq!(mod_check_sig(VALID, 64), Ok(()));
        assert_eq!(mod_check_sig(VALID, 28), Err(-EBADMSG));
        assert_eq!(
            mod_check_sig(
                ModuleSignature {
                    id_type: 1,
                    ..VALID
                },
                64
            ),
            Err(-ENOPKG)
        );
        assert_eq!(
            mod_check_sig(ModuleSignature { algo: 1, ..VALID }, 64),
            Err(-EBADMSG)
        );
    }
}
