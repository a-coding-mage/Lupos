//! linux-parity: partial
//! linux-source: vendor/linux/certs
//! test-origin: linux:vendor/linux/certs
//! System trusted keyring and compiled-in X.509 certificate loader.
//!
//! Mirrors the initcall shape of `vendor/linux/certs/system_keyring.c` and the
//! DER-list framing in `vendor/linux/crypto/asymmetric_keys/x509_loader.c`.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

use crate::security::keys;

pub const BUILTIN_TRUSTED_KEYRING_NAME: &str = ".builtin_trusted_keys";

const SYSTEM_CERTIFICATE_LIST: &[u8] = &[];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedCertificate {
    pub key_id: i32,
    pub len: usize,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemKeyringState {
    pub builtin_trusted_keyring: i32,
    pub loaded_certificates: Vec<LoadedCertificate>,
}

static SYSTEM_KEYRINGS: Mutex<Option<SystemKeyringState>> = Mutex::new(None);

/// Initialise the system trusted keyring and load the compiled-in X.509 list.
/// Idempotent, matching Linux's one-shot initcall behaviour.
pub fn init() {
    let _ = init_with_certificate_list(SYSTEM_CERTIFICATE_LIST);
}

pub fn snapshot() -> Option<SystemKeyringState> {
    SYSTEM_KEYRINGS.lock().clone()
}

pub fn builtin_trusted_keyring_id() -> Option<i32> {
    snapshot().map(|state| state.builtin_trusted_keyring)
}

pub fn loaded_certificate_count() -> usize {
    snapshot()
        .map(|state| state.loaded_certificates.len())
        .unwrap_or(0)
}

fn init_with_certificate_list(cert_list: &[u8]) -> Result<SystemKeyringState, i32> {
    if let Some(state) = SYSTEM_KEYRINGS.lock().clone() {
        return Ok(state);
    }

    crate::kernel::printk::log_info!("", "Initialise system trusted keyrings");
    let keyring_id = keys::add_key("keyring", BUILTIN_TRUSTED_KEYRING_NAME, &[]);
    if keyring_id < 0 {
        return Err(keyring_id);
    }

    crate::kernel::printk::log_info!("", "Loading compiled-in X.509 certificates");
    let loaded_certificates = load_system_certificate_list(cert_list);
    let state = SystemKeyringState {
        builtin_trusted_keyring: keyring_id,
        loaded_certificates,
    };
    *SYSTEM_KEYRINGS.lock() = Some(state.clone());
    Ok(state)
}

fn load_system_certificate_list(cert_list: &[u8]) -> Vec<LoadedCertificate> {
    let mut loaded = Vec::new();
    let mut offset = 0usize;

    while offset < cert_list.len() {
        let cert_len = match x509_der_len_at(cert_list, offset) {
            Ok(cert_len) => cert_len,
            Err(()) => {
                crate::kernel::printk::log_info!(
                    "",
                    "Problem parsing in-kernel X.509 certificate list"
                );
                return loaded;
            }
        };
        let remaining = cert_list.len() - offset;
        if cert_len > remaining {
            crate::kernel::printk::log_info!(
                "",
                "Problem parsing in-kernel X.509 certificate list"
            );
            return loaded;
        }

        let cert_der = &cert_list[offset..offset + cert_len];
        let fallback = format!("builtin X.509 certificate #{}", loaded.len());
        let description = x509_certificate_description(cert_der).unwrap_or(fallback);
        let Some(certificate) = load_x509_certificate(&description, cert_der) else {
            return loaded;
        };
        crate::kernel::printk::log_info!("", "{}", format_loaded_x509_log(&description));
        loaded.push(certificate);
        offset += cert_len;
    }

    loaded
}

/// Return the length of a Linux-style DER certificate starting at `offset`.
///
/// This mirrors the framing consumed by Linux's `x509_load_certificate_list()`
/// path: each certificate starts with a DER SEQUENCE encoded as
/// `30 82 len_hi len_lo`.
pub(crate) fn x509_der_len_at(cert_list: &[u8], offset: usize) -> Result<usize, ()> {
    let remaining = cert_list.len().checked_sub(offset).ok_or(())?;
    if remaining < 4 || cert_list[offset] != 0x30 || cert_list[offset + 1] != 0x82 {
        return Err(());
    }

    Ok((((cert_list[offset + 2] as usize) << 8) | cert_list[offset + 3] as usize) + 4)
}

pub(crate) fn load_x509_certificate(
    description: &str,
    cert_der: &[u8],
) -> Option<LoadedCertificate> {
    let key_id = keys::add_key("asymmetric", description, cert_der);
    if key_id < 0 {
        return None;
    }

    Some(LoadedCertificate {
        key_id,
        len: cert_der.len(),
        description: String::from(description),
    })
}

pub(crate) fn format_loaded_x509_log(description: &str) -> String {
    format!("Loaded X.509 cert '{}'", description)
}

pub(crate) fn x509_certificate_description(cert_der: &[u8]) -> Option<String> {
    x509_subject_common_name(cert_der)
}

const X509_COMMON_NAME_OID: &[u8] = &[0x55, 0x04, 0x03];

fn x509_subject_common_name(cert_der: &[u8]) -> Option<String> {
    let mut offset = 0usize;
    while offset < cert_der.len() {
        if cert_der[offset] != 0x06 {
            offset += 1;
            continue;
        }

        let (oid_len, oid_start) = der_len(cert_der, offset + 1)?;
        let oid_end = oid_start.checked_add(oid_len)?;
        if oid_end > cert_der.len() {
            return None;
        }

        if &cert_der[oid_start..oid_end] == X509_COMMON_NAME_OID {
            let tag = *cert_der.get(oid_end)?;
            let (value_len, value_start) = der_len(cert_der, oid_end + 1)?;
            let value_end = value_start.checked_add(value_len)?;
            if value_end > cert_der.len() {
                return None;
            }
            return decode_directory_string(tag, &cert_der[value_start..value_end]);
        }

        offset = oid_end;
    }

    None
}

fn der_len(bytes: &[u8], offset: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(offset)?;
    if first < 0x80 {
        return Some((first as usize, offset + 1));
    }

    let width = (first & 0x7f) as usize;
    if width == 0 || width > core::mem::size_of::<usize>() {
        return None;
    }
    let start = offset + 1;
    let end = start.checked_add(width)?;
    if end > bytes.len() {
        return None;
    }

    let mut len = 0usize;
    for b in &bytes[start..end] {
        len = len.checked_shl(8)?.checked_add(*b as usize)?;
    }
    Some((len, end))
}

fn decode_directory_string(tag: u8, value: &[u8]) -> Option<String> {
    match tag {
        // UTF8String, PrintableString, TeletexString, IA5String.
        0x0c | 0x13 | 0x14 | 0x16 => core::str::from_utf8(value).ok().map(String::from),
        // BMPString, big-endian UCS-2.
        0x1e => {
            if value.len() % 2 != 0 {
                return None;
            }
            let mut out = String::new();
            let mut offset = 0usize;
            while offset < value.len() {
                let code = u16::from_be_bytes([value[offset], value[offset + 1]]);
                out.push(char::from_u32(code as u32)?);
                offset += 2;
            }
            Some(out)
        }
        _ => None,
    }
}

#[cfg(test)]
pub fn reset_for_test() {
    *SYSTEM_KEYRINGS.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_all() {
        keys::reset_for_test();
        keys::init();
        reset_for_test();
    }

    #[test]
    fn init_allocates_builtin_trusted_keyring_and_loader_state() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let state = init_with_certificate_list(&[]).expect("system keyring init");

        assert!(state.builtin_trusted_keyring > 0);
        assert_eq!(state.loaded_certificates.len(), 0);
        assert_eq!(
            keys::describe(state.builtin_trusted_keyring).as_deref(),
            Some("keyring;0;0;3f010000;.builtin_trusted_keys")
        );
        assert!(keys::key_type_registered("asymmetric"));
    }

    #[test]
    fn x509_loader_accepts_concatenated_der_sequences() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        let certs = [
            0x30, 0x82, 0x00, 0x03, b'a', b'b', b'c', 0x30, 0x82, 0x00, 0x01, b'z',
        ];

        let state = init_with_certificate_list(&certs).expect("x509 list");

        assert_eq!(state.loaded_certificates.len(), 2);
        assert_eq!(state.loaded_certificates[0].len, 7);
        assert_eq!(state.loaded_certificates[1].len, 5);
        assert_eq!(
            state.loaded_certificates[0].description,
            "builtin X.509 certificate #0"
        );
        assert_eq!(
            keys::read(state.loaded_certificates[0].key_id),
            Err(-crate::include::uapi::errno::EOPNOTSUPP)
        );
    }

    #[test]
    fn x509_description_prefers_subject_common_name_when_present() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();
        let cert = [
            0x30, 0x82, 0x00, 0x1a, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x13, b'A', b'r', b'c',
            b'h', b' ', b'S', b'e', b'c', b'u', b'r', b'e', b' ', b'B', b'o', b'o', b't', b' ',
            b'C', b'A',
        ];

        let state = init_with_certificate_list(&cert).expect("x509 list");

        assert_eq!(state.loaded_certificates.len(), 1);
        assert_eq!(
            state.loaded_certificates[0].description,
            "Arch Secure Boot CA"
        );
        assert_eq!(
            format_loaded_x509_log(&state.loaded_certificates[0].description),
            "Loaded X.509 cert 'Arch Secure Boot CA'"
        );
    }

    #[test]
    fn x509_loader_rejects_dodgy_certificate_list() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let state =
            init_with_certificate_list(&[0x30, 0x82, 0x00]).expect("dodgy x509 list is non-fatal");

        assert!(state.builtin_trusted_keyring > 0);
        assert!(state.loaded_certificates.is_empty());
        assert_eq!(snapshot(), Some(state));
    }

    #[test]
    fn init_is_idempotent() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let first = init_with_certificate_list(&[]).expect("first init");
        let second =
            init_with_certificate_list(&[0x30, 0x82, 0x00, 0x01, b'z']).expect("second init");

        assert_eq!(first, second);
        assert_eq!(loaded_certificate_count(), 0);
        assert_eq!(
            builtin_trusted_keyring_id(),
            Some(first.builtin_trusted_keyring)
        );
    }
}
