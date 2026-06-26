//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/platform_certs/load_ipl_s390.c
//! test-origin: linux:vendor/linux/security/integrity/platform_certs/load_ipl_s390.c
//! s390 IPL report certificate import.

use crate::include::uapi::errno::EBADMSG;

pub const IPL_CERT_SOURCE: &str = "IPL:db";

pub fn load_ipl_certs(cert_list: &[u8]) -> Result<usize, i32> {
    let mut offset = 0usize;
    let mut loaded = 0usize;

    while offset < cert_list.len() {
        let len = read_native_u32(cert_list, offset)? as usize;
        offset += core::mem::size_of::<u32>();

        let end = offset.checked_add(len).ok_or(-EBADMSG)?;
        let cert = cert_list.get(offset..end).ok_or(-EBADMSG)?;
        if crate::security::platform_certs::add_to_platform_keyring(IPL_CERT_SOURCE, cert)?
            .is_some()
        {
            loaded += 1;
        }
        offset = end;
    }

    Ok(loaded)
}

fn read_native_u32(bytes: &[u8], offset: usize) -> Result<u32, i32> {
    let end = offset
        .checked_add(core::mem::size_of::<u32>())
        .ok_or(-EBADMSG)?;
    let raw = bytes.get(offset..end).ok_or(-EBADMSG)?;
    Ok(u32::from_ne_bytes(raw.try_into().map_err(|_| -EBADMSG)?))
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

    fn append_ipl_cert(list: &mut Vec<u8>, cert: &[u8]) {
        list.extend_from_slice(&(cert.len() as u32).to_ne_bytes());
        list.extend_from_slice(cert);
    }

    #[test]
    fn load_ipl_certs_imports_length_prefixed_certs_to_platform_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let first = der_with_common_name("s390 IPL CA");
        let second = der_with_common_name("s390 IPL Backup CA");
        let mut report = Vec::new();
        append_ipl_cert(&mut report, &first);
        append_ipl_cert(&mut report, &second);

        assert_eq!(load_ipl_certs(&report), Ok(2));

        let state = crate::security::platform_certs::snapshot().expect("platform keyring");
        assert_eq!(state.loaded_certificates.len(), 2);
        assert_eq!(state.loaded_certificates[0].source, IPL_CERT_SOURCE);
        assert_eq!(state.loaded_certificates[0].description, "s390 IPL CA");
        assert_eq!(
            state.loaded_certificates[1].description,
            "s390 IPL Backup CA"
        );
    }

    #[test]
    fn load_ipl_certs_matches_source_and_rejects_truncated_records() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/load_ipl_s390.c"
        ));
        assert!(source.contains("if (!ipl_cert_list_addr)"));
        assert!(source.contains("len = *(unsigned int *) ptr;"));
        assert!(source.contains("add_to_platform_keyring(\"IPL:db\", ptr, len);"));

        assert_eq!(load_ipl_certs(&[]), Ok(0));
        assert_eq!(load_ipl_certs(&[1, 0, 0]), Err(-EBADMSG));

        let mut truncated = Vec::new();
        truncated.extend_from_slice(&8u32.to_ne_bytes());
        truncated.extend_from_slice(b"short");
        assert_eq!(load_ipl_certs(&truncated), Err(-EBADMSG));
    }
}
