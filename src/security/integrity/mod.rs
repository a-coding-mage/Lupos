//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity
//! test-origin: linux:vendor/linux/security/integrity
//! Integrity subsystem glue for IMA/EVM.
//!
//! This module wires the Linux boot-time integrity initializers into the
//! security init path. IMA appraisal consumes EVM immutable/failure statuses
//! and verifies Linux `signature_v2_hdr` RSA signatures through the `.ima`
//! keyring with the `.platform` kexec fallback; EVM asymmetric signature
//! verification uses the `.evm` integrity keyring.

pub mod efi_secureboot;
pub mod evm;
#[path = "evm/evm_posix_acl.rs"]
pub mod evm_posix_acl;
pub mod evm_secfs;
pub mod iint;
pub mod ima;
#[path = "ima/ima_asymmetric_keys.rs"]
pub mod ima_asymmetric_keys;
pub mod ima_efi;
pub mod ima_fs;
pub mod ima_mok;
pub mod integrity_audit;
pub mod platform_certs;

pub fn init() {
    ima::init();
    evm::init();
}

#[cfg(test)]
mod tests {
    use crate::security::hooks::{LSM_ID_EVM, LSM_ID_IMA};
    use crate::security::lsm_list::{TEST_LSM_LOCK, lsm_active_ids, reset_for_test};

    #[test]
    fn integrity_init_registers_ima_and_evm_lsms() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        super::ima::reset_for_test();
        super::evm::reset_for_test();

        super::init();

        let mut ids = [0u64; 4];
        let count = lsm_active_ids(&mut ids);
        assert_eq!(count, 2);
        assert_eq!(ids[0], LSM_ID_IMA);
        assert_eq!(ids[1], LSM_ID_EVM);
        assert!(super::ima::snapshot().tpm_bypass);
        assert_eq!(super::evm::snapshot().xattr_count, 8);
    }
}
