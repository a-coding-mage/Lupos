//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/rwbase_rt.c
//! test-origin: linux:vendor/linux/kernel/locking/rwbase_rt.c
//! RT read/write base lock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/rwbase_rt.c`.  Linux uses this as the
//! common real-time backend for rwsems and rwlocks.  The Lupos port keeps the
//! same writer-exclusion contract with a bounded atomic state word.

use core::sync::atomic::{AtomicI32, Ordering};

pub const RWBASE_RT_WRITE_LOCKED: i32 = -1;

#[repr(C)]
pub struct RtRwBase {
    state: AtomicI32,
}

impl RtRwBase {
    pub const fn new() -> Self {
        Self {
            state: AtomicI32::new(0),
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
            .compare_exchange(
                0,
                RWBASE_RT_WRITE_LOCKED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
    }

    pub fn state(&self) -> i32 {
        self.state.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_block_writer_until_released() {
        let lock = RtRwBase::new();
        assert!(lock.try_read_lock());
        assert!(!lock.try_write_lock());
        lock.read_unlock();
        assert!(lock.try_write_lock());
        lock.write_unlock();
        assert_eq!(lock.state(), 0);
    }
}
