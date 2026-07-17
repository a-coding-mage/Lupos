//! linux-parity: complete
//! linux-source: vendor/linux/crypto/asymmetric_keys/x509_loader.c
//! test-origin: linux:vendor/linux/crypto/asymmetric_keys/x509_loader.c
//! In-kernel X.509 certificate-list loader.

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X509LoadReport {
    pub loaded: usize,
    pub dodgy_cert: bool,
    pub key_ids: Vec<i32>,
}

impl X509LoadReport {
    pub const fn linux_return_code(&self) -> i32 {
        0
    }
}

pub fn x509_load_certificate_list(cert_list: &[u8], keyring_id: i32) -> X509LoadReport {
    let mut offset = 0usize;
    let mut report = X509LoadReport {
        loaded: 0,
        dodgy_cert: false,
        key_ids: Vec::new(),
    };

    while offset < cert_list.len() {
        let remaining = cert_list.len() - offset;
        if remaining < 4 || cert_list[offset] != 0x30 || cert_list[offset + 1] != 0x82 {
            report.dodgy_cert = true;
            break;
        }

        let plen = (((cert_list[offset + 2] as usize) << 8) | cert_list[offset + 3] as usize) + 4;
        if plen > remaining {
            report.dodgy_cert = true;
            break;
        }

        let cert = &cert_list[offset..offset + plen];
        let description = crate::security::certs::x509_certificate_description(cert)
            .unwrap_or_else(|| format!("builtin X.509 certificate #{}", report.loaded));
        if let Some(loaded) = crate::security::certs::load_x509_certificate(&description, cert) {
            if crate::security::keys::link_key_to_keyring(loaded.key_id, keyring_id).is_ok() {
                crate::kernel::printk::log_info!(
                    "",
                    "{}",
                    crate::security::certs::format_loaded_x509_log(&description)
                );
                report.loaded += 1;
                report.key_ids.push(loaded.key_id);
            }
        }

        offset += plen;
    }

    if report.dodgy_cert {
        crate::kernel::printk::log_info!("", "Problem parsing in-kernel X.509 certificate list");
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset_all() -> i32 {
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        crate::security::certs::reset_for_test();
        crate::security::keys::add_key("keyring", ".builtin_trusted_keys", &[])
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
    fn x509_loader_imports_concatenated_certificates_into_keyring() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        let keyring = reset_all();
        assert!(keyring > 0);

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/asymmetric_keys/x509_loader.c"
        ));
        assert!(source.contains("int x509_load_certificate_list"));
        assert!(source.contains("if (end - p < 4)"));
        assert!(source.contains("if (p[0] != 0x30 ||"));
        assert!(source.contains("key_create_or_update(make_key_ref(keyring, 1)"));
        assert!(source.contains("KEY_ALLOC_BYPASS_RESTRICTION"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(x509_load_certificate_list);"));

        let first = der_with_common_name("Builtin CA");
        let second = der_with_common_name("Backup CA");
        let mut certs = Vec::new();
        certs.extend_from_slice(&first);
        certs.extend_from_slice(&second);

        let report = x509_load_certificate_list(&certs, keyring);
        assert_eq!(report.linux_return_code(), 0);
        assert_eq!(report.loaded, 2);
        assert!(!report.dodgy_cert);
        assert_eq!(report.key_ids.len(), 2);
        assert_eq!(
            crate::security::keys::payloads_in_keyring_matching(keyring, "asymmetric", |_| true),
            vec![first, second]
        );
    }

    #[test]
    fn x509_loader_reports_dodgy_list_but_keeps_linux_zero_return() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        let keyring = reset_all();
        let report = x509_load_certificate_list(&[0x30, 0x82, 0x00], keyring);

        assert_eq!(report.linux_return_code(), 0);
        assert_eq!(report.loaded, 0);
        assert!(report.dodgy_cert);
    }

    #[test]
    fn x509_loader_rejects_either_invalid_der_prefix_byte() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        let keyring = reset_all();

        for prefix in [[0x31, 0x82, 0x00, 0x00], [0x30, 0x81, 0x00, 0x00]] {
            let report = x509_load_certificate_list(&prefix, keyring);
            assert_eq!(report.loaded, 0);
            assert!(report.dodgy_cert);
        }
    }
}
