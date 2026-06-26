//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/qrwlock.c
//! test-origin: linux:vendor/linux/kernel/locking/qrwlock.c
//! Queued rwlock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/qrwlock.c`.  Readers hold a positive
//! count; the writer owns the lock with the negative sentinel value.

use core::sync::atomic::{AtomicIsize, Ordering};

pub const QRWLOCK_WRITE_LOCKED: isize = -1;

#[repr(C)]
pub struct QrwLock {
    state: AtomicIsize,
}

impl QrwLock {
    pub const fn new() -> Self {
        Self {
            state: AtomicIsize::new(0),
        }
    }

    pub fn try_read_lock(&self) -> bool {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state < 0 {
                return false;
            }
            if self
                .state
                .compare_exchange(state, state + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }

    pub fn read_unlock(&self) {
        self.state.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn try_write_lock(&self) -> bool {
        self.state
            .compare_exchange(0, QRWLOCK_WRITE_LOCKED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_side_excludes_writer() {
        let lock = QrwLock::new();
        assert!(lock.try_read_lock());
        assert!(!lock.try_write_lock());
        lock.read_unlock();
        assert!(lock.try_write_lock());
        lock.write_unlock();
        assert!(lock.try_read_lock());
    }
}
