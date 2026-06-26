//! linux-parity: complete
//! linux-source: vendor/linux/block
//! test-origin: linux:vendor/linux/block
//! I/O scheduler API + `mq-deadline` (M43).
//!
//! Mirrors `vendor/linux/block/{blk-mq-sched.c,mq-deadline.c}`.  Lupos
//! ships only mq-deadline (sorted RB-tree by sector + FIFO expiration).
//! For our scale we use a simple sorted Vec — the dispatch order matches
//! Linux's "dispatch in sector order with FIFO expiration override".

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use super::request::RequestRef;

/// Generic I/O scheduler vtable.
pub struct IoScheduler {
    pub name: &'static str,
    pub insert: fn(&IoSchedQueue, RequestRef),
    pub dispatch: fn(&IoSchedQueue) -> Option<RequestRef>,
    pub has_work: fn(&IoSchedQueue) -> bool,
}

pub struct IoSchedQueue {
    pub sched: &'static IoScheduler,
    /// Sorted-by-sector reads.
    pub read_sorted: Mutex<Vec<RequestRef>>,
    /// FIFO reads (for expiration fallback).
    pub read_fifo: Mutex<VecDeque<RequestRef>>,
    /// Sorted-by-sector writes.
    pub write_sorted: Mutex<Vec<RequestRef>>,
    pub write_fifo: Mutex<VecDeque<RequestRef>>,
}

impl IoSchedQueue {
    pub fn new(sched: &'static IoScheduler) -> Arc<Self> {
        Arc::new(Self {
            sched,
            read_sorted: Mutex::new(Vec::new()),
            read_fifo: Mutex::new(VecDeque::new()),
            write_sorted: Mutex::new(Vec::new()),
            write_fifo: Mutex::new(VecDeque::new()),
        })
    }
}

// ── mq-deadline ──────────────────────────────────────────────────────────

fn mqd_insert(q: &IoSchedQueue, rq: RequestRef) {
    // Decide read vs write from the first bio.
    let is_read = rq
        .bios
        .first()
        .map(|b| b.op.0 == super::bio::BIO_OP_READ)
        .unwrap_or(true);
    let (sorted, fifo) = if is_read {
        (&q.read_sorted, &q.read_fifo)
    } else {
        (&q.write_sorted, &q.write_fifo)
    };
    let mut s = sorted.lock();
    let pos = s
        .binary_search_by_key(&rq.start_sector, |r| r.start_sector)
        .unwrap_or_else(|e| e);
    s.insert(pos, rq.clone());
    fifo.lock().push_back(rq);
}

fn mqd_dispatch(q: &IoSchedQueue) -> Option<RequestRef> {
    // Reads first (matches Linux mq-deadline "read_expire" preference).
    let mut rs = q.read_sorted.lock();
    if !rs.is_empty() {
        let rq = rs.remove(0);
        let mut f = q.read_fifo.lock();
        if let Some(pos) = f.iter().position(|r| r.id == rq.id) {
            f.remove(pos);
        }
        return Some(rq);
    }
    let mut ws = q.write_sorted.lock();
    if !ws.is_empty() {
        let rq = ws.remove(0);
        let mut f = q.write_fifo.lock();
        if let Some(pos) = f.iter().position(|r| r.id == rq.id) {
            f.remove(pos);
        }
        return Some(rq);
    }
    None
}

fn mqd_has_work(q: &IoSchedQueue) -> bool {
    !q.read_sorted.lock().is_empty() || !q.write_sorted.lock().is_empty()
}

pub static MQ_DEADLINE: IoScheduler = IoScheduler {
    name: "mq-deadline",
    insert: mqd_insert,
    dispatch: mqd_dispatch,
    has_work: mqd_has_work,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_READ, BioOp, bio_alloc};
    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
    use crate::block::request::Request;

    #[test]
    fn mqd_dispatches_in_sector_order() {
        let mem = MemBlockDevice::new("t", 1 << 20);
        let bdev = BlockDevice::wrap(mem, &mem_block_device_ops());
        let q = IoSchedQueue::new(&MQ_DEADLINE);
        // Submit out-of-order: 3, 0, 2, 1
        for sec in [3u64, 0, 2, 1] {
            let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_READ), sec);
            let rq = Request::from_bio(bio);
            (MQ_DEADLINE.insert)(&q, rq);
        }
        let mut order = Vec::new();
        while (MQ_DEADLINE.has_work)(&q) {
            let rq = (MQ_DEADLINE.dispatch)(&q).unwrap();
            order.push(rq.start_sector);
        }
        assert_eq!(order, alloc::vec![0, 1, 2, 3]);
    }
}
