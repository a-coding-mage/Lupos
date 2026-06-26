//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/platform_certs/platform_keyring.c
//! test-origin: linux:vendor/linux/security/integrity/platform_certs/platform_keyring.c
//! Platform keyring init and certificate import.

pub const PLATFORM_KEYRING_PERMISSION: u32 = 0x3f01_0000;

pub fn platform_keyring_init() -> Result<crate::security::platform_certs::PlatformKeyringState, i32>
{
    crate::security::platform_certs::init_with_uefi_signature_lists(&[])
}

pub fn add_to_platform_keyring(
    source: &str,
    data: &[u8],
) -> Result<Option<crate::security::platform_certs::PlatformCertificate>, i32> {
    crate::security::platform_certs::add_to_platform_keyring(source, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset_all() {
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        crate::security::certs::reset_for_test();
        crate::security::platform_certs::reset_for_test();
    }

    fn der_with_common_name(name: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, name.len() as u8]);
        body.extend_from_slice(name.as_bytes());

        let mut der = Vec::new();
        der.extend_from_slice(&[0x30, 0x82]);
        der.extend_from_slice(&(body.len() as u16).to_be_bytes());
        der.extend_from_slice(&body);
        der
    }

    #[test]
    fn platform_keyring_init_and_add_delegate_to_integrity_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/platform_keyring.c"
        ));
        assert!(source.contains("add_to_platform_keyring(const char *source"));
        assert!(source.contains("integrity_load_cert(INTEGRITY_KEYRING_PLATFORM"));
        assert!(source.contains("integrity_init_keyring(INTEGRITY_KEYRING_PLATFORM)"));
        assert!(source.contains("device_initcall(platform_keyring_init);"));

        let state = platform_keyring_init().expect("platform keyring");
        assert!(state.platform_keyring > 0);
        assert_eq!(
            crate::security::keys::describe(state.platform_keyring).as_deref(),
            Some("keyring;0;0;3f010000;.platform")
        );

        let cert = der_with_common_name("Firmware Platform CA");
        let loaded = add_to_platform_keyring("UEFI:db", &cert)
            .expect("platform import")
            .expect("loaded cert");
        assert_eq!(loaded.source, "UEFI:db");
        assert_eq!(loaded.description, "Firmware Platform CA");
        assert_eq!(
            crate::security::platform_certs::loaded_certificate_count(),
            1
        );
    }
}
