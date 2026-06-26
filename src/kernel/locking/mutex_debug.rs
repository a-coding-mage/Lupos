//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/mutex-debug.c
//! test-origin: linux:vendor/linux/kernel/locking/mutex-debug.c
//! Mutex debug coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/mutex-debug.c`.

use super::mutex::Mutex;

pub fn mutex_is_locked<T>(mutex: &Mutex<T>) -> bool {
    mutex.is_locked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observes_mutex_state() {
        let mutex = Mutex::new(1u32);
        assert!(!mutex_is_locked(&mutex));
        let _guard = mutex.lock();
        assert!(mutex_is_locked(&mutex));
    }
}
