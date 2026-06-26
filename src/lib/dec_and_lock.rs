//! linux-parity: complete
//! linux-source: vendor/linux/lib/dec_and_lock.c
//! test-origin: linux:vendor/linux/lib/dec_and_lock.c
//! Atomic decrement-and-lock helpers.

use core::sync::atomic::{AtomicI32, Ordering};

use crate::kernel::locking::irqflags::{IrqFlags, local_irq_restore, local_irq_save};
use crate::kernel::locking::preempt::{preempt_disable, preempt_enable};
use crate::kernel::locking::raw_spinlock::RawSpinLock;
use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
pub struct AtomicT {
    counter: AtomicI32,
}

impl AtomicT {
    pub const fn new(value: i32) -> Self {
        Self {
            counter: AtomicI32::new(value),
        }
    }

    pub fn read(&self) -> i32 {
        self.counter.load(Ordering::Acquire)
    }
}

#[repr(C)]
pub struct SpinlockT {
    raw: RawSpinLock,
}

impl SpinlockT {
    pub const fn new() -> Self {
        Self {
            raw: RawSpinLock::new(),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.raw.is_locked()
    }

    pub fn unlock(&self) {
        spin_unlock(self);
    }

    pub fn unlock_irqrestore(&self, flags: IrqFlags) {
        spin_unlock_irqrestore(self, flags);
    }
}

impl Default for SpinlockT {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
pub struct RawSpinlockT {
    raw: RawSpinLock,
}

impl RawSpinlockT {
    pub const fn new() -> Self {
        Self {
            raw: RawSpinLock::new(),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.raw.is_locked()
    }

    pub fn unlock(&self) {
        raw_spin_unlock(self);
    }

    pub fn unlock_irqrestore(&self, flags: IrqFlags) {
        raw_spin_unlock_irqrestore(self, flags);
    }
}

impl Default for RawSpinlockT {
    fn default() -> Self {
        Self::new()
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("atomic_dec_and_lock", atomic_dec_and_lock as usize, false);
    export_symbol_once(
        "_atomic_dec_and_lock_irqsave",
        _atomic_dec_and_lock_irqsave as usize,
        false,
    );
    export_symbol_once(
        "atomic_dec_and_raw_lock",
        atomic_dec_and_raw_lock as usize,
        false,
    );
    export_symbol_once(
        "_atomic_dec_and_raw_lock_irqsave",
        _atomic_dec_and_raw_lock_irqsave as usize,
        false,
    );
}

fn atomic_add_unless(atomic: &AtomicT, add: i32, unless: i32) -> bool {
    let mut current = atomic.counter.load(Ordering::Acquire);
    loop {
        if current == unless {
            return false;
        }
        match atomic.counter.compare_exchange(
            current,
            current.wrapping_add(add),
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(next) => current = next,
        }
    }
}

fn atomic_dec_and_test(atomic: &AtomicT) -> bool {
    atomic.counter.fetch_sub(1, Ordering::AcqRel) == 1
}

fn spin_lock(lock: &SpinlockT) {
    preempt_disable();
    lock.raw.lock();
}

fn spin_unlock(lock: &SpinlockT) {
    lock.raw.unlock();
    preempt_enable();
}

fn spin_lock_irqsave(lock: &SpinlockT, flags: &mut IrqFlags) {
    *flags = local_irq_save();
    preempt_disable();
    lock.raw.lock();
}

fn spin_unlock_irqrestore(lock: &SpinlockT, flags: IrqFlags) {
    lock.raw.unlock();
    preempt_enable();
    local_irq_restore(flags);
}

fn raw_spin_lock(lock: &RawSpinlockT) {
    preempt_disable();
    lock.raw.lock();
}

fn raw_spin_unlock(lock: &RawSpinlockT) {
    lock.raw.unlock();
    preempt_enable();
}

fn raw_spin_lock_irqsave(lock: &RawSpinlockT, flags: &mut IrqFlags) {
    *flags = local_irq_save();
    preempt_disable();
    lock.raw.lock();
}

fn raw_spin_unlock_irqrestore(lock: &RawSpinlockT, flags: IrqFlags) {
    lock.raw.unlock();
    preempt_enable();
    local_irq_restore(flags);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atomic_dec_and_lock(atomic: *mut AtomicT, lock: *mut SpinlockT) -> i32 {
    let atomic = unsafe { &*atomic };
    let lock = unsafe { &*lock };

    if atomic_add_unless(atomic, -1, 1) {
        return 0;
    }

    spin_lock(lock);
    if atomic_dec_and_test(atomic) {
        return 1;
    }
    spin_unlock(lock);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _atomic_dec_and_lock_irqsave(
    atomic: *mut AtomicT,
    lock: *mut SpinlockT,
    flags: *mut IrqFlags,
) -> i32 {
    let atomic = unsafe { &*atomic };
    let lock = unsafe { &*lock };
    let flags = unsafe { &mut *flags };

    if atomic_add_unless(atomic, -1, 1) {
        return 0;
    }

    spin_lock_irqsave(lock, flags);
    if atomic_dec_and_test(atomic) {
        return 1;
    }
    spin_unlock_irqrestore(lock, *flags);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atomic_dec_and_raw_lock(
    atomic: *mut AtomicT,
    lock: *mut RawSpinlockT,
) -> i32 {
    let atomic = unsafe { &*atomic };
    let lock = unsafe { &*lock };

    if atomic_add_unless(atomic, -1, 1) {
        return 0;
    }

    raw_spin_lock(lock);
    if atomic_dec_and_test(atomic) {
        return 1;
    }
    raw_spin_unlock(lock);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _atomic_dec_and_raw_lock_irqsave(
    atomic: *mut AtomicT,
    lock: *mut RawSpinlockT,
    flags: *mut IrqFlags,
) -> i32 {
    let atomic = unsafe { &*atomic };
    let lock = unsafe { &*lock };
    let flags = unsafe { &mut *flags };

    if atomic_add_unless(atomic, -1, 1) {
        return 0;
    }

    raw_spin_lock_irqsave(lock, flags);
    if atomic_dec_and_test(atomic) {
        return 1;
    }
    raw_spin_unlock_irqrestore(lock, *flags);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_and_lock_matches_linux_slow_path_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/dec_and_lock.c"
        ));
        assert!(source.contains("if (atomic_add_unless(atomic, -1, 1))"));
        assert!(source.contains("spin_lock(lock);"));
        assert!(source.contains("spin_lock_irqsave(lock, *flags);"));
        assert!(source.contains("raw_spin_lock(lock);"));
        assert!(source.contains("raw_spin_lock_irqsave(lock, *flags);"));
        assert!(source.contains("if (atomic_dec_and_test(atomic))"));
        assert!(source.contains("spin_unlock(lock);"));
        assert!(source.contains("raw_spin_unlock(lock);"));
        assert!(source.contains("EXPORT_SYMBOL(atomic_dec_and_lock);"));
        assert!(source.contains("EXPORT_SYMBOL(_atomic_dec_and_raw_lock_irqsave);"));
    }

    #[test]
    fn fast_path_decrements_without_locking() {
        let mut atomic = AtomicT::new(3);
        let mut lock = SpinlockT::new();

        let ret = unsafe { atomic_dec_and_lock(&mut atomic, &mut lock) };

        assert_eq!(ret, 0);
        assert_eq!(atomic.read(), 2);
        assert!(!lock.is_locked());
    }

    #[test]
    fn slow_path_returns_with_spinlock_held_at_zero() {
        let mut atomic = AtomicT::new(1);
        let mut lock = SpinlockT::new();

        let ret = unsafe { atomic_dec_and_lock(&mut atomic, &mut lock) };

        assert_eq!(ret, 1);
        assert_eq!(atomic.read(), 0);
        assert!(lock.is_locked());
        lock.unlock();
        assert!(!lock.is_locked());
    }

    #[test]
    fn irqsave_and_raw_variants_follow_the_same_paths() {
        let mut flags = IrqFlags::MAX;
        let mut atomic = AtomicT::new(1);
        let mut lock = SpinlockT::new();

        let ret = unsafe { _atomic_dec_and_lock_irqsave(&mut atomic, &mut lock, &mut flags) };

        assert_eq!(ret, 1);
        assert_eq!(atomic.read(), 0);
        assert!(lock.is_locked());
        lock.unlock_irqrestore(flags);

        let mut raw_flags = IrqFlags::MAX;
        let mut raw_atomic = AtomicT::new(1);
        let mut raw_lock = RawSpinlockT::new();

        let ret = unsafe {
            _atomic_dec_and_raw_lock_irqsave(&mut raw_atomic, &mut raw_lock, &mut raw_flags)
        };

        assert_eq!(ret, 1);
        assert_eq!(raw_atomic.read(), 0);
        assert!(raw_lock.is_locked());
        raw_lock.unlock_irqrestore(raw_flags);
    }

    #[test]
    fn dec_and_lock_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("atomic_dec_and_lock"),
            Some(atomic_dec_and_lock as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("_atomic_dec_and_lock_irqsave"),
            Some(_atomic_dec_and_lock_irqsave as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("atomic_dec_and_raw_lock"),
            Some(atomic_dec_and_raw_lock as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("_atomic_dec_and_raw_lock_irqsave"),
            Some(_atomic_dec_and_raw_lock_irqsave as usize)
        );
    }
}
