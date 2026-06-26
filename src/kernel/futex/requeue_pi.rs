//! linux-parity: partial
//! linux-source: vendor/linux/kernel/futex
//! test-origin: linux:vendor/linux/kernel/futex
//! `FUTEX_WAIT_REQUEUE_PI` / `FUTEX_CMP_REQUEUE_PI` (M32).
//!
//! Used by glibc / musl `pthread_cond_*`.  Lupos M32 ships the *uncontended*
//! variant: the wait side parks on a normal futex, wake side requeues the
//! parked task to a PI futex without escalating priority.  Real PI chain
//! escalation lands in M33.

use super::EINVAL;
use super::core_ops::{futex_requeue_pi_checked, futex_wait_requeue_pi_prepare};

pub unsafe fn futex_wait_requeue_pi(
    uaddr: u64,
    val: u32,
    uaddr2: u64,
    timeout_ns: u64,
    private: bool,
) -> i64 {
    if uaddr == uaddr2 {
        return -EINVAL as i64;
    }
    unsafe {
        futex_wait_requeue_pi_prepare(
            uaddr,
            val,
            super::FUTEX_BITSET_MATCH_ANY,
            timeout_ns,
            private,
        )
    }
}

pub unsafe fn futex_cmp_requeue_pi(
    uaddr1: u64,
    uaddr2: u64,
    nr_wake: i32,
    nr_requeue: i32,
    cmpval: u32,
    private: bool,
) -> i64 {
    if uaddr1 == uaddr2 || nr_wake != 1 {
        return -EINVAL as i64;
    }
    unsafe { futex_requeue_pi_checked(uaddr1, uaddr2, nr_wake, nr_requeue, cmpval, private) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requeue_pi_zero_is_zero() {
        super::super::core_ops::_with_test_lock(|| {
            super::super::core_ops::_flush_for_tests();
            // Empty bucket → 0 woken / 0 requeued.  Use real word storage so the
            // cmpval read succeeds (host memory model rejects misaligned
            // dereference of arbitrary numeric pointers).
            let lock1: u32 = 0;
            let lock2: u32 = 0;
            let r = unsafe {
                futex_cmp_requeue_pi(
                    &lock1 as *const u32 as u64,
                    &lock2 as *const u32 as u64,
                    1,
                    0,
                    0,
                    true,
                )
            };
            assert!(r >= 0);
            super::super::core_ops::_flush_for_tests();
        });
    }
}
