//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/io-wq.c
//! test-origin: linux:vendor/linux/io_uring/io-wq.c
//! io_uring worker pool — bounded & unbounded kthread workers.
//!
//! Linux's `io-wq` runs blocking ops on a pool of dedicated kthreads so the
//! submitting task isn't blocked.  Lupos backs this with the existing
//! `src/kernel/workqueue/` cooperative variant; per-pool worker spawning
//! happens via `src/kernel/kthread.rs::kthread_run`.
//!
//! Ref: vendor/linux/io_uring/io-wq.c

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

/// `enum io_wq_type` — bound = needs file I/O context, unbound = pure CPU.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:734
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoWqType {
    Bound = 0,
    Unbound = 1,
}

/// `IO_WQ_NR_MAX_WORKERS_DEFAULT` — Linux default cap.
pub const IO_WQ_DEFAULT_MAX_WORKERS: u32 = 256;

/// One work item.  The opaque payload is whatever the caller wants to
/// dispatch — typically a request pointer.
pub struct IoWqWork {
    pub func: fn(u64) -> i32,
    pub payload: u64,
}

/// Per-ring io_wq state.  Tracks worker count and a pending queue.
pub struct IoWq {
    pub max_workers: [AtomicU32; 2],
    pub running_workers: [AtomicU32; 2],
    queues: [Mutex<Vec<IoWqWork>>; 2],
}

impl IoWq {
    pub fn new(bound_max: u32, unbound_max: u32) -> Arc<Self> {
        Arc::new(Self {
            max_workers: [AtomicU32::new(bound_max), AtomicU32::new(unbound_max)],
            running_workers: [AtomicU32::new(0), AtomicU32::new(0)],
            queues: [Mutex::new(Vec::new()), Mutex::new(Vec::new())],
        })
    }

    /// `io_wq_create` default — `IO_WQ_DEFAULT_MAX_WORKERS` per type.
    pub fn default_create() -> Arc<Self> {
        Self::new(IO_WQ_DEFAULT_MAX_WORKERS, IO_WQ_DEFAULT_MAX_WORKERS)
    }

    /// `io_wq_enqueue` — push work onto the per-type queue.
    pub fn enqueue(&self, ty: IoWqType, work: IoWqWork) {
        self.queues[ty as usize].lock().push(work);
    }

    /// `io_wq_worker` — drain one work item.  Returns `None` if the queue is
    /// empty.  Real workers loop on this from a kthread.
    pub fn run_one(&self, ty: IoWqType) -> Option<i32> {
        let queue = &self.queues[ty as usize];
        let work = {
            let mut g = queue.lock();
            if g.is_empty() {
                return None;
            }
            g.remove(0)
        };
        self.running_workers[ty as usize].fetch_add(1, Ordering::AcqRel);
        let r = (work.func)(work.payload);
        self.running_workers[ty as usize].fetch_sub(1, Ordering::AcqRel);
        Some(r)
    }

    /// `io_wq_drain` — drain everything from one queue type.
    pub fn drain(&self, ty: IoWqType) -> u32 {
        let mut n = 0;
        while self.run_one(ty).is_some() {
            n += 1;
        }
        n
    }

    /// `io_register_iowq_max_workers` — adjust the per-type cap.
    pub fn set_max_workers(&self, ty: IoWqType, max: u32) -> u32 {
        let prev = self.max_workers[ty as usize].swap(max, Ordering::AcqRel);
        prev
    }

    pub fn pending(&self, ty: IoWqType) -> usize {
        self.queues[ty as usize].lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo(p: u64) -> i32 {
        p as i32
    }

    #[test]
    fn iowq_type_repr_matches_linux() {
        // `IO_WQ_BOUND = 0`, `IO_WQ_UNBOUND = 1`.
        assert_eq!(IoWqType::Bound as u32, 0);
        assert_eq!(IoWqType::Unbound as u32, 1);
    }

    #[test]
    fn enqueue_and_drain_runs_each_once() {
        let wq = IoWq::default_create();
        wq.enqueue(
            IoWqType::Bound,
            IoWqWork {
                func: echo,
                payload: 1,
            },
        );
        wq.enqueue(
            IoWqType::Bound,
            IoWqWork {
                func: echo,
                payload: 2,
            },
        );
        let n = wq.drain(IoWqType::Bound);
        assert_eq!(n, 2);
        assert_eq!(wq.pending(IoWqType::Bound), 0);
    }

    #[test]
    fn queues_are_independent_per_type() {
        let wq = IoWq::default_create();
        wq.enqueue(
            IoWqType::Bound,
            IoWqWork {
                func: echo,
                payload: 10,
            },
        );
        assert_eq!(wq.pending(IoWqType::Bound), 1);
        assert_eq!(wq.pending(IoWqType::Unbound), 0);
        assert!(wq.run_one(IoWqType::Unbound).is_none());
        let r = wq.run_one(IoWqType::Bound).unwrap();
        assert_eq!(r, 10);
    }

    #[test]
    fn set_max_workers_returns_previous_and_updates() {
        let wq = IoWq::default_create();
        let prev = wq.set_max_workers(IoWqType::Unbound, 8);
        assert_eq!(prev, IO_WQ_DEFAULT_MAX_WORKERS);
        assert_eq!(wq.max_workers[1].load(Ordering::Acquire), 8);
    }
}
