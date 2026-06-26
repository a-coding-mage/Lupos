//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/ima/ima_asymmetric_keys.c
//! test-origin: linux:vendor/linux/security/integrity/ima/ima_asymmetric_keys.c
//! IMA hook for measuring asymmetric key payloads.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

pub const KEY_TYPE_ASYMMETRIC: &str = "asymmetric";

static IMA_PROCESS_KEYS: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref QUEUED_KEYS: Mutex<Vec<ImaQueuedKey>> = Mutex::new(Vec::new());
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImaQueuedKey {
    pub keyring_name: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImaKeyPostAction {
    Ignored,
    Queued,
    Measured(bool),
}

pub fn ima_should_queue_key() -> bool {
    !IMA_PROCESS_KEYS.load(Ordering::Acquire)
}

pub fn ima_queue_key(keyring_description: &str, payload: &[u8]) -> bool {
    if !ima_should_queue_key() {
        return false;
    }
    QUEUED_KEYS.lock().push(ImaQueuedKey {
        keyring_name: String::from(keyring_description),
        payload: payload.to_vec(),
    });
    true
}

pub fn ima_process_queued_keys() -> Result<usize, i32> {
    if IMA_PROCESS_KEYS.swap(true, Ordering::AcqRel) {
        return Ok(0);
    }

    let queued = {
        let mut guard = QUEUED_KEYS.lock();
        core::mem::take(&mut *guard)
    };
    let mut measured = 0usize;
    for entry in queued {
        if crate::security::integrity::ima::measure_buffer_for_keyring(
            &entry.keyring_name,
            &entry.keyring_name,
            &entry.payload,
        )? {
            measured += 1;
        }
    }
    Ok(measured)
}

pub fn ima_post_key_create_or_update(
    keyring_description: &str,
    key_type: &str,
    payload: Option<&[u8]>,
    _flags: u64,
    _create: bool,
) -> Result<ImaKeyPostAction, i32> {
    if key_type != KEY_TYPE_ASYMMETRIC {
        return Ok(ImaKeyPostAction::Ignored);
    }
    let Some(payload) = payload else {
        return Ok(ImaKeyPostAction::Ignored);
    };
    if payload.is_empty() {
        return Ok(ImaKeyPostAction::Ignored);
    }

    if ima_should_queue_key() && ima_queue_key(keyring_description, payload) {
        return Ok(ImaKeyPostAction::Queued);
    }

    crate::security::integrity::ima::measure_buffer_for_keyring(
        keyring_description,
        keyring_description,
        payload,
    )
    .map(ImaKeyPostAction::Measured)
}

pub fn queued_key_count() -> usize {
    QUEUED_KEYS.lock().len()
}

#[cfg(test)]
pub fn reset_for_test() {
    IMA_PROCESS_KEYS.store(false, Ordering::Release);
    QUEUED_KEYS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn ima_asymmetric_key_hook_filters_queues_and_measures_by_keyring_policy() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_for_test();
        crate::security::integrity::ima::reset_for_test();
        crate::security::integrity::ima::init();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/ima/ima_asymmetric_keys.c"
        ));
        assert!(source.contains("key->type != &key_type_asymmetric"));
        assert!(source.contains("if (!payload || (payload_len == 0))"));
        assert!(source.contains("ima_queue_key(keyring, payload, payload_len)"));
        assert!(source.contains("process_buffer_measurement"));
        assert!(source.contains("keyring->description"));

        assert_eq!(
            ima_post_key_create_or_update(".ima", "user", Some(b"payload"), 0, true),
            Ok(ImaKeyPostAction::Ignored)
        );
        assert_eq!(
            ima_post_key_create_or_update(".ima", KEY_TYPE_ASYMMETRIC, None, 0, true),
            Ok(ImaKeyPostAction::Ignored)
        );
        assert_eq!(
            ima_post_key_create_or_update(".ima", KEY_TYPE_ASYMMETRIC, Some(b""), 0, true),
            Ok(ImaKeyPostAction::Ignored)
        );

        let policy = b"measure func=KEY_CHECK keyrings=.ima pcr=10\n";
        assert_eq!(
            crate::security::integrity::ima::load_policy(policy),
            Ok(policy.len())
        );
        assert_eq!(
            ima_post_key_create_or_update(".ima", KEY_TYPE_ASYMMETRIC, Some(b"cert"), 0, true),
            Ok(ImaKeyPostAction::Queued)
        );
        assert_eq!(queued_key_count(), 1);
        assert_eq!(ima_process_queued_keys(), Ok(1));
        assert_eq!(queued_key_count(), 0);
        let ascii = crate::security::integrity::ima::ascii_runtime_measurements_sha1();
        assert!(ascii.contains(".ima"));

        assert_eq!(
            ima_post_key_create_or_update(
                ".builtin_trusted_keys",
                KEY_TYPE_ASYMMETRIC,
                Some(b"cert2"),
                0,
                false
            ),
            Ok(ImaKeyPostAction::Measured(false))
        );
    }
}
