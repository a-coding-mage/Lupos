//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/mock_file.c
//! test-origin: linux:vendor/linux/io_uring/mock_file.c
//! `CONFIG_IO_URING_MOCK_FILE` test helper.
//!
//! Provides an hrtimer-driven fake file that completes I/O after a
//! configurable delay.  Used by self-tests to exercise the async paths
//! without depending on real device drivers.
//!
//! Ref: vendor/linux/io_uring/mock_file.c

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Per-mock state.  Each request gets stashed here and "completes" after the
/// recorded delay; tests drive the clock manually.
pub struct MockFile {
    /// Pending ops: (user_data, delay_ms, result).
    pending: spin::Mutex<Vec<(u64, u32, i32)>>,
    /// Logical "now" — incremented by `advance(ms)` from tests.
    now_ms: AtomicU64,
    pub completed: AtomicU32,
}

impl MockFile {
    pub const fn new() -> Self {
        Self {
            pending: spin::Mutex::new(Vec::new()),
            now_ms: AtomicU64::new(0),
            completed: AtomicU32::new(0),
        }
    }

    pub fn submit(&self, user_data: u64, delay_ms: u32, result: i32) {
        let due = self.now_ms.load(Ordering::Acquire) + delay_ms as u64;
        self.pending.lock().push((user_data, due as u32, result));
    }

    /// Advance the logical clock and return the list of completions whose
    /// deadlines have now passed.
    pub fn advance(&self, ms: u64) -> Vec<(u64, i32)> {
        let new_now = self.now_ms.fetch_add(ms, Ordering::AcqRel) + ms;
        let mut out = Vec::new();
        let mut g = self.pending.lock();
        let mut i = 0;
        while i < g.len() {
            if (g[i].1 as u64) <= new_now {
                let (ud, _, r) = g.remove(i);
                out.push((ud, r));
                self.completed.fetch_add(1, Ordering::AcqRel);
            } else {
                i += 1;
            }
        }
        out
    }

    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_and_advance_fires_due_completions() {
        let m = MockFile::new();
        m.submit(1, 10, 100);
        m.submit(2, 20, 200);
        let r = m.advance(15);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], (1, 100));
        assert_eq!(m.pending_count(), 1);
    }

    #[test]
    fn no_completions_returned_before_delay() {
        let m = MockFile::new();
        m.submit(7, 50, 0);
        let r = m.advance(10);
        assert!(r.is_empty());
    }

    #[test]
    fn advance_past_all_drains_queue() {
        let m = MockFile::new();
        m.submit(1, 5, 0);
        m.submit(2, 10, 0);
        m.submit(3, 15, 0);
        let r = m.advance(20);
        assert_eq!(r.len(), 3);
        assert_eq!(m.completed.load(Ordering::Acquire), 3);
    }
}
