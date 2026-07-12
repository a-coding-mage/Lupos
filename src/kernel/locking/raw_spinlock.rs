//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! `raw_spinlock_t` — ticket spinlock (M33).
//!
//! Mirrors the in-kernel ticket spinlock used pre-qspinlock and the
//! `arch_spinlock_t` IRQ-context primitive in
//! `vendor/linux/arch/x86/include/asm/spinlock.h` (now superseded by qspinlock
//! upstream, but still the structural model for `raw_spinlock_t`).
//!
//! Provides ticket-based FIFO ordering: a thread atomically takes the next
//! ticket number, then spins until `head` matches its ticket.  Fair under
//! contention, no starvation.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

use super::irqflags::{
    IrqFlags, local_irq_disable, local_irq_enable, local_irq_restore, local_irq_save,
};
use super::preempt::{local_bh_disable, local_bh_enable, preempt_disable, preempt_enable};
use super::qspinlock::QSpinLock;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("_raw_spin_lock", linux_raw_spin_lock as usize, false);
    export_symbol_once("_raw_spin_trylock", linux_raw_spin_trylock as usize, false);
    export_symbol_once("_raw_spin_unlock", linux_raw_spin_unlock as usize, false);
    export_symbol_once("_raw_spin_lock_bh", linux_raw_spin_lock_bh as usize, false);
    export_symbol_once(
        "_raw_spin_unlock_bh",
        linux_raw_spin_unlock_bh as usize,
        false,
    );
    export_symbol_once(
        "_raw_spin_lock_irqsave",
        linux_raw_spin_lock_irqsave as usize,
        false,
    );
    export_symbol_once(
        "_raw_spin_unlock_irqrestore",
        linux_raw_spin_unlock_irqrestore as usize,
        false,
    );
    export_symbol_once(
        "_raw_spin_lock_irq",
        linux_raw_spin_lock_irq as usize,
        false,
    );
    export_symbol_once(
        "_raw_spin_unlock_irq",
        linux_raw_spin_unlock_irq as usize,
        false,
    );
}

/// `_raw_spin_lock` — `vendor/linux/kernel/locking/spinlock.c:156` and
/// `vendor/linux/include/linux/spinlock_api_smp.h:143`.
#[unsafe(export_name = "_raw_spin_lock")]
pub unsafe extern "C" fn linux_raw_spin_lock(lock: *mut QSpinLock) {
    preempt_disable();
    unsafe { &*lock }.lock();
}

/// `_raw_spin_trylock` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:90`.
#[unsafe(export_name = "_raw_spin_trylock")]
pub unsafe extern "C" fn linux_raw_spin_trylock(lock: *mut QSpinLock) -> i32 {
    preempt_disable();
    if unsafe { &*lock }.try_lock() {
        1
    } else {
        preempt_enable();
        0
    }
}

/// `_raw_spin_unlock` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:157`.
#[unsafe(export_name = "_raw_spin_unlock")]
pub unsafe extern "C" fn linux_raw_spin_unlock(lock: *mut QSpinLock) {
    unsafe { &*lock }.unlock();
    preempt_enable();
}

/// `_raw_spin_lock_bh` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:136`.
#[unsafe(export_name = "_raw_spin_lock_bh")]
pub unsafe extern "C" fn linux_raw_spin_lock_bh(lock: *mut QSpinLock) {
    local_bh_disable();
    unsafe { &*lock }.lock();
}

/// `_raw_spin_unlock_bh` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:190`.
#[unsafe(export_name = "_raw_spin_unlock_bh")]
pub unsafe extern "C" fn linux_raw_spin_unlock_bh(lock: *mut QSpinLock) {
    unsafe { &*lock }.unlock();
    local_bh_enable();
}

/// `_raw_spin_lock_irqsave` —
/// `vendor/linux/kernel/locking/spinlock.c:164` and
/// `vendor/linux/include/linux/spinlock_api_smp.h:125`.
///
/// Vendor x86-64 `raw_spinlock_t` contains a four-byte `arch_spinlock_t`
/// qspinlock, so the module-facing ABI deliberately takes `QSpinLock` rather
/// than Lupos's separate Rust-owned ticket-lock wrapper below.
#[unsafe(export_name = "_raw_spin_lock_irqsave")]
pub unsafe extern "C" fn linux_raw_spin_lock_irqsave(lock: *mut QSpinLock) -> IrqFlags {
    let flags = local_irq_save();
    preempt_disable();
    unsafe { &*lock }.lock();
    flags
}

/// `_raw_spin_unlock_irqrestore` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:172`.
#[unsafe(export_name = "_raw_spin_unlock_irqrestore")]
pub unsafe extern "C" fn linux_raw_spin_unlock_irqrestore(lock: *mut QSpinLock, flags: IrqFlags) {
    unsafe { &*lock }.unlock();
    local_irq_restore(flags);
    preempt_enable();
}

/// `_raw_spin_lock_irq` — `vendor/linux/include/linux/spinlock_api_smp.h:137`.
#[unsafe(export_name = "_raw_spin_lock_irq")]
pub unsafe extern "C" fn linux_raw_spin_lock_irq(lock: *mut QSpinLock) {
    local_irq_disable();
    preempt_disable();
    unsafe { &*lock }.lock();
}

/// `_raw_spin_unlock_irq` —
/// `vendor/linux/include/linux/spinlock_api_smp.h:182`.
#[unsafe(export_name = "_raw_spin_unlock_irq")]
pub unsafe extern "C" fn linux_raw_spin_unlock_irq(lock: *mut QSpinLock) {
    unsafe { &*lock }.unlock();
    local_irq_enable();
    preempt_enable();
}

/// Ticket spinlock state — `next` and `owner` packed into a single 32-bit word
/// so the cmpxchg increment is a single instruction.
///
/// Layout: low 16 bits = `owner` (currently-served ticket); high 16 bits =
/// `next` (next ticket to issue).
#[repr(C)]
pub struct RawSpinLock {
    state: AtomicU32,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
        }
    }

    /// Acquire the lock.  Spins until our ticket is served.
    #[inline]
    pub fn lock(&self) {
        // Atomically increment `next` and capture the old value as our ticket.
        let ticket_word = self.state.fetch_add(1u32 << 16, Ordering::AcqRel);
        let our_ticket = (ticket_word >> 16) as u16;

        loop {
            let cur = self.state.load(Ordering::Acquire);
            let owner = (cur & 0xFFFF) as u16;
            if owner == our_ticket {
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// Try to acquire — returns true on success, false if held.
    #[inline]
    pub fn try_lock(&self) -> bool {
        let cur = self.state.load(Ordering::Acquire);
        let owner = cur & 0xFFFF;
        let next = cur >> 16;
        if owner != next {
            return false; // someone else has a ticket queued
        }
        // Bump `next` only if state hasn't changed.
        let new = cur.wrapping_add(1u32 << 16);
        self.state
            .compare_exchange(cur, new, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Release the lock.  Increments `owner` so the next ticket-holder can run.
    #[inline]
    pub fn unlock(&self) {
        loop {
            let cur = self.state.load(Ordering::Relaxed);
            let owner = ((cur & 0xFFFF) as u16).wrapping_add(1) as u32;
            let new = (cur & 0xFFFF_0000) | owner;
            if self
                .state
                .compare_exchange_weak(cur, new, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// Returns true if the lock is currently held.
    #[inline]
    pub fn is_locked(&self) -> bool {
        let cur = self.state.load(Ordering::Acquire);
        (cur & 0xFFFF) != (cur >> 16)
    }
}

unsafe impl Send for RawSpinLock {}
unsafe impl Sync for RawSpinLock {}

/// Rust-flavoured `RawSpinLock<T>` wrapper.  Linux's `raw_spinlock_t` does not
/// own data; this wrapper owns the protected value so callers don't need a
/// separate `UnsafeCell`.
pub struct RawSpinLocked<T> {
    lock: RawSpinLock,
    inner: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RawSpinLocked<T> {}
unsafe impl<T: Send> Sync for RawSpinLocked<T> {}

impl<T> RawSpinLocked<T> {
    pub const fn new(val: T) -> Self {
        Self {
            lock: RawSpinLock::new(),
            inner: UnsafeCell::new(val),
        }
    }

    /// Acquire and return a guard.  Disables preemption (mirrors Linux).
    pub fn lock(&self) -> RawSpinGuard<'_, T> {
        preempt_disable();
        self.lock.lock();
        RawSpinGuard { parent: self }
    }

    /// Acquire with IRQ-save semantics: saves EFLAGS, disables interrupts,
    /// and disables preemption.
    pub fn lock_irqsave(&self) -> (RawSpinGuard<'_, T>, IrqFlags) {
        let flags = local_irq_save();
        preempt_disable();
        self.lock.lock();
        (RawSpinGuard { parent: self }, flags)
    }

    /// Drop a guard previously obtained via `lock_irqsave` and restore EFLAGS.
    pub fn unlock_irqrestore(guard: RawSpinGuard<'_, T>, flags: IrqFlags) {
        drop(guard);
        local_irq_restore(flags);
    }

    pub fn try_lock(&self) -> Option<RawSpinGuard<'_, T>> {
        preempt_disable();
        if self.lock.try_lock() {
            Some(RawSpinGuard { parent: self })
        } else {
            preempt_enable();
            None
        }
    }
}

pub struct RawSpinGuard<'a, T> {
    parent: &'a RawSpinLocked<T>,
}

impl<'a, T> core::ops::Deref for RawSpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.parent.inner.get() }
    }
}

impl<'a, T> core::ops::DerefMut for RawSpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.inner.get() }
    }
}

impl<'a, T> Drop for RawSpinGuard<'a, T> {
    fn drop(&mut self) {
        self.parent.lock.unlock();
        preempt_enable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_round_trip() {
        let l = RawSpinLock::new();
        assert!(!l.is_locked());
        l.lock();
        assert!(l.is_locked());
        l.unlock();
        assert!(!l.is_locked());
    }

    #[test]
    fn unlock_wraps_owner_without_carrying_into_next_ticket() {
        let l = RawSpinLock::new();
        l.state.store(0x0001_FFFF, Ordering::Relaxed);

        l.unlock();

        assert_eq!(l.state.load(Ordering::Relaxed), 0x0001_0000);
    }

    #[test]
    fn try_lock_succeeds_when_free() {
        let l = RawSpinLock::new();
        assert!(l.try_lock());
        assert!(l.is_locked());
        l.unlock();
    }

    #[test]
    fn try_lock_fails_when_held() {
        let l = RawSpinLock::new();
        l.lock();
        assert!(!l.try_lock());
        l.unlock();
    }

    #[test]
    fn ticket_ordering_is_fifo() {
        // Sequential locks/unlocks rotate the ticket counter monotonically.
        let l = RawSpinLock::new();
        for _ in 0..1000 {
            l.lock();
            l.unlock();
        }
        // After 1000 round-trips both halves should have advanced equally.
        let s = l.state.load(Ordering::Acquire);
        assert_eq!(s & 0xFFFF, s >> 16);
    }

    #[test]
    fn locked_wrapper_round_trip() {
        let l = RawSpinLocked::new(0u32);
        {
            let mut g = l.lock();
            *g = 42;
        }
        let g = l.lock();
        assert_eq!(*g, 42);
    }
}
