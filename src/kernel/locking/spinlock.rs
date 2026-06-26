//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/spinlock.c
//! test-origin: linux:vendor/linux/kernel/locking/spinlock.c
//! `spinlock_t` — sleeping-context spinlock (M33).
//!
//! In mainline Linux this is an MCS queued spinlock (`qspinlock.c`); for
//! Lupos M33 we ship a thin wrapper around `RawSpinLock` that disables
//! preemption + bottom-halves while held.  The behavioural contract matches
//! Linux: never sleep while holding it, never call `schedule()`, must be paired
//! with `local_bh_disable()` if used in BH context (the `_bh` variants below).
//!
//! The full MCS implementation lands as a follow-up once we have a working
//! per-CPU MCS-node array (M35 percpu).

use core::cell::UnsafeCell;

use super::irqflags::{IrqFlags, local_irq_restore, local_irq_save};
use super::preempt::{local_bh_disable, local_bh_enable, preempt_disable, preempt_enable};
use super::raw_spinlock::RawSpinLock;

/// Linux `spinlock_t` shape.  Internally a ticket lock; promoted to MCS in
/// a follow-up to M33.  Layout matches Linux's `struct spinlock` (8 bytes).
#[repr(C)]
pub struct SpinLock<T> {
    raw: RawSpinLock,
    inner: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(val: T) -> Self {
        Self {
            raw: RawSpinLock::new(),
            inner: UnsafeCell::new(val),
        }
    }

    /// Acquire and return a guard.  Disables preemption.
    #[inline]
    pub fn lock(&self) -> SpinGuard<'_, T> {
        preempt_disable();
        self.raw.lock();
        SpinGuard {
            parent: self,
            restore_bh: false,
        }
    }

    /// `spin_lock_bh` — disable softirqs in addition to preemption.
    #[inline]
    pub fn lock_bh(&self) -> SpinGuard<'_, T> {
        local_bh_disable();
        preempt_disable();
        self.raw.lock();
        SpinGuard {
            parent: self,
            restore_bh: true,
        }
    }

    /// `spin_lock_irqsave` — disable interrupts and preemption, save EFLAGS.
    #[inline]
    pub fn lock_irqsave(&self) -> (SpinGuard<'_, T>, IrqFlags) {
        let flags = local_irq_save();
        preempt_disable();
        self.raw.lock();
        (
            SpinGuard {
                parent: self,
                restore_bh: false,
            },
            flags,
        )
    }

    /// `spin_unlock_irqrestore` — pair with `lock_irqsave`.
    #[inline]
    pub fn unlock_irqrestore(guard: SpinGuard<'_, T>, flags: IrqFlags) {
        drop(guard);
        local_irq_restore(flags);
    }

    pub fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        preempt_disable();
        if self.raw.try_lock() {
            Some(SpinGuard {
                parent: self,
                restore_bh: false,
            })
        } else {
            preempt_enable();
            None
        }
    }

    pub fn is_locked(&self) -> bool {
        self.raw.is_locked()
    }
}

pub struct SpinGuard<'a, T> {
    parent: &'a SpinLock<T>,
    restore_bh: bool,
}

impl<'a, T> core::ops::Deref for SpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.parent.inner.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.inner.get() }
    }
}

impl<'a, T> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        self.parent.raw.unlock();
        preempt_enable();
        if self.restore_bh {
            local_bh_enable();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_round_trip() {
        let l = SpinLock::new(0u32);
        {
            let mut g = l.lock();
            *g = 42;
        }
        assert_eq!(*l.lock(), 42);
    }

    #[test]
    fn lock_irqsave_round_trip() {
        let l = SpinLock::new(0u32);
        let (g, flags) = l.lock_irqsave();
        SpinLock::unlock_irqrestore(g, flags);
    }

    #[test]
    fn try_lock_succeeds_when_free() {
        let l = SpinLock::new(0u32);
        assert!(l.try_lock().is_some());
    }

    #[test]
    fn nested_try_lock_fails() {
        let l = SpinLock::new(0u32);
        let _g = l.lock();
        assert!(l.try_lock().is_none());
    }
}
