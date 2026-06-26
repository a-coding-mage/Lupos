//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Wound/wait mutex self-test coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/test-ww_mutex.c`.

use super::ww_rt_mutex::{WwClass, WwRtMutex};

pub fn ww_mutex_selftest() -> bool {
    let class = WwClass::new();
    let a = class.acquire_init();
    let b = class.acquire_init();
    let mutex = WwRtMutex::new();
    mutex.lock(&a).is_ok() && mutex.lock(&b).is_err() && mutex.unlock(&a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selftest_observes_contention() {
        assert!(ww_mutex_selftest());
    }
}
