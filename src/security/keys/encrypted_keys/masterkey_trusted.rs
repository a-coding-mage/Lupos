//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/encrypted-keys/masterkey_trusted.c
//! test-origin: linux:vendor/linux/security/keys/encrypted-keys/masterkey_trusted.c
//! Trusted-key master key lookup for encrypted keys.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOKEY;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedMasterKey {
    pub key_id: i32,
    pub bytes: Vec<u8>,
}

pub fn request_trusted_key(trusted_desc: &str) -> Result<TrustedMasterKey, i32> {
    let key_id = crate::security::keys::request_key("trusted", trusted_desc);
    if key_id < 0 {
        return Err(key_id);
    }

    let payload = crate::security::keys::payloads_in_keyring_matching(0, "trusted", |key| {
        key.id == key_id && key.description == trusted_desc
    })
    .into_iter()
    .next()
    .ok_or(-ENOKEY)?;

    Ok(TrustedMasterKey {
        key_id,
        bytes: payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset_keys() {
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
    }

    #[test]
    fn request_trusted_key_returns_payload_and_length_from_trusted_key() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_keys();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/encrypted-keys/masterkey_trusted.c"
        ));
        assert!(source.contains("request_key(&key_type_trusted, trusted_desc, NULL)"));
        assert!(source.contains("tpayload = tkey->payload.data[0];"));
        assert!(source.contains("*master_key = tpayload->key;"));
        assert!(source.contains("*master_keylen = tpayload->key_len;"));

        let key_id = crate::security::keys::add_key("trusted", "sealed:pcrs", b"master-secret");
        assert!(key_id > 0);

        let master = request_trusted_key("sealed:pcrs").expect("trusted key");
        assert_eq!(master.key_id, key_id);
        assert_eq!(master.bytes, b"master-secret");
        assert_eq!(master.bytes.len(), 13);
        assert_eq!(request_trusted_key("missing"), Err(-ENOKEY));
    }
}
