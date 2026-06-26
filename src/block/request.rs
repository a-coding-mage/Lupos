//! linux-parity: partial
//! linux-source: vendor/linux/block
//! `struct request` — a request_queue entry built from one or more bios.
//!
//! Mirrors `vendor/linux/include/linux/blk-mq.h::struct request` and
//! `block/blk-core.c::blk_mq_alloc_request`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::bio::BioRef;

static NEXT_RQ_ID: AtomicU64 = AtomicU64::new(1);

pub type RequestRef = Arc<Request>;

pub struct Request {
    pub id: u64,
    pub start_sector: u64,
    pub bios: Vec<BioRef>,
}

impl Request {
    pub fn from_bio(bio: BioRef) -> RequestRef {
        Arc::new(Self {
            id: NEXT_RQ_ID.fetch_add(1, Ordering::AcqRel),
            start_sector: bio.sector,
            bios: alloc::vec![bio],
        })
    }
}
