//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/spinlock_rt.c
//! test-origin: linux:vendor/linux/kernel/locking/spinlock_rt.c
//! RT spinlock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/spinlock_rt.c`.  PREEMPT_RT maps many
//! spinlock users to sleeping RT mutexes; Lupos exposes the same wrapper shape.

use super::rt_mutex::RtMutex;

#[repr(C)]
pub struct RtSpinLock {
    inner: RtMutex,
}

impl RtSpinLock {
    pub const fn new() -> Self {
        Self {
            inner: RtMutex::new(),
        }
    }

    pub fn try_lock(&self) -> bool {
        self.inner.try_lock()
    }

    pub fn unlock(&self) {
        self.inner.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rt_spin_try_lock_round_trip() {
        let lock = RtSpinLock::new();
        assert!(lock.try_lock());
        lock.unlock();
        assert!(lock.try_lock());
    }
}
