//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/rtmutex_api.c
//! test-origin: linux:vendor/linux/kernel/locking/rtmutex_api.c
//! RT mutex API surface coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/rtmutex_api.c`.  The heavy-weight
//! owner and waiter mechanics live in `rt_mutex.rs`; this file exposes the
//! small C-style helper layer used by futex PI and later scheduler paths.

use crate::include::uapi::errno::EBUSY;

use super::rt_mutex::RtMutex;

pub fn rt_mutex_init(mutex: &mut RtMutex) {
    *mutex = RtMutex::new();
}

pub fn rt_mutex_trylock(mutex: &RtMutex) -> bool {
    mutex.try_lock()
}

pub fn rt_mutex_lock(mutex: &RtMutex) -> Result<(), i32> {
    if mutex.lock() { Ok(()) } else { Err(EBUSY) }
}

pub fn rt_mutex_unlock(mutex: &RtMutex) {
    mutex.unlock();
}

pub fn rt_mutex_is_locked(mutex: &RtMutex) -> bool {
    mutex.is_locked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_lock_unlock_round_trip() {
        let mutex = RtMutex::new();
        rt_mutex_lock(&mutex).unwrap();
        assert!(rt_mutex_is_locked(&mutex));
        rt_mutex_unlock(&mutex);
        assert!(!rt_mutex_is_locked(&mutex));
    }
}
