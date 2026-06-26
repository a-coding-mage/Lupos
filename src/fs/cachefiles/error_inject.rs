//! linux-parity: complete
//! linux-source: vendor/linux/fs/cachefiles/error_inject.c
//! test-origin: linux:vendor/linux/fs/cachefiles/error_inject.c
//! cachefiles error-injection sysctl state.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::include::uapi::errno::ENOMEM;

pub static CACHEFILES_ERROR_INJECTION_STATE: AtomicU32 = AtomicU32::new(0);
static CACHEFILES_SYSCTL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn cachefiles_register_error_injection(register_sysctl_ok: bool) -> Result<(), i32> {
    if register_sysctl_ok {
        CACHEFILES_SYSCTL_REGISTERED.store(true, Ordering::Release);
        Ok(())
    } else {
        Err(-ENOMEM)
    }
}

pub fn cachefiles_unregister_error_injection() {
    CACHEFILES_SYSCTL_REGISTERED.store(false, Ordering::Release);
}

pub fn cachefiles_sysctl_registered() -> bool {
    CACHEFILES_SYSCTL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cachefiles_error_injection_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/cachefiles/error_inject.c"
        ));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("unsigned int cachefiles_error_injection_state;"));
        assert!(source.contains(".procname"));
        assert!(source.contains("\"error_injection\""));
        assert!(source.contains("register_sysctl(\"cachefiles\", cachefiles_sysctls);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_sysctl_table(cachefiles_sysctl);"));
        CACHEFILES_ERROR_INJECTION_STATE.store(7, Ordering::Release);
        assert_eq!(CACHEFILES_ERROR_INJECTION_STATE.load(Ordering::Acquire), 7);
        assert_eq!(cachefiles_register_error_injection(false), Err(-ENOMEM));
        assert_eq!(cachefiles_register_error_injection(true), Ok(()));
        assert!(cachefiles_sysctl_registered());
        cachefiles_unregister_error_injection();
        assert!(!cachefiles_sysctl_registered());
    }
}
