//! linux-parity: complete
//! linux-source: vendor/linux/security/landlock/cred.c
//! test-origin: linux:vendor/linux/security/landlock/cred.c
//! Landlock credential hook state transfer.

use core::sync::atomic::{AtomicBool, Ordering};

use super::LandlockRuleset;

static CRED_HOOKS_REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LandlockCredSecurity {
    pub domain: Option<LandlockRuleset>,
    pub domain_exec: u16,
    pub log_subdomains_off: bool,
}

pub fn hook_cred_transfer(new: &mut LandlockCredSecurity, old: &LandlockCredSecurity) {
    *new = old.clone();
}

pub fn hook_cred_prepare(
    new: &mut LandlockCredSecurity,
    old: &LandlockCredSecurity,
) -> Result<(), i32> {
    hook_cred_transfer(new, old);
    Ok(())
}

pub fn hook_cred_free(cred: &mut LandlockCredSecurity) {
    cred.domain = None;
}

pub fn hook_bprm_creds_for_exec(cred: &mut LandlockCredSecurity) -> Result<(), i32> {
    cred.domain_exec = 0;
    Ok(())
}

pub fn landlock_add_cred_hooks() {
    CRED_HOOKS_REGISTERED.store(true, Ordering::Release);
}

pub fn cred_hooks_registered() -> bool {
    CRED_HOOKS_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn reset_for_test() {
    CRED_HOOKS_REGISTERED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::landlock::{LANDLOCK_ACCESS_FS_READ_FILE, create_ruleset};

    #[test]
    fn landlock_cred_hooks_transfer_and_free_domain_state() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::landlock::reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/cred.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/cred.h"
        ));
        assert!(source.contains("LSM_HOOK_INIT(cred_prepare, hook_cred_prepare)"));
        assert!(source.contains("LSM_HOOK_INIT(cred_transfer, hook_cred_transfer)"));
        assert!(source.contains("LSM_HOOK_INIT(cred_free, hook_cred_free)"));
        assert!(source.contains("domain_exec = 0"));
        assert!(header.contains("struct landlock_cred_security"));

        let id = create_ruleset(LANDLOCK_ACCESS_FS_READ_FILE);
        let old = LandlockCredSecurity {
            domain: Some(
                crate::security::landlock::ruleset_snapshot(id).expect("ruleset snapshot"),
            ),
            domain_exec: 0xffff,
            log_subdomains_off: true,
        };
        let mut new = LandlockCredSecurity::default();
        assert_eq!(hook_cred_prepare(&mut new, &old), Ok(()));
        assert_eq!(new, old);
        assert_eq!(hook_bprm_creds_for_exec(&mut new), Ok(()));
        assert_eq!(new.domain_exec, 0);
        hook_cred_free(&mut new);
        assert!(new.domain.is_none());

        landlock_add_cred_hooks();
        assert!(cred_hooks_registered());
    }
}
