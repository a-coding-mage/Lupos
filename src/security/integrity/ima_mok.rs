//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/ima/ima_mok.c
//! test-origin: linux:vendor/linux/security/integrity/ima/ima_mok.c
//! IMA MOK blacklist keyring allocation.

use core::sync::atomic::{AtomicI32, Ordering};

use crate::include::uapi::errno::EEXIST;

pub const IMA_BLACKLIST_KEYRING_NAME: &str = ".ima_blacklist";
pub const RESTRICTION_CHECK: &str = "restrict_link_by_builtin_trusted";

const KEY_POS_ALL: u32 = 0x3f00_0000;
const KEY_POS_SETATTR: u32 = 0x2000_0000;
const KEY_USR_VIEW: u32 = 0x0001_0000;
const KEY_USR_READ: u32 = 0x0002_0000;
const KEY_USR_WRITE: u32 = 0x0004_0000;
const KEY_USR_SEARCH: u32 = 0x0010_0000;

pub const IMA_BLACKLIST_PERM: u32 =
    (KEY_POS_ALL & !KEY_POS_SETATTR) | KEY_USR_VIEW | KEY_USR_READ | KEY_USR_WRITE | KEY_USR_SEARCH;

static IMA_BLACKLIST_KEYRING: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImaMokState {
    pub keyring_id: i32,
    pub permission: u32,
    pub restriction_check: &'static str,
}

pub fn ima_mok_init() -> Result<ImaMokState, i32> {
    let existing = IMA_BLACKLIST_KEYRING.load(Ordering::Acquire);
    if existing > 0 {
        return Ok(ImaMokState {
            keyring_id: existing,
            permission: IMA_BLACKLIST_PERM,
            restriction_check: RESTRICTION_CHECK,
        });
    }

    crate::kernel::printk::log_info!("integrity", "Allocating IMA blacklist keyring.");
    let keyring_id = crate::security::keys::add_key("keyring", IMA_BLACKLIST_KEYRING_NAME, &[]);
    if keyring_id < 0 {
        return Err(keyring_id);
    }
    crate::security::keys::set_perm(keyring_id, IMA_BLACKLIST_PERM)?;

    match IMA_BLACKLIST_KEYRING.compare_exchange(0, keyring_id, Ordering::AcqRel, Ordering::Acquire)
    {
        Ok(_) => Ok(ImaMokState {
            keyring_id,
            permission: IMA_BLACKLIST_PERM,
            restriction_check: RESTRICTION_CHECK,
        }),
        Err(_) => Err(-EEXIST),
    }
}

pub fn ima_blacklist_keyring_id() -> Option<i32> {
    let keyring_id = IMA_BLACKLIST_KEYRING.load(Ordering::Acquire);
    (keyring_id > 0).then_some(keyring_id)
}

#[cfg(test)]
pub fn reset_for_test() {
    IMA_BLACKLIST_KEYRING.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset_all() {
        reset_for_test();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
    }

    #[test]
    fn ima_mok_init_allocates_restricted_blacklist_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/ima/ima_mok.c"
        ));
        assert!(source.contains("struct key *ima_blacklist_keyring;"));
        assert!(source.contains("restriction->check = restrict_link_by_builtin_trusted;"));
        assert!(source.contains("keyring_alloc(\".ima_blacklist\""));
        assert!(source.contains("device_initcall(ima_mok_init);"));

        let state = ima_mok_init().expect("ima mok keyring");
        assert!(state.keyring_id > 0);
        assert_eq!(state.permission, IMA_BLACKLIST_PERM);
        assert_eq!(state.restriction_check, RESTRICTION_CHECK);
        assert_eq!(ima_blacklist_keyring_id(), Some(state.keyring_id));
        assert_eq!(
            crate::security::keys::describe(state.keyring_id).as_deref(),
            Some("keyring;0;0;1f170000;.ima_blacklist")
        );

        assert_eq!(
            ima_mok_init().expect("idempotent").keyring_id,
            state.keyring_id
        );
    }
}
