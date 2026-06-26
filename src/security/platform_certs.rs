//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/platform_certs
//! test-origin: linux:vendor/linux/security/integrity/platform_certs
//! UEFI Secure Boot platform certificate keyring.
//!
//! Mirrors the split Linux uses in
//! `security/integrity/platform_certs/platform_keyring.c`,
//! `keyring_handler.c`, `efi_parser.c`, and `load_uefi.c`: initialise a
//! separate `.platform` keyring, parse EFI signature lists, and load X.509
//! `db`/MOK elements into that platform trusted keyring. Production init also
//! asks the EFI runtime-variable facade for `db` and `MokListRT`, matching the
//! Linux late-init import path once an OVMF provider registers variables.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

use crate::efi::vars;
use crate::include::uapi::errno::{EBADMSG, EINVAL};
use crate::security::{certs, keys};

pub const PLATFORM_KEYRING_NAME: &str = ".platform";

/// UEFI `EFI_CERT_X509_GUID` encoded as it appears in EFI signature lists.
pub const EFI_CERT_X509_GUID: [u8; 16] = [
    0xa1, 0x59, 0xc0, 0xa5, 0xe4, 0x94, 0xa7, 0x4a, 0x87, 0xb5, 0xab, 0x15, 0x5c, 0x2b, 0xf0, 0x72,
];

const EFI_SIGNATURE_LIST_HEADER_SIZE: usize = 16 + 4 + 4 + 4;
const EFI_SIGNATURE_DATA_OWNER_SIZE: usize = 16;

#[cfg(feature = "test-uefi-platform-certs")]
const TEST_ARCH_SECURE_BOOT_CA_EFI_SIGNATURE_LIST: &[u8] = &[
    0xa1, 0x59, 0xc0, 0xa5, 0xe4, 0x94, 0xa7, 0x4a, 0x87, 0xb5, 0xab, 0x15, 0x5c, 0x2b, 0xf0, 0x72,
    0x4a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2e, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x30, 0x82, 0x00, 0x1a,
    0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x13, b'A', b'r', b'c', b'h', b' ', b'S', b'e', b'c', b'u',
    b'r', b'e', b' ', b'B', b'o', b'o', b't', b' ', b'C', b'A',
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformCertificate {
    pub key_id: i32,
    pub source: String,
    pub description: String,
    pub len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformKeyringState {
    pub platform_keyring: i32,
    pub loaded_certificates: Vec<PlatformCertificate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UefiSignatureSource<'a> {
    pub source: &'static str,
    pub data: &'a [u8],
}

static PLATFORM_KEYRINGS: Mutex<Option<PlatformKeyringState>> = Mutex::new(None);

pub fn init() {
    let _ = init_with_runtime_uefi_variables();
}

#[cfg(feature = "test-uefi-platform-certs")]
pub fn register_test_runtime_uefi_db() -> Result<(), i32> {
    vars::register_runtime_variables(&[vars::RuntimeVariableSource {
        name: "db",
        vendor: vars::EFI_IMAGE_SECURITY_DATABASE_GUID,
        attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
        data: TEST_ARCH_SECURE_BOOT_CA_EFI_SIGNATURE_LIST,
    }])
}

pub fn snapshot() -> Option<PlatformKeyringState> {
    PLATFORM_KEYRINGS.lock().clone()
}

pub fn platform_keyring_id() -> Option<i32> {
    snapshot().map(|state| state.platform_keyring)
}

pub fn loaded_certificate_count() -> usize {
    snapshot()
        .map(|state| state.loaded_certificates.len())
        .unwrap_or(0)
}

pub fn init_with_uefi_signature_lists(
    lists: &[UefiSignatureSource<'_>],
) -> Result<PlatformKeyringState, i32> {
    init_with_loader(|state| {
        for list in lists {
            load_nonfatal_uefi_signature_list(state, list.source, list.data);
        }
    })
}

pub fn init_with_runtime_uefi_variables() -> Result<PlatformKeyringState, i32> {
    init_with_loader(|state| {
        let _ = load_runtime_uefi_signature_lists_into_state(state);
    })
}

fn init_with_loader<F>(mut load: F) -> Result<PlatformKeyringState, i32>
where
    F: FnMut(&mut PlatformKeyringState),
{
    if let Some(state) = PLATFORM_KEYRINGS.lock().clone() {
        return Ok(state);
    }

    let keyring_id = keys::add_key("keyring", PLATFORM_KEYRING_NAME, &[]);
    if keyring_id < 0 {
        return Err(keyring_id);
    }
    crate::kernel::printk::log_info!("integrity", "Platform Keyring initialized");

    let mut state = PlatformKeyringState {
        platform_keyring: keyring_id,
        loaded_certificates: Vec::new(),
    };

    load(&mut state);

    *PLATFORM_KEYRINGS.lock() = Some(state.clone());
    Ok(state)
}

pub fn load_uefi_signature_list(source: &str, data: &[u8]) -> Result<usize, i32> {
    if PLATFORM_KEYRINGS.lock().is_none() {
        init_with_uefi_signature_lists(&[])?;
    }

    let mut guard = PLATFORM_KEYRINGS.lock();
    let state = guard.as_mut().ok_or(-EINVAL)?;
    load_uefi_signature_list_into_state(state, source, data)
}

pub fn load_runtime_uefi_signature_lists() -> Result<usize, i32> {
    if PLATFORM_KEYRINGS.lock().is_none() {
        init_with_uefi_signature_lists(&[])?;
    }

    let mut guard = PLATFORM_KEYRINGS.lock();
    let state = guard.as_mut().ok_or(-EINVAL)?;
    Ok(load_runtime_uefi_signature_lists_into_state(state))
}

pub fn add_to_platform_keyring(
    source: &str,
    cert_der: &[u8],
) -> Result<Option<PlatformCertificate>, i32> {
    if PLATFORM_KEYRINGS.lock().is_none() {
        init_with_uefi_signature_lists(&[])?;
    }

    let mut guard = PLATFORM_KEYRINGS.lock();
    let state = guard.as_mut().ok_or(-EINVAL)?;
    let before = state.loaded_certificates.len();
    if import_platform_x509(state, source, cert_der) {
        Ok(state.loaded_certificates.get(before).cloned())
    } else {
        Ok(None)
    }
}

fn load_runtime_uefi_signature_lists_into_state(state: &mut PlatformKeyringState) -> usize {
    if !vars::runtime_variables_available() {
        return 0;
    }

    let mut loaded = 0usize;
    if vars::variable_exists("MokIgnoreDB", vars::EFI_SHIM_LOCK_GUID) != Ok(true) {
        loaded += load_runtime_signature_variable(
            state,
            "db",
            vars::EFI_IMAGE_SECURITY_DATABASE_GUID,
            "UEFI:db",
        );
    }
    loaded += load_runtime_signature_variable(
        state,
        "MokListRT",
        vars::EFI_SHIM_LOCK_GUID,
        "UEFI:MokListRT",
    );
    loaded
}

fn load_runtime_signature_variable(
    state: &mut PlatformKeyringState,
    name: &str,
    vendor: vars::Guid,
    source: &'static str,
) -> usize {
    match vars::get_variable(name, vendor) {
        Ok(var) => {
            let before = state.loaded_certificates.len();
            load_nonfatal_uefi_signature_list(state, source, &var.data);
            state.loaded_certificates.len().saturating_sub(before)
        }
        Err(_) => 0,
    }
}

fn load_nonfatal_uefi_signature_list(state: &mut PlatformKeyringState, source: &str, data: &[u8]) {
    if let Err(err) = load_uefi_signature_list_into_state(state, source, data) {
        crate::kernel::printk::log_info!(
            "integrity",
            "Problem parsing {} signatures: {}",
            source,
            err
        );
    }
}

fn load_uefi_signature_list_into_state(
    state: &mut PlatformKeyringState,
    source: &str,
    data: &[u8],
) -> Result<usize, i32> {
    let mut offset = 0usize;
    let mut loaded = 0usize;

    while offset < data.len() {
        let remaining = data.len() - offset;
        if remaining < EFI_SIGNATURE_LIST_HEADER_SIZE {
            return Err(-EBADMSG);
        }

        let signature_type = read_guid(data, offset).ok_or(-EBADMSG)?;
        let signature_list_size = read_le_u32(data, offset + 16).ok_or(-EBADMSG)? as usize;
        let signature_header_size = read_le_u32(data, offset + 20).ok_or(-EBADMSG)? as usize;
        let signature_size = read_le_u32(data, offset + 24).ok_or(-EBADMSG)? as usize;

        if signature_list_size > remaining
            || signature_list_size < EFI_SIGNATURE_LIST_HEADER_SIZE
            || signature_size < EFI_SIGNATURE_DATA_OWNER_SIZE
        {
            return Err(-EBADMSG);
        }

        let entries_start = offset
            .checked_add(EFI_SIGNATURE_LIST_HEADER_SIZE)
            .and_then(|value| value.checked_add(signature_header_size))
            .ok_or(-EBADMSG)?;
        let list_end = offset.checked_add(signature_list_size).ok_or(-EBADMSG)?;
        if entries_start > list_end {
            return Err(-EBADMSG);
        }

        let entries_len = list_end - entries_start;
        if entries_len < signature_size || entries_len % signature_size != 0 {
            return Err(-EBADMSG);
        }

        if signature_type == EFI_CERT_X509_GUID {
            let mut entry_offset = entries_start;
            while entry_offset < list_end {
                let cert_start = entry_offset + EFI_SIGNATURE_DATA_OWNER_SIZE;
                let cert_end = entry_offset + signature_size;
                if import_platform_x509(state, source, &data[cert_start..cert_end]) {
                    loaded += 1;
                }
                entry_offset += signature_size;
            }
        }

        offset = list_end;
    }

    Ok(loaded)
}

fn import_platform_x509(state: &mut PlatformKeyringState, source: &str, cert_der: &[u8]) -> bool {
    match certs::x509_der_len_at(cert_der, 0) {
        Ok(len) if len == cert_der.len() => {}
        _ => {
            crate::kernel::printk::log_info!(
                "integrity",
                "Problem loading X.509 certificate from {}",
                source
            );
            return false;
        }
    }

    let fallback = format!(
        "{} certificate #{}",
        source,
        state.loaded_certificates.len()
    );
    let description = certs::x509_certificate_description(cert_der).unwrap_or(fallback);
    let Some(certificate) = certs::load_x509_certificate(&description, cert_der) else {
        return false;
    };
    if keys::link_key_to_keyring(certificate.key_id, state.platform_keyring).is_err() {
        return false;
    }
    crate::kernel::printk::log_info!(
        "integrity",
        "{}",
        certs::format_loaded_x509_log(&description)
    );

    state.loaded_certificates.push(PlatformCertificate {
        key_id: certificate.key_id,
        source: String::from(source),
        description: certificate.description,
        len: certificate.len,
    });
    true
}

fn read_guid(data: &[u8], offset: usize) -> Option<[u8; 16]> {
    let end = offset.checked_add(16)?;
    let bytes = data.get(offset..end)?;
    let mut guid = [0u8; 16];
    guid.copy_from_slice(bytes);
    Some(guid)
}

fn read_le_u32(data: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(data.get(offset..end)?.try_into().ok()?))
}

#[cfg(test)]
pub fn reset_for_test() {
    *PLATFORM_KEYRINGS.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_all() {
        keys::reset_for_test();
        keys::init();
        certs::reset_for_test();
        vars::unregister_runtime_variables();
        reset_for_test();
        crate::linux_driver_abi::tty::serial::clear_capture_for_tests();
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

    fn efi_signature_list(cert_der: &[u8]) -> Vec<u8> {
        let signature_size = EFI_SIGNATURE_DATA_OWNER_SIZE + cert_der.len();
        let list_size = EFI_SIGNATURE_LIST_HEADER_SIZE + signature_size;
        let mut list = Vec::new();
        list.extend_from_slice(&EFI_CERT_X509_GUID);
        list.extend_from_slice(&(list_size as u32).to_le_bytes());
        list.extend_from_slice(&0u32.to_le_bytes());
        list.extend_from_slice(&(signature_size as u32).to_le_bytes());
        list.extend_from_slice(&[0x42; EFI_SIGNATURE_DATA_OWNER_SIZE]);
        list.extend_from_slice(cert_der);
        list
    }

    #[test]
    fn init_allocates_separate_platform_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let state = init_with_uefi_signature_lists(&[]).expect("platform keyring");

        assert!(state.platform_keyring > 0);
        assert!(state.loaded_certificates.is_empty());
        assert_eq!(
            keys::describe(state.platform_keyring).as_deref(),
            Some("keyring;0;0;3f010000;.platform")
        );
        assert_eq!(platform_keyring_id(), Some(state.platform_keyring));
    }

    #[test]
    fn uefi_signature_list_imports_x509_to_platform_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        let cert = der_with_common_name("Arch Secure Boot CA");
        let list = efi_signature_list(&cert);

        let state = init_with_uefi_signature_lists(&[UefiSignatureSource {
            source: "UEFI:db",
            data: &list,
        }])
        .expect("uefi db import");

        assert_eq!(state.loaded_certificates.len(), 1);
        assert_eq!(state.loaded_certificates[0].source, "UEFI:db");
        assert_eq!(
            state.loaded_certificates[0].description,
            "Arch Secure Boot CA"
        );
        assert_eq!(
            keys::describe(state.loaded_certificates[0].key_id).as_deref(),
            Some("asymmetric;0;0;3f010000;Arch Secure Boot CA")
        );
        let serial =
            String::from_utf8(crate::linux_driver_abi::tty::serial::captured_bytes_for_tests())
                .expect("serial utf8");
        assert!(serial.contains("integrity: Platform Keyring initialized"));
        assert!(serial.contains("integrity: Loaded X.509 cert 'Arch Secure Boot CA'"));
    }

    #[test]
    fn runtime_uefi_variables_import_db_and_moklist_to_platform_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        let db_cert = der_with_common_name("OVMF db CA");
        let mok_cert = der_with_common_name("Shim MOK CA");
        let db_list = efi_signature_list(&db_cert);
        let mok_list = efi_signature_list(&mok_cert);
        vars::register_runtime_variables(&[
            vars::RuntimeVariableSource {
                name: "db",
                vendor: vars::EFI_IMAGE_SECURITY_DATABASE_GUID,
                attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &db_list,
            },
            vars::RuntimeVariableSource {
                name: "MokListRT",
                vendor: vars::EFI_SHIM_LOCK_GUID,
                attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &mok_list,
            },
        ])
        .expect("runtime vars");

        let state = init_with_runtime_uefi_variables().expect("runtime import");

        assert_eq!(state.loaded_certificates.len(), 2);
        assert_eq!(state.loaded_certificates[0].source, "UEFI:db");
        assert_eq!(state.loaded_certificates[0].description, "OVMF db CA");
        assert_eq!(state.loaded_certificates[1].source, "UEFI:MokListRT");
        assert_eq!(state.loaded_certificates[1].description, "Shim MOK CA");
        assert_eq!(
            keys::describe(state.loaded_certificates[0].key_id).as_deref(),
            Some("asymmetric;0;0;3f010000;OVMF db CA")
        );
        assert_eq!(
            keys::describe(state.loaded_certificates[1].key_id).as_deref(),
            Some("asymmetric;0;0;3f010000;Shim MOK CA")
        );
    }

    #[test]
    fn runtime_uefi_mok_ignore_db_suppresses_db_but_keeps_moklist() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        let db_cert = der_with_common_name("Ignored db CA");
        let mok_cert = der_with_common_name("Runtime MOK CA");
        let db_list = efi_signature_list(&db_cert);
        let mok_list = efi_signature_list(&mok_cert);
        vars::register_runtime_variables(&[
            vars::RuntimeVariableSource {
                name: "MokIgnoreDB",
                vendor: vars::EFI_SHIM_LOCK_GUID,
                attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &[1],
            },
            vars::RuntimeVariableSource {
                name: "db",
                vendor: vars::EFI_IMAGE_SECURITY_DATABASE_GUID,
                attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &db_list,
            },
            vars::RuntimeVariableSource {
                name: "MokListRT",
                vendor: vars::EFI_SHIM_LOCK_GUID,
                attributes: vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &mok_list,
            },
        ])
        .expect("runtime vars");

        let state = init_with_runtime_uefi_variables().expect("runtime import");

        assert_eq!(state.loaded_certificates.len(), 1);
        assert_eq!(state.loaded_certificates[0].source, "UEFI:MokListRT");
        assert_eq!(state.loaded_certificates[0].description, "Runtime MOK CA");
    }

    #[test]
    fn malformed_uefi_signature_list_is_nonfatal_during_init() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let state = init_with_uefi_signature_lists(&[UefiSignatureSource {
            source: "UEFI:db",
            data: &[0x30, 0x82, 0x00],
        }])
        .expect("malformed list is nonfatal during init");

        assert!(state.platform_keyring > 0);
        assert!(state.loaded_certificates.is_empty());
        assert_eq!(loaded_certificate_count(), 0);
    }

    #[test]
    fn direct_uefi_signature_list_load_reports_bad_message() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        init_with_uefi_signature_lists(&[]).expect("platform keyring");

        assert_eq!(
            load_uefi_signature_list("UEFI:db", &[0x30, 0x82, 0x00]),
            Err(-EBADMSG)
        );
    }
}
