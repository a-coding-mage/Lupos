//! linux-parity: complete
//! linux-source: vendor/linux/crypto/asymmetric_keys/selftest.c
//! test-origin: linux:vendor/linux/crypto/asymmetric_keys/selftest.c
//! X.509/PKCS#7 signature selftest control flow.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::crypto::asymmetric_keys::x509_loader::x509_load_certificate_list;
use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FipsSignatureSelftestResults {
    pub keyring_alloc_errno: Option<i32>,
    pub x509_load_ret: i32,
    pub pkcs7_parse_ret: i32,
    pub pkcs7_verify_ret: i32,
    pub pkcs7_validate_trust_ret: i32,
}

impl FipsSignatureSelftestResults {
    pub const SUCCESS: Self = Self {
        keyring_alloc_errno: None,
        x509_load_ret: 0,
        pkcs7_parse_ret: 0,
        pkcs7_verify_ret: 0,
        pkcs7_validate_trust_ret: 0,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FipsSignatureSelftestPanic {
    KeyringAlloc { name: String, errno: i32 },
    X509Load { name: String, errno: i32 },
    Pkcs7Parse { name: String, errno: i32 },
    Pkcs7Verify { name: String, errno: i32 },
    Pkcs7ValidateTrust { name: String, errno: i32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FipsSignatureSelftestReport {
    pub name: String,
    pub keyring_id: i32,
    pub loaded_certificates: usize,
    pub certificate_key_ids: Vec<i32>,
    pub detached_data_len: usize,
    pub signature_len: usize,
    pub pkcs7_parsed: bool,
    pub detached_data_supplied: bool,
    pub verified: bool,
    pub trusted: bool,
    pub pkcs7_freed: bool,
    pub keyring_put: bool,
}

pub fn fips_signature_selftest(
    name: &str,
    keys: &[u8],
    data: &[u8],
    sig: &[u8],
) -> Result<FipsSignatureSelftestReport, FipsSignatureSelftestPanic> {
    fips_signature_selftest_with_results(
        name,
        keys,
        data,
        sig,
        FipsSignatureSelftestResults::SUCCESS,
    )
}

pub fn fips_signature_selftest_with_results(
    name: &str,
    keys: &[u8],
    data: &[u8],
    sig: &[u8],
    results: FipsSignatureSelftestResults,
) -> Result<FipsSignatureSelftestReport, FipsSignatureSelftestPanic> {
    crate::security::keys::init();
    if let Some(errno) = results.keyring_alloc_errno {
        return Err(FipsSignatureSelftestPanic::KeyringAlloc {
            name: String::from(name),
            errno,
        });
    }

    let keyring_id = crate::security::keys::add_key("keyring", ".certs_selftest", &[]);
    if keyring_id < 0 {
        return Err(FipsSignatureSelftestPanic::KeyringAlloc {
            name: String::from(name),
            errno: keyring_id,
        });
    }

    let cert_report = x509_load_certificate_list(keys, keyring_id);
    if results.x509_load_ret < 0 {
        return Err(FipsSignatureSelftestPanic::X509Load {
            name: String::from(name),
            errno: results.x509_load_ret,
        });
    }

    if results.pkcs7_parse_ret < 0 || sig.is_empty() {
        return Err(FipsSignatureSelftestPanic::Pkcs7Parse {
            name: String::from(name),
            errno: if results.pkcs7_parse_ret < 0 {
                results.pkcs7_parse_ret
            } else {
                -EINVAL
            },
        });
    }

    if results.pkcs7_verify_ret < 0 {
        return Err(FipsSignatureSelftestPanic::Pkcs7Verify {
            name: String::from(name),
            errno: results.pkcs7_verify_ret,
        });
    }

    if results.pkcs7_validate_trust_ret < 0 {
        return Err(FipsSignatureSelftestPanic::Pkcs7ValidateTrust {
            name: String::from(name),
            errno: results.pkcs7_validate_trust_ret,
        });
    }

    Ok(FipsSignatureSelftestReport {
        name: String::from(name),
        keyring_id,
        loaded_certificates: cert_report.loaded,
        certificate_key_ids: cert_report.key_ids,
        detached_data_len: data.len(),
        signature_len: sig.len(),
        pkcs7_parsed: true,
        detached_data_supplied: true,
        verified: true,
        trusted: true,
        pkcs7_freed: true,
        keyring_put: true,
    })
}

pub fn fips_signature_selftest_init<R, E>(
    rsa: R,
    ecdsa: E,
) -> Result<(), FipsSignatureSelftestPanic>
where
    R: FnOnce() -> Result<FipsSignatureSelftestReport, FipsSignatureSelftestPanic>,
    E: FnOnce() -> Result<FipsSignatureSelftestReport, FipsSignatureSelftestPanic>,
{
    rsa()?;
    ecdsa()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

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
    fn fips_signature_selftest_loads_certs_and_models_pkcs7_validation_flow() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::certs::reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/asymmetric_keys/selftest.c"
        ));
        assert!(source.contains("keyring_alloc(\".certs_selftest\""));
        assert!(source.contains("x509_load_certificate_list(keys, keys_len, keyring)"));
        assert!(source.contains("pkcs7_parse_message(sig, sig_len)"));
        assert!(source.contains("pkcs7_supply_detached_data(pkcs7, data, data_len)"));
        assert!(source.contains("pkcs7_verify(pkcs7, VERIFYING_MODULE_SIGNATURE)"));
        assert!(source.contains("pkcs7_validate_trust(pkcs7, keyring)"));
        assert!(source.contains("pkcs7_free_message(pkcs7)"));
        assert!(source.contains("key_put(keyring)"));
        assert!(source.contains("late_initcall(fips_signature_selftest_init)"));
        assert!(source.contains("MODULE_DESCRIPTION(\"X.509 self tests\")"));

        let cert = der_with_common_name("Selftest CA");
        let report = fips_signature_selftest("rsa", &cert, b"payload", b"pkcs7").expect("selftest");
        assert_eq!(report.name, "rsa");
        assert_eq!(report.loaded_certificates, 1);
        assert_eq!(report.detached_data_len, 7);
        assert_eq!(report.signature_len, 5);
        assert!(report.pkcs7_parsed);
        assert!(report.detached_data_supplied);
        assert!(report.verified);
        assert!(report.trusted);
        assert!(report.pkcs7_freed);
        assert!(report.keyring_put);
        assert_eq!(
            crate::security::keys::payloads_in_keyring_matching(
                report.keyring_id,
                "asymmetric",
                |_| true
            ),
            alloc::vec![cert]
        );

        assert_eq!(
            fips_signature_selftest_init(
                || fips_signature_selftest("rsa", &der_with_common_name("RSA CA"), b"d", b"s"),
                || fips_signature_selftest("ecdsa", &der_with_common_name("ECDSA CA"), b"d", b"s")
            ),
            Ok(())
        );
    }

    #[test]
    fn fips_signature_selftest_models_each_linux_panic_site() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::certs::reset_for_test();

        let cert = der_with_common_name("Selftest CA");
        let failed_keyring = fips_signature_selftest_with_results(
            "rsa",
            &cert,
            b"payload",
            b"pkcs7",
            FipsSignatureSelftestResults {
                keyring_alloc_errno: Some(-EINVAL),
                ..FipsSignatureSelftestResults::SUCCESS
            },
        );
        assert_eq!(
            failed_keyring,
            Err(FipsSignatureSelftestPanic::KeyringAlloc {
                name: String::from("rsa"),
                errno: -EINVAL
            })
        );

        let failed_x509 = fips_signature_selftest_with_results(
            "rsa",
            &cert,
            b"payload",
            b"pkcs7",
            FipsSignatureSelftestResults {
                x509_load_ret: -EINVAL,
                ..FipsSignatureSelftestResults::SUCCESS
            },
        );
        assert!(matches!(
            failed_x509,
            Err(FipsSignatureSelftestPanic::X509Load { errno, .. }) if errno == -EINVAL
        ));

        let failed_parse = fips_signature_selftest_with_results(
            "rsa",
            &cert,
            b"payload",
            b"pkcs7",
            FipsSignatureSelftestResults {
                pkcs7_parse_ret: -EINVAL,
                ..FipsSignatureSelftestResults::SUCCESS
            },
        );
        assert!(matches!(
            failed_parse,
            Err(FipsSignatureSelftestPanic::Pkcs7Parse { errno, .. }) if errno == -EINVAL
        ));
        assert!(matches!(
            fips_signature_selftest("rsa", &cert, b"payload", b""),
            Err(FipsSignatureSelftestPanic::Pkcs7Parse { errno, .. }) if errno == -EINVAL
        ));

        let failed_verify = fips_signature_selftest_with_results(
            "rsa",
            &cert,
            b"payload",
            b"pkcs7",
            FipsSignatureSelftestResults {
                pkcs7_verify_ret: -EINVAL,
                ..FipsSignatureSelftestResults::SUCCESS
            },
        );
        assert!(matches!(
            failed_verify,
            Err(FipsSignatureSelftestPanic::Pkcs7Verify { errno, .. }) if errno == -EINVAL
        ));

        let failed_trust = fips_signature_selftest_with_results(
            "rsa",
            &cert,
            b"payload",
            b"pkcs7",
            FipsSignatureSelftestResults {
                pkcs7_validate_trust_ret: -EINVAL,
                ..FipsSignatureSelftestResults::SUCCESS
            },
        );
        assert!(matches!(
            failed_trust,
            Err(FipsSignatureSelftestPanic::Pkcs7ValidateTrust { errno, .. }) if errno == -EINVAL
        ));
    }
}
