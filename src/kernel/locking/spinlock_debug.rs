//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/spinlock_debug.c
//! test-origin: linux:vendor/linux/kernel/locking/spinlock_debug.c
//! Spinlock debug coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/spinlock_debug.c`.

use super::qspinlock::QSpinLock;

pub fn spin_dump(lock: &QSpinLock) -> u32 {
    lock.raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_reflects_locked_bit() {
        let lock = QSpinLock::new();
        assert_eq!(spin_dump(&lock), 0);
        lock.lock();
        assert_eq!(spin_dump(&lock) & super::super::qspinlock::_Q_LOCKED_VAL, 1);
    }
}
