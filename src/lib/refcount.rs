//! linux-parity: partial
//! linux-source: vendor/linux/lib/refcount.c
//! test-origin: linux:vendor/linux/lib/refcount.c
//! Linux refcount helper exports.

use core::sync::atomic::{AtomicI32, Ordering};

use crate::kernel::locking::irqflags::{IrqFlags, local_irq_restore, local_irq_save};
use crate::kernel::locking::mutex::{LinuxRawMutex, linux_mutex_lock, linux_mutex_unlock};
use crate::kernel::locking::preempt::{preempt_disable, preempt_enable};
use crate::kernel::locking::qspinlock::QSpinLock;
use crate::kernel::module::{export_symbol, find_symbol};

const REFCOUNT_SATURATED: i32 = i32::MIN / 2;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "refcount_warn_saturate",
        linux_refcount_warn_saturate as usize,
        false,
    );
    export_symbol_once(
        "refcount_dec_not_one",
        linux_refcount_dec_not_one as usize,
        false,
    );
    export_symbol_once(
        "refcount_dec_and_lock",
        linux_refcount_dec_and_lock as usize,
        false,
    );
    export_symbol_once(
        "refcount_dec_and_mutex_lock",
        linux_refcount_dec_and_mutex_lock as usize,
        false,
    );
    export_symbol_once(
        "refcount_dec_and_lock_irqsave",
        linux_refcount_dec_and_lock_irqsave as usize,
        false,
    );
}

/// `refcount_warn_saturate` - `vendor/linux/lib/refcount.c:10`.
pub unsafe extern "C" fn linux_refcount_warn_saturate(refcount: *mut AtomicI32, _type_: i32) {
    if !refcount.is_null() {
        unsafe { &*refcount }.store(REFCOUNT_SATURATED, Ordering::Release);
    }
}

/// `refcount_dec_not_one` - `vendor/linux/lib/refcount.c:74`.
pub unsafe extern "C" fn linux_refcount_dec_not_one(refcount: *mut AtomicI32) -> bool {
    if refcount.is_null() {
        return false;
    }

    let refs = unsafe { &*refcount };
    let mut val = refs.load(Ordering::Acquire);
    loop {
        if val == REFCOUNT_SATURATED {
            return true;
        }
        if val <= 0 {
            return true;
        }
        if val == 1 {
            return false;
        }

        let new = val - 1;
        match refs.compare_exchange_weak(val, new, Ordering::Release, Ordering::Acquire) {
            Ok(_) => return true,
            Err(observed) => val = observed,
        }
    }
}

fn refcount_dec_and_test(refs: &AtomicI32) -> bool {
    let mut val = refs.load(Ordering::Acquire);
    loop {
        if val == REFCOUNT_SATURATED || val <= 0 {
            return false;
        }
        let new = val - 1;
        match refs.compare_exchange_weak(val, new, Ordering::Release, Ordering::Acquire) {
            Ok(_) => return new == 0,
            Err(observed) => val = observed,
        }
    }
}

fn spin_lock(lock: &QSpinLock) {
    preempt_disable();
    lock.lock();
}

fn spin_unlock(lock: &QSpinLock) {
    lock.unlock();
    preempt_enable();
}

fn spin_lock_irqsave(lock: &QSpinLock, flags: &mut IrqFlags) {
    *flags = local_irq_save();
    preempt_disable();
    lock.lock();
}

fn spin_unlock_irqrestore(lock: &QSpinLock, flags: IrqFlags) {
    lock.unlock();
    preempt_enable();
    local_irq_restore(flags);
}

/// `refcount_dec_and_lock` - `vendor/linux/lib/refcount.c:119`.
pub unsafe extern "C" fn linux_refcount_dec_and_lock(
    refcount: *mut AtomicI32,
    lock: *mut QSpinLock,
) -> bool {
    if refcount.is_null() || lock.is_null() {
        return false;
    }
    if unsafe { linux_refcount_dec_not_one(refcount) } {
        return false;
    }

    let refs = unsafe { &*refcount };
    let lock = unsafe { &*lock };
    spin_lock(lock);
    if refcount_dec_and_test(refs) {
        return true;
    }
    spin_unlock(lock);
    false
}

/// `refcount_dec_and_mutex_lock` - `vendor/linux/lib/refcount.c:113`.
pub unsafe extern "C" fn linux_refcount_dec_and_mutex_lock(
    refcount: *mut AtomicI32,
    lock: *mut LinuxRawMutex,
) -> bool {
    if refcount.is_null() || lock.is_null() {
        return false;
    }
    if unsafe { linux_refcount_dec_not_one(refcount) } {
        return false;
    }

    let refs = unsafe { &*refcount };
    unsafe { linux_mutex_lock(lock) };
    if refcount_dec_and_test(refs) {
        return true;
    }
    unsafe { linux_mutex_unlock(lock) };
    false
}

/// `refcount_dec_and_lock_irqsave` - `vendor/linux/lib/refcount.c:143`.
pub unsafe extern "C" fn linux_refcount_dec_and_lock_irqsave(
    refcount: *mut AtomicI32,
    lock: *mut QSpinLock,
    flags: *mut IrqFlags,
) -> bool {
    if refcount.is_null() || lock.is_null() || flags.is_null() {
        return false;
    }
    if unsafe { linux_refcount_dec_not_one(refcount) } {
        return false;
    }

    let refs = unsafe { &*refcount };
    let lock = unsafe { &*lock };
    let flags = unsafe { &mut *flags };
    spin_lock_irqsave(lock, flags);
    if refcount_dec_and_test(refs) {
        return true;
    }
    spin_unlock_irqrestore(lock, *flags);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_not_one_decrements_above_one() {
        let refs = AtomicI32::new(3);

        assert!(unsafe { linux_refcount_dec_not_one((&refs as *const AtomicI32).cast_mut()) });
        assert_eq!(refs.load(Ordering::Acquire), 2);
    }

    #[test]
    fn dec_not_one_leaves_one_unchanged() {
        let refs = AtomicI32::new(1);

        assert!(!unsafe { linux_refcount_dec_not_one((&refs as *const AtomicI32).cast_mut()) });
        assert_eq!(refs.load(Ordering::Acquire), 1);
    }

    #[test]
    fn refcount_exports_match_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/refcount.c"
        ));
        assert!(source.contains("void refcount_warn_saturate"));
        assert!(source.contains("bool refcount_dec_and_lock"));
        assert!(source.contains("spin_lock(lock);"));
        assert!(source.contains("EXPORT_SYMBOL(refcount_warn_saturate);"));
        assert!(source.contains("EXPORT_SYMBOL(refcount_dec_and_lock);"));
        assert!(source.contains("bool refcount_dec_and_mutex_lock"));
        assert!(source.contains("EXPORT_SYMBOL(refcount_dec_and_mutex_lock);"));
        assert!(source.contains("bool refcount_dec_and_lock_irqsave"));
        assert!(source.contains("spin_lock_irqsave(lock, *flags);"));
        assert!(source.contains("EXPORT_SYMBOL(refcount_dec_and_lock_irqsave);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("refcount_warn_saturate"),
            Some(linux_refcount_warn_saturate as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("refcount_dec_not_one"),
            Some(linux_refcount_dec_not_one as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("refcount_dec_and_lock"),
            Some(linux_refcount_dec_and_lock as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("refcount_dec_and_mutex_lock"),
            Some(linux_refcount_dec_and_mutex_lock as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("refcount_dec_and_lock_irqsave"),
            Some(linux_refcount_dec_and_lock_irqsave as usize)
        );
    }

    #[test]
    fn dec_not_one_does_not_underflow_zero() {
        let refs = AtomicI32::new(0);

        assert!(unsafe { linux_refcount_dec_not_one((&refs as *const AtomicI32).cast_mut()) });
        assert_eq!(refs.load(Ordering::Acquire), 0);
    }

    #[test]
    fn warn_saturate_sets_saturated_value() {
        let refs = AtomicI32::new(2);

        unsafe { linux_refcount_warn_saturate((&refs as *const AtomicI32).cast_mut(), 0) };

        assert_eq!(refs.load(Ordering::Acquire), REFCOUNT_SATURATED);
    }

    #[test]
    fn dec_and_lock_fast_path_decrements_without_locking() {
        let refs = AtomicI32::new(3);
        let mut lock = QSpinLock::new();

        let ret = unsafe {
            linux_refcount_dec_and_lock((&refs as *const AtomicI32).cast_mut(), &mut lock)
        };

        assert!(!ret);
        assert_eq!(refs.load(Ordering::Acquire), 2);
        assert!(!lock.is_locked());
    }

    #[test]
    fn dec_and_lock_slow_path_returns_with_lock_held() {
        let refs = AtomicI32::new(1);
        let mut lock = QSpinLock::new();

        let ret = unsafe {
            linux_refcount_dec_and_lock((&refs as *const AtomicI32).cast_mut(), &mut lock)
        };

        assert!(ret);
        assert_eq!(refs.load(Ordering::Acquire), 0);
        assert!(lock.is_locked());
        unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_unlock(&mut lock) };
    }

    #[test]
    fn dec_and_lock_irqsave_fast_path_decrements_without_locking() {
        let refs = AtomicI32::new(3);
        let mut lock = QSpinLock::new();
        let mut flags = IrqFlags::MAX;

        let ret = unsafe {
            linux_refcount_dec_and_lock_irqsave(
                (&refs as *const AtomicI32).cast_mut(),
                &mut lock,
                &mut flags,
            )
        };

        assert!(!ret);
        assert_eq!(refs.load(Ordering::Acquire), 2);
        assert!(!lock.is_locked());
        assert_eq!(flags, IrqFlags::MAX);
    }

    #[test]
    fn dec_and_lock_irqsave_slow_path_returns_with_lock_held() {
        let refs = AtomicI32::new(1);
        let mut lock = QSpinLock::new();
        let mut flags = IrqFlags::MAX;

        let ret = unsafe {
            linux_refcount_dec_and_lock_irqsave(
                (&refs as *const AtomicI32).cast_mut(),
                &mut lock,
                &mut flags,
            )
        };

        assert!(ret);
        assert_eq!(refs.load(Ordering::Acquire), 0);
        assert!(lock.is_locked());
        unsafe {
            crate::kernel::locking::raw_spinlock::linux_raw_spin_unlock_irqrestore(&mut lock, flags)
        };
    }
}
