//! linux-parity: partial
//! linux-source: vendor/linux/fs/buffer.c
//! test-origin: linux:vendor/linux/fs/buffer.c
//! A small in-memory block buffer cache for the synchronous bio path.
//!
//! Linux serves repeated metadata/data block reads from the page/buffer cache
//! (`__bread` → `find_get_block`), so the disk is touched only on a cold miss.
//! Lupos' `block::submit_bio` was instead issuing a device I/O for *every* read,
//! which is invisible on low-latency transports (virtio-blk under QEMU) but
//! dominates boot under VirtualBox's emulated AHCI (~1 ms per command): systemd
//! reads the same inode-table and directory blocks thousands of times while
//! scanning units, paying that latency each time.
//!
//! This cache closes that gap. It is deliberately conservative for correctness:
//!
//!   * Reads are served from the cache only on an *exact* (device, start-sector,
//!     length) match; a miss reads the device and populates the entry.
//!   * Any write or discard removes **every cached entry whose sector range
//!     overlaps** the written range, so a subsequent read can never observe
//!     stale bytes. (All I/O to a device funnels through `block::submit_bio`, so
//!     this is sufficient — there is no back-door writer.)
//!   * The cache lock is held only for the brief lookup / insert / invalidate;
//!     never across the device I/O itself.
//!
//! The cache is keyed by the `Arc<BlockDevice>` identity, so distinct devices
//! (and distinct partitions) never alias.

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use super::block_device::BlockDeviceRef;

/// Master switch — set to `false` to disable caching entirely.
const ENABLED: bool = true;

/// Largest single entry (in 512-byte sectors) we are willing to cache. Bounds
/// the overlap-scan window and avoids caching unusually large transfers.
const MAX_ENTRY_SECTORS: u64 = 256;

/// Soft cap on total cached bytes; oldest entries are evicted (FIFO) past it.
const CAPACITY_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone)]
struct Entry {
    nr_sectors: u64,
    data: Vec<u8>,
}

struct BlockCache {
    map: BTreeMap<(usize, u64), Entry>,
    fifo: VecDeque<(usize, u64)>,
    bytes: usize,
}

impl BlockCache {
    const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            fifo: VecDeque::new(),
            bytes: 0,
        }
    }

    fn remove_key(&mut self, key: (usize, u64)) {
        if let Some(e) = self.map.remove(&key) {
            self.bytes = self.bytes.saturating_sub(e.data.len());
        }
    }

    /// Invalidate every entry on `dev_id` whose range overlaps
    /// `[sector, sector + nr_sectors)`.
    fn invalidate_overlap(&mut self, dev_id: usize, sector: u64, nr_sectors: u64) {
        let end = sector.saturating_add(nr_sectors);
        let lo = sector.saturating_sub(MAX_ENTRY_SECTORS);
        let doomed: Vec<(usize, u64)> = self
            .map
            .range((dev_id, lo)..(dev_id, end))
            .filter(|((_, start), e)| *start < end && start.saturating_add(e.nr_sectors) > sector)
            .map(|(k, _)| *k)
            .collect();
        for k in doomed {
            self.remove_key(k);
        }
    }

    fn insert(&mut self, key: (usize, u64), entry: Entry) {
        // Replace any existing entry at this exact key first.
        self.remove_key(key);
        while self.bytes + entry.data.len() > CAPACITY_BYTES {
            match self.fifo.pop_front() {
                Some(old) => self.remove_key(old),
                None => break,
            }
        }
        self.bytes += entry.data.len();
        self.fifo.push_back(key);
        self.map.insert(key, entry);
    }
}

static CACHE: Mutex<BlockCache> = Mutex::new(BlockCache::new());

#[inline]
fn dev_id(bdev: &BlockDeviceRef) -> usize {
    Arc::as_ptr(bdev) as usize
}

/// Look up a cached read of exactly `nr_sectors` starting at `sector`.
/// Returns a fresh copy of the bytes on an exact hit.
pub fn lookup(bdev: &BlockDeviceRef, sector: u64, nr_sectors: u64) -> Option<Vec<u8>> {
    if !ENABLED {
        return None;
    }
    let cache = CACHE.lock();
    cache
        .map
        .get(&(dev_id(bdev), sector))
        .filter(|e| e.nr_sectors == nr_sectors)
        .map(|e| e.data.clone())
}

/// Populate the cache with the result of a completed read.
pub fn store(bdev: &BlockDeviceRef, sector: u64, nr_sectors: u64, data: &[u8]) {
    if !ENABLED || nr_sectors == 0 || nr_sectors > MAX_ENTRY_SECTORS {
        return;
    }
    let mut cache = CACHE.lock();
    cache.insert(
        (dev_id(bdev), sector),
        Entry {
            nr_sectors,
            data: data.to_vec(),
        },
    );
}

/// Invalidate every cached entry overlapping a write/discard range.
pub fn invalidate(bdev: &BlockDeviceRef, sector: u64, nr_sectors: u64) {
    if !ENABLED || nr_sectors == 0 {
        return;
    }
    let mut cache = CACHE.lock();
    cache.invalidate_overlap(dev_id(bdev), sector, nr_sectors);
}

/// Drop every cached entry for a device (used for discards, whose range is not
/// described by the bio's data segments).
pub fn invalidate_device(bdev: &BlockDeviceRef) {
    if !ENABLED {
        return;
    }
    let id = dev_id(bdev);
    let mut cache = CACHE.lock();
    let doomed: Vec<(usize, u64)> = cache
        .map
        .range((id, 0)..(id, u64::MAX))
        .map(|(k, _)| *k)
        .collect();
    for k in doomed {
        cache.remove_key(k);
    }
    cache.fifo.retain(|(d, _)| *d != id);
}
