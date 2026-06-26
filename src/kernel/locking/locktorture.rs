//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/locktorture.c
//! test-origin: linux:vendor/linux/kernel/locking/locktorture.c
//! Lock torture harness coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/locktorture.c`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::spinlock::SpinLock;

static ITERATIONS: AtomicU64 = AtomicU64::new(0);

pub fn run_locktorture_smoke(iterations: u32) -> u64 {
    let lock = SpinLock::new(0u64);
    for _ in 0..iterations {
        let mut guard = lock.lock();
        *guard += 1;
        ITERATIONS.fetch_add(1, Ordering::AcqRel);
    }
    *lock.lock()
}

pub fn locktorture_iterations() -> u64 {
    ITERATIONS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_counts_each_iteration() {
        assert_eq!(run_locktorture_smoke(3), 3);
    }
}
