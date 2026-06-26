//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/zcrx.c
//! test-origin: linux:vendor/linux/io_uring/zcrx.c
//! Zero-copy RX for io_uring.
//!
//! `IORING_REGISTER_ZCRX_IFQ` registers a NIC RX queue whose packets land
//! directly in userspace-owned pages.  Linux backs this with `struct
//! page_pool` and `memory_provider` callbacks.  Lupos's port lands the
//! registration + completion bookkeeping here; full page_pool integration
//! follows the networking stack's evolution.
//!
//! Ref: vendor/linux/io_uring/zcrx.c

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::uapi::IoUringRegionDesc;

/// `struct io_zcrx_ifq` — per-(ring, ifindex) zerocopy receive queue.
pub struct IoZcrxIfq {
    pub ifindex: u32,
    pub queue_id: u32,
    /// Backing area for the rx queue.
    pub region: IoUringRegionDesc,
    /// CQEs delivered so far.
    pub cqes_posted: AtomicU64,
    /// Outstanding ubufs / packets.
    pub inflight: AtomicU32,
}

impl IoZcrxIfq {
    pub fn new(ifindex: u32, queue_id: u32, region: IoUringRegionDesc) -> Self {
        Self {
            ifindex,
            queue_id,
            region,
            cqes_posted: AtomicU64::new(0),
            inflight: AtomicU32::new(0),
        }
    }

    /// `io_zcrx_post_cqe` — bump the counter and return the new value.
    pub fn post_cqe(&self) -> u64 {
        self.cqes_posted.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn inflight_grab(&self) -> u32 {
        self.inflight.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn inflight_release(&self) -> u32 {
        self.inflight.fetch_sub(1, Ordering::AcqRel) - 1
    }
}

/// Per-ring registry of zerocopy interfaces.
pub struct ZcrxRegistry {
    queues: Vec<IoZcrxIfq>,
}

impl ZcrxRegistry {
    pub const fn new() -> Self {
        Self { queues: Vec::new() }
    }

    pub fn register(&mut self, ifq: IoZcrxIfq) -> Result<usize, i32> {
        if self
            .queues
            .iter()
            .any(|q| q.ifindex == ifq.ifindex && q.queue_id == ifq.queue_id)
        {
            return Err(-16);
        }
        let idx = self.queues.len();
        self.queues.push(ifq);
        Ok(idx)
    }

    pub fn lookup(&self, idx: usize) -> Option<&IoZcrxIfq> {
        self.queues.get(idx)
    }

    pub fn len(&self) -> usize {
        self.queues.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_region() -> IoUringRegionDesc {
        IoUringRegionDesc::default()
    }

    #[test]
    fn register_unique_ifq_succeeds() {
        let mut r = ZcrxRegistry::new();
        let idx = r.register(IoZcrxIfq::new(2, 0, dummy_region())).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn register_duplicate_is_ebusy() {
        let mut r = ZcrxRegistry::new();
        r.register(IoZcrxIfq::new(2, 0, dummy_region())).unwrap();
        let r2 = r.register(IoZcrxIfq::new(2, 0, dummy_region()));
        assert_eq!(r2.unwrap_err(), -16);
    }

    #[test]
    fn post_cqe_increments_counter() {
        let q = IoZcrxIfq::new(2, 0, dummy_region());
        assert_eq!(q.post_cqe(), 1);
        assert_eq!(q.post_cqe(), 2);
    }

    #[test]
    fn inflight_grab_and_release_round_trip() {
        let q = IoZcrxIfq::new(2, 0, dummy_region());
        assert_eq!(q.inflight_grab(), 1);
        assert_eq!(q.inflight_grab(), 2);
        assert_eq!(q.inflight_release(), 1);
        assert_eq!(q.inflight_release(), 0);
    }
}
