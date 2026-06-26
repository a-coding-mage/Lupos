//! linux-parity: complete
//! linux-source: vendor/linux/security
//! test-origin: linux:vendor/linux/security
//! Registered LSM list.  Fixed-size array, max 8 LSMs.

use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use super::hooks::{LsmHooks, NOOP_HOOKS};

const MAX_LSMS: usize = 8;

static LSM_TABLE: Mutex<[LsmHooks; MAX_LSMS]> = Mutex::new([NOOP_HOOKS; MAX_LSMS]);
static LSM_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub static TEST_LSM_LOCK: Mutex<()> = Mutex::new(());

pub fn register_lsm(hooks: LsmHooks) -> Result<(), i32> {
    let mut g = LSM_TABLE.lock();
    let n = LSM_COUNT.load(Ordering::Acquire);
    if n >= MAX_LSMS {
        return Err(-28); // ENOSPC
    }
    // Don't double-register.
    for i in 0..n {
        if g[i].name == hooks.name {
            return Err(-17); // EEXIST
        }
    }
    g[n] = hooks;
    LSM_COUNT.store(n + 1, Ordering::Release);
    Ok(())
}

pub fn lsm_active_count() -> usize {
    LSM_COUNT.load(Ordering::Acquire)
}

pub fn lsm_active_ids(out: &mut [u64]) -> usize {
    let g = LSM_TABLE.lock();
    let n = LSM_COUNT.load(Ordering::Acquire);
    let count = n.min(out.len());
    for i in 0..count {
        out[i] = g[i].id;
    }
    n
}

/// Walk the registered LSM list, calling `f` on each registered hooks.
/// Returns the first non-zero value, otherwise 0.
pub(super) fn call_int_hook(mut f: impl FnMut(&LsmHooks) -> i32) -> i32 {
    let g = LSM_TABLE.lock();
    let n = LSM_COUNT.load(Ordering::Acquire);
    for i in 0..n {
        let r = f(&g[i]);
        if r != 0 {
            return r;
        }
    }
    0
}

/// Like `call_int_hook` but with no return value (e.g. notification hooks).
pub(super) fn call_void_hook(mut f: impl FnMut(&LsmHooks)) {
    let g = LSM_TABLE.lock();
    let n = LSM_COUNT.load(Ordering::Acquire);
    for i in 0..n {
        f(&g[i]);
    }
}

/// Test-only: drain all registered LSMs.
#[cfg(test)]
pub fn reset_for_test() {
    let mut g = LSM_TABLE.lock();
    for i in 0..MAX_LSMS {
        g[i] = NOOP_HOOKS;
    }
    LSM_COUNT.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::hooks::{LSM_ID_CAPABILITY, LsmHooks, NOOP_HOOKS};

    #[test]
    fn active_ids_return_linux_lsm_ids() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        register_lsm(LsmHooks {
            name: "capability",
            id: LSM_ID_CAPABILITY,
            ..NOOP_HOOKS
        })
        .expect("register");
        let mut ids = [0u64; 4];
        assert_eq!(lsm_active_ids(&mut ids), 1);
        assert_eq!(ids[0], LSM_ID_CAPABILITY);
    }
}
