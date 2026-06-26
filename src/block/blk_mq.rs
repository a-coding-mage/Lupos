//! linux-parity: partial
//! linux-source: vendor/linux/block/blk-mq.c
//! `blk-mq` software + hardware queue.
//!
//! Mirrors `vendor/linux/block/blk-mq.c`.  Lupos M43 simplifies to one
//! software queue + one hardware queue per device — per-CPU steering lands
//! once SMP migration finishes (M55+).

extern crate alloc;

use alloc::sync::Arc;
use spin::Mutex;

use super::bio::{BioRef, submit_bio};
use super::block_device::BlockDeviceRef;
use super::request::{Request, RequestRef};
use super::sched::{IoSchedQueue, MQ_DEADLINE};

pub struct RequestQueue {
    pub bdev: BlockDeviceRef,
    pub sched_q: Arc<IoSchedQueue>,
    pub depth: u32,
    pub _hw_lock: Mutex<()>,
}

impl RequestQueue {
    pub fn init(bdev: BlockDeviceRef) -> Arc<Self> {
        Arc::new(Self {
            bdev,
            sched_q: IoSchedQueue::new(&MQ_DEADLINE),
            depth: 64,
            _hw_lock: Mutex::new(()),
        })
    }

    /// Insert a bio's request into the scheduler.
    pub fn insert_bio(&self, bio: BioRef) {
        let rq = Request::from_bio(bio);
        (self.sched_q.sched.insert)(&self.sched_q, rq);
    }

    /// Drain all queued requests, dispatching them to the device.
    pub fn run_hw_queue(&self) -> Result<usize, i32> {
        let _g = self._hw_lock.lock();
        let mut n = 0usize;
        while (self.sched_q.sched.has_work)(&self.sched_q) {
            let rq: RequestRef = match (self.sched_q.sched.dispatch)(&self.sched_q) {
                Some(r) => r,
                None => break,
            };
            for bio in rq.bios.iter() {
                submit_bio(bio.clone())?;
                n += 1;
            }
        }
        Ok(n)
    }
}
