//! linux-parity: complete
//! linux-source: vendor/linux/block/bio.c
//! test-origin: linux:vendor/linux/block/bio.c
//! `struct bio` — the block-layer I/O descriptor.
//!
//! Mirrors `vendor/linux/include/linux/{bio,blk_types}.h` and
//! `vendor/linux/block/bio.c`.  Lupos M43 simplifies: each `Bio` carries an
//! inline `Vec<BioVec>` (no shared-info / fragments yet) and submits
//! synchronously through the device's `BlockDeviceOps::submit_bio`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

use super::block_device::BlockDeviceRef;

// ── Op codes (vendor/linux/include/linux/blk_types.h::REQ_OP_*) ──────────

pub const BIO_OP_READ: u8 = 0;
pub const BIO_OP_WRITE: u8 = 1;
pub const BIO_OP_FLUSH: u8 = 2;
pub const BIO_OP_DISCARD: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BioOp(pub u8);

// ── BioVec — a single contiguous segment ─────────────────────────────────

#[derive(Clone)]
pub struct BioVec {
    pub data: Arc<Mutex<Vec<u8>>>,
    pub off: usize,
    pub len: usize,
}

impl BioVec {
    pub fn new(buf: Vec<u8>) -> Self {
        let len = buf.len();
        Self {
            data: Arc::new(Mutex::new(buf)),
            off: 0,
            len,
        }
    }
}

// ── Bio — the descriptor itself ──────────────────────────────────────────

pub type BioRef = Arc<Bio>;

pub struct Bio {
    pub op: BioOp,
    pub sector: u64, // 512-byte LBA
    pub bdev: BlockDeviceRef,
    pub vecs: Mutex<Vec<BioVec>>,
    pub status: AtomicU32, // 0 = success; non-zero = errno
    pub completion: Mutex<Option<fn(&BioRef)>>,
    pub size: Mutex<usize>, // total bytes across vecs
}

impl Bio {
    pub fn new(bdev: BlockDeviceRef, op: BioOp, sector: u64) -> BioRef {
        Arc::new(Self {
            op,
            sector,
            bdev,
            vecs: Mutex::new(Vec::new()),
            status: AtomicU32::new(0),
            completion: Mutex::new(None),
            size: Mutex::new(0),
        })
    }

    pub fn add_vec(self: &BioRef, v: BioVec) {
        *self.size.lock() += v.len;
        self.vecs.lock().push(v);
    }
    pub fn set_completion(self: &BioRef, f: fn(&BioRef)) {
        *self.completion.lock() = Some(f);
    }
    pub fn total_size(&self) -> usize {
        *self.size.lock()
    }
}

/// Allocate a fresh Bio.
pub fn bio_alloc(bdev: BlockDeviceRef, op: BioOp, sector: u64) -> BioRef {
    Bio::new(bdev, op, sector)
}

/// Submit a Bio.  In M43 this is synchronous: dispatches through the
/// device's `submit_bio` hook and runs the completion callback inline.
pub fn submit_bio(bio: BioRef) -> Result<(), i32> {
    use super::bcache;

    let bdev = bio.bdev.clone();
    let op = bio.op.0;
    let total = bio.total_size();
    let sector = bio.sector;
    let aligned = total != 0 && total % 512 == 0;
    let nr_sectors = (total / 512) as u64;

    // A read of a contiguous, sector-aligned, single-segment range may be served
    // straight from the block buffer cache, skipping the device entirely. This
    // is the hot path for repeated metadata reads at boot.
    if op == BIO_OP_READ && aligned {
        let single = {
            let vecs = bio.vecs.lock();
            vecs.len() == 1 && vecs[0].off == 0 && vecs[0].len == total
        };
        if single {
            if let Some(cached) = bcache::lookup(&bdev, sector, nr_sectors) {
                if cached.len() == total {
                    {
                        let vecs = bio.vecs.lock();
                        *vecs[0].data.lock() = cached;
                    }
                    bio.status.store(0, Ordering::Release);
                    bio_endio(bio);
                    return Ok(());
                }
            }
        }
    }

    let result = (bdev.ops.submit_bio)(&bdev, &bio);
    if let Err(e) = result {
        bio.status.store(e as u32, Ordering::Release);
    }

    // Keep the cache coherent with what just hit the device.
    if result.is_ok() {
        match op {
            BIO_OP_WRITE => bcache::invalidate(&bdev, sector, nr_sectors.max(1)),
            // Discard/flush ranges are not described by the data vecs; drop the
            // whole device's cache rather than risk a stale read.
            BIO_OP_DISCARD => bcache::invalidate_device(&bdev),
            BIO_OP_READ if aligned => {
                let data = {
                    let vecs = bio.vecs.lock();
                    if vecs.len() == 1 && vecs[0].off == 0 && vecs[0].len == total {
                        Some(vecs[0].data.lock().clone())
                    } else {
                        None
                    }
                };
                if let Some(data) = data {
                    if data.len() == total {
                        bcache::store(&bdev, sector, nr_sectors, &data);
                    }
                }
            }
            _ => {}
        }
    }

    bio_endio(bio);
    result
}

/// Mark `bio` complete and invoke its registered callback (if any).
pub fn bio_endio(bio: BioRef) {
    let cb = bio.completion.lock().take();
    if let Some(cb) = cb {
        cb(&bio);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::block_device::{BlockDevice, BlockDeviceOps};
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    #[test]
    fn bio_round_trip_through_mem_device() {
        let mem = MemBlockDevice::new("test0", 1 << 16);
        let bdev = BlockDevice::wrap(mem.clone(), &mem_block_device_ops());
        // Write
        let w = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), 0);
        w.add_vec(BioVec::new(alloc::vec![0xAB; 512]));
        submit_bio(w).unwrap();
        // Read back
        let r = bio_alloc(bdev.clone(), BioOp(BIO_OP_READ), 0);
        let buf = alloc::vec![0u8; 512];
        r.add_vec(BioVec::new(buf));
        submit_bio(r.clone()).unwrap();
        let v = r.vecs.lock();
        let g = v[0].data.lock();
        assert!(g.iter().all(|&b| b == 0xAB));
    }
}
