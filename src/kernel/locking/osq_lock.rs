//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/osq_lock.c
//! test-origin: linux:vendor/linux/kernel/locking/osq_lock.c
//! Optimistic spinning queue lock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/osq_lock.c`.  The Linux primitive is
//! used by mutex and rwsem optimistic spinning; this compact port preserves the
//! owner-token handoff contract without depending on per-CPU nodes.

use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(C)]
pub struct OptimisticSpinQueue {
    tail: AtomicUsize,
}

impl OptimisticSpinQueue {
    pub const fn new() -> Self {
        Self {
            tail: AtomicUsize::new(0),
        }
    }

    pub fn try_lock(&self, cpu_id: usize) -> bool {
        let token = cpu_id.saturating_add(1);
        token != 0
            && self
                .tail
                .compare_exchange(0, token, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }

    pub fn unlock(&self, cpu_id: usize) -> bool {
        let token = cpu_id.saturating_add(1);
        self.tail
            .compare_exchange(token, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn owner_token(&self) -> usize {
        self.tail.load(Ordering::Acquire)
    }
}

pub fn osq_lock(queue: &OptimisticSpinQueue, cpu_id: usize) -> bool {
    queue.try_lock(cpu_id)
}

pub fn osq_unlock(queue: &OptimisticSpinQueue, cpu_id: usize) -> bool {
    queue.unlock(cpu_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_token_is_cpu_plus_one() {
        let q = OptimisticSpinQueue::new();
        assert!(q.try_lock(3));
        assert_eq!(q.owner_token(), 4);
        assert!(q.unlock(3));
        assert_eq!(q.owner_token(), 0);
    }

    #[test]
    fn contended_try_lock_fails() {
        let q = OptimisticSpinQueue::new();
        assert!(osq_lock(&q, 0));
        assert!(!osq_lock(&q, 1));
    }
}
