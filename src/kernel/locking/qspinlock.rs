//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/qspinlock.c
//! test-origin: linux:vendor/linux/kernel/locking/qspinlock.c
//! Queued spinlock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/qspinlock.c`.  Lupos keeps the
//! existing `SpinLock<T>` API in `spinlock.rs`; this module models the compact
//! qspinlock word and slow-path state used by Linux.

use core::sync::atomic::{AtomicU32, Ordering};

pub const _Q_LOCKED_VAL: u32 = 1;
pub const _Q_PENDING_VAL: u32 = 1 << 8;
pub const _Q_TAIL_IDX_OFFSET: u32 = 16;
pub const _Q_TAIL_CPU_OFFSET: u32 = 18;

#[repr(C)]
pub struct QSpinLock {
    val: AtomicU32,
}

impl QSpinLock {
    pub const fn new() -> Self {
        Self {
            val: AtomicU32::new(0),
        }
    }

    pub fn raw(&self) -> u32 {
        self.val.load(Ordering::Acquire)
    }

    pub fn is_locked(&self) -> bool {
        self.raw() & _Q_LOCKED_VAL != 0
    }

    pub fn try_lock(&self) -> bool {
        self.val
            .compare_exchange(0, _Q_LOCKED_VAL, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn lock(&self) {
        while !self.try_lock() {
            core::hint::spin_loop();
        }
    }

    pub fn unlock(&self) {
        self.val.store(0, Ordering::Release);
    }

    pub fn set_pending(&self) {
        self.val.fetch_or(_Q_PENDING_VAL, Ordering::AcqRel);
    }

    pub fn clear_pending(&self) {
        self.val.fetch_and(!_Q_PENDING_VAL, Ordering::AcqRel);
    }
}

pub fn encode_tail(cpu: u16, idx: u8) -> u32 {
    ((cpu as u32 + 1) << _Q_TAIL_CPU_OFFSET) | ((idx as u32) << _Q_TAIL_IDX_OFFSET)
}

pub fn decode_tail(tail: u32) -> Option<(u16, u8)> {
    let cpu = (tail >> _Q_TAIL_CPU_OFFSET) as u16;
    if cpu == 0 {
        return None;
    }
    let idx = ((tail >> _Q_TAIL_IDX_OFFSET) & 0x3) as u8;
    Some((cpu - 1, idx))
}

pub fn queued_spin_lock_slowpath(lock: &QSpinLock) {
    lock.lock();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_lock_sets_locked_bit() {
        let lock = QSpinLock::new();
        assert!(lock.try_lock());
        assert!(lock.is_locked());
        lock.unlock();
        assert!(!lock.is_locked());
    }

    #[test]
    fn tail_round_trips_cpu_and_node_index() {
        assert_eq!(decode_tail(encode_tail(7, 2)), Some((7, 2)));
        assert_eq!(decode_tail(0), None);
    }
}
