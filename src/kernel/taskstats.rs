//! linux-parity: partial
//! linux-source: vendor/linux/kernel/taskstats.c
//! test-origin: linux:vendor/linux/kernel/taskstats.c
//! Taskstats generic-netlink registration shim.
//!
//! Linux prepares taskstats bookkeeping from `taskstats_init_early()` in
//! `start_kernel`, then registers the generic-netlink family from
//! `taskstats_init()` as a `late_initcall`. Lupos does not yet export full
//! per-task accounting, but PID 1 and boot triage expect the registration
//! surface and boot line to exist.

use core::sync::atomic::{AtomicBool, Ordering};

/// Mirrors `vendor/linux/include/uapi/linux/taskstats.h`.
pub const TASKSTATS_GENL_VERSION: u8 = 0x1;
pub const TASKSTATS_REGISTERED_LOG: &str = "registered taskstats version 1";

static REGISTERED: AtomicBool = AtomicBool::new(false);
static EARLY_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Prepare taskstats bookkeeping once.
pub fn init_early() {
    EARLY_INITIALIZED.store(true, Ordering::Release);
}

/// Register the taskstats generic-netlink family once.
pub fn init() {
    if !REGISTERED.swap(true, Ordering::AcqRel) {
        crate::log_info!("", "{}", TASKSTATS_REGISTERED_LOG);
    }
}

pub fn is_registered() -> bool {
    REGISTERED.load(Ordering::Acquire)
}

pub fn is_early_initialized() -> bool {
    EARLY_INITIALIZED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskstats_version_matches_linux_uapi() {
        assert_eq!(TASKSTATS_GENL_VERSION, 1);
        assert_eq!(TASKSTATS_REGISTERED_LOG, "registered taskstats version 1");
    }

    #[test]
    fn taskstats_init_is_idempotent() {
        EARLY_INITIALIZED.store(false, Ordering::Release);
        REGISTERED.store(false, Ordering::Release);
        assert!(!is_early_initialized());
        assert!(!is_registered());
        init_early();
        assert!(is_early_initialized());
        init_early();
        assert!(is_early_initialized());
        init();
        assert!(is_registered());
        init();
        assert!(is_registered());
        REGISTERED.store(false, Ordering::Release);
        EARLY_INITIALIZED.store(false, Ordering::Release);
    }
}
