//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/platform_certs/keyring_handler.c
//! test-origin: linux:vendor/linux/security/integrity/platform_certs/keyring_handler.c
//! UEFI signature-list keyring handler selection.

use super::machine_keyring::{MachineTrustConfig, imputed_trust_enabled};

pub type EfiGuid = [u8; 16];

pub const EFI_CERT_X509_GUID: EfiGuid = crate::security::platform_certs::EFI_CERT_X509_GUID;
pub const EFI_CERT_SHA256_GUID: EfiGuid = [
    0x26, 0x16, 0xc4, 0xc1, 0x4c, 0x50, 0x92, 0x40, 0xac, 0xa9, 0x41, 0xf9, 0x36, 0x93, 0x43, 0x28,
];
pub const EFI_CERT_X509_SHA256_GUID: EfiGuid = [
    0x92, 0xa4, 0xd2, 0x3b, 0xc0, 0x96, 0x79, 0x40, 0xb4, 0x20, 0xfc, 0xf9, 0x8e, 0xf1, 0x03, 0xed,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EfiElementHandler {
    PlatformKeyring,
    MachineKeyring,
    SecondaryKeyring,
    BlacklistX509Tbs,
    BlacklistBinary,
    RevocationListX509,
}

pub const fn get_handler_for_db(sig_type: &EfiGuid) -> Option<EfiElementHandler> {
    if guid_eq(sig_type, &EFI_CERT_X509_GUID) {
        Some(EfiElementHandler::PlatformKeyring)
    } else {
        None
    }
}

pub const fn get_handler_for_mok(
    sig_type: &EfiGuid,
    machine_keyring_enabled: bool,
    trust_config: MachineTrustConfig,
) -> Option<EfiElementHandler> {
    if guid_eq(sig_type, &EFI_CERT_X509_GUID) {
        if machine_keyring_enabled && imputed_trust_enabled(trust_config) {
            Some(EfiElementHandler::MachineKeyring)
        } else {
            Some(EfiElementHandler::PlatformKeyring)
        }
    } else {
        None
    }
}

pub const fn get_handler_for_ca_keys(sig_type: &EfiGuid) -> Option<EfiElementHandler> {
    if guid_eq(sig_type, &EFI_CERT_X509_GUID) {
        Some(EfiElementHandler::MachineKeyring)
    } else {
        None
    }
}

pub const fn get_handler_for_code_signing_keys(sig_type: &EfiGuid) -> Option<EfiElementHandler> {
    if guid_eq(sig_type, &EFI_CERT_X509_GUID) {
        Some(EfiElementHandler::SecondaryKeyring)
    } else {
        None
    }
}

pub const fn get_handler_for_dbx(sig_type: &EfiGuid) -> Option<EfiElementHandler> {
    if guid_eq(sig_type, &EFI_CERT_X509_SHA256_GUID) {
        Some(EfiElementHandler::BlacklistX509Tbs)
    } else if guid_eq(sig_type, &EFI_CERT_SHA256_GUID) {
        Some(EfiElementHandler::BlacklistBinary)
    } else if guid_eq(sig_type, &EFI_CERT_X509_GUID) {
        Some(EfiElementHandler::RevocationListX509)
    } else {
        None
    }
}

const fn guid_eq(left: &EfiGuid, right: &EfiGuid) -> bool {
    let mut index = 0usize;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRUSTED_MOK: MachineTrustConfig = MachineTrustConfig {
        efi_boot: true,
        mok_list_trusted: true,
    };
    const UNTRUSTED_MOK: MachineTrustConfig = MachineTrustConfig {
        efi_boot: true,
        mok_list_trusted: false,
    };

    #[test]
    fn keyring_handlers_match_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/keyring_handler.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/keyring_handler.h"
        ));
        assert!(
            source
                .contains("static efi_guid_t efi_cert_x509_guid __initdata = EFI_CERT_X509_GUID;")
        );
        assert!(source.contains("static efi_guid_t efi_cert_x509_sha256_guid __initdata"));
        assert!(source.contains("static efi_guid_t efi_cert_sha256_guid __initdata"));
        assert!(source.contains("mark_hash_blacklisted(data, len, BLACKLIST_HASH_X509_TBS);"));
        assert!(source.contains("mark_hash_blacklisted(data, len, BLACKLIST_HASH_BINARY);"));
        assert!(source.contains("add_key_to_revocation_list(data, len);"));
        assert!(source.contains("efi_element_handler_t get_handler_for_db"));
        assert!(source.contains("return add_to_platform_keyring;"));
        assert!(source.contains("efi_element_handler_t get_handler_for_mok"));
        assert!(source.contains("return add_to_machine_keyring;"));
        assert!(source.contains("return add_to_secondary_keyring;"));
        assert!(source.contains("efi_element_handler_t get_handler_for_dbx"));
        assert!(header.contains("get_handler_for_code_signing_keys"));
        assert!(header.contains("get_handler_for_dbx"));

        assert_eq!(
            get_handler_for_db(&EFI_CERT_X509_GUID),
            Some(EfiElementHandler::PlatformKeyring)
        );
        assert_eq!(
            get_handler_for_mok(&EFI_CERT_X509_GUID, true, TRUSTED_MOK),
            Some(EfiElementHandler::MachineKeyring)
        );
        assert_eq!(
            get_handler_for_mok(&EFI_CERT_X509_GUID, true, UNTRUSTED_MOK),
            Some(EfiElementHandler::PlatformKeyring)
        );
        assert_eq!(
            get_handler_for_mok(&EFI_CERT_X509_GUID, false, TRUSTED_MOK),
            Some(EfiElementHandler::PlatformKeyring)
        );
        assert_eq!(
            get_handler_for_ca_keys(&EFI_CERT_X509_GUID),
            Some(EfiElementHandler::MachineKeyring)
        );
        assert_eq!(
            get_handler_for_code_signing_keys(&EFI_CERT_X509_GUID),
            Some(EfiElementHandler::SecondaryKeyring)
        );
        assert_eq!(
            get_handler_for_dbx(&EFI_CERT_X509_SHA256_GUID),
            Some(EfiElementHandler::BlacklistX509Tbs)
        );
        assert_eq!(
            get_handler_for_dbx(&EFI_CERT_SHA256_GUID),
            Some(EfiElementHandler::BlacklistBinary)
        );
        assert_eq!(
            get_handler_for_dbx(&EFI_CERT_X509_GUID),
            Some(EfiElementHandler::RevocationListX509)
        );
        assert_eq!(get_handler_for_db(&[0u8; 16]), None);
    }
}
