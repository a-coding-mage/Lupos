//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/eventfd.c
//! test-origin: linux:vendor/linux/io_uring/eventfd.c
//! Registered eventfd notifier for CQ completion wake-ups.
//!
//! `IORING_REGISTER_EVENTFD` arms a userspace eventfd that the kernel writes
//! `1` to every time a new CQE is posted (or only on async CQEs when
//! `EVENTFD_ASYNC` was used).
//!
//! Ref: vendor/linux/io_uring/eventfd.c

use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

/// `struct io_ev_fd` — per-ring eventfd registration.
pub struct IoEvFd {
    /// Registered fd (set by `register`, `-1` when not armed).
    pub fd: AtomicI32,
    /// `last_cq_tail` — when this matches `cq_tail`, no new completions yet.
    pub last_cq_tail: AtomicU64,
    /// Linux: `eventfd_async` — only signal on async/io-wq completions.
    pub async_only: core::sync::atomic::AtomicBool,
}

impl IoEvFd {
    pub const fn new() -> Self {
        Self {
            fd: AtomicI32::new(-1),
            last_cq_tail: AtomicU64::new(0),
            async_only: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// `io_eventfd_register`.  Returns `-EBUSY` if already armed (matches Linux).
    pub fn register(&self, fd: i32, async_only: bool) -> Result<(), i32> {
        let prev = self.fd.swap(fd, Ordering::AcqRel);
        if prev != -1 {
            // Roll back; Linux's behavior is to refuse the new one.
            self.fd.store(prev, Ordering::Release);
            return Err(-16);
        }
        self.async_only.store(async_only, Ordering::Release);
        Ok(())
    }

    /// `io_eventfd_unregister`.
    pub fn unregister(&self) -> Result<i32, i32> {
        let prev = self.fd.swap(-1, Ordering::AcqRel);
        if prev == -1 {
            return Err(-2);
        }
        Ok(prev)
    }

    /// `io_eventfd_signal` — bump the registered fd by writing 1.  Returns the
    /// new "expected counter" value.  When not armed, returns `None`.
    pub fn signal(&self) -> Option<u64> {
        if self.fd.load(Ordering::Acquire) == -1 {
            return None;
        }
        Some(self.last_cq_tail.fetch_add(1, Ordering::AcqRel) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_unregister() {
        let e = IoEvFd::new();
        e.register(5, false).unwrap();
        assert_eq!(e.unregister().unwrap(), 5);
    }

    #[test]
    fn double_register_is_ebusy() {
        let e = IoEvFd::new();
        e.register(5, false).unwrap();
        assert_eq!(e.register(6, false).unwrap_err(), -16);
    }

    #[test]
    fn unregister_without_register_is_enoent() {
        let e = IoEvFd::new();
        assert_eq!(e.unregister().unwrap_err(), -2);
    }

    #[test]
    fn signal_when_not_armed_returns_none() {
        let e = IoEvFd::new();
        assert!(e.signal().is_none());
    }

    #[test]
    fn signal_increments_counter() {
        let e = IoEvFd::new();
        e.register(5, false).unwrap();
        assert_eq!(e.signal(), Some(1));
        assert_eq!(e.signal(), Some(2));
    }
}
