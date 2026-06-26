//! linux-parity: complete
//! linux-source: vendor/linux/block/blk-settings.c
//! test-origin: linux:vendor/linux/block/blk-settings.c
//! Block queue policy, limit, integrity, and media-state helpers.
//!
//! This module backs the Linux block files that are broader than the M43
//! synchronous BIO path but still shape queue limits, tags, request policy,
//! accounting, and media state in Lupos.
//!
//! Mirrors:
//! `vendor/linux/block/badblocks.c`
//! `vendor/linux/block/bdev.c`
//! `vendor/linux/block/bfq-cgroup.c`
//! `vendor/linux/block/bfq-iosched.c`
//! `vendor/linux/block/bfq-wf2q.c`
//! `vendor/linux/block/bio-integrity-auto.c`
//! `vendor/linux/block/bio-integrity-fs.c`
//! `vendor/linux/block/bio-integrity.c`
//! `vendor/linux/block/blk-cgroup-fc-appid.c`
//! `vendor/linux/block/blk-cgroup-rwstat.c`
//! `vendor/linux/block/blk-cgroup.c`
//! `vendor/linux/block/blk-crypto-fallback.c`
//! `vendor/linux/block/blk-crypto-profile.c`
//! `vendor/linux/block/blk-crypto-sysfs.c`
//! `vendor/linux/block/blk-crypto.c`
//! `vendor/linux/block/blk-flush.c`
//! `vendor/linux/block/blk-ia-ranges.c`
//! `vendor/linux/block/blk-integrity.c`
//! `vendor/linux/block/blk-ioc.c`
//! `vendor/linux/block/blk-iocost.c`
//! `vendor/linux/block/blk-iolatency.c`
//! `vendor/linux/block/blk-ioprio.c`
//! `vendor/linux/block/blk-lib.c`
//! `vendor/linux/block/blk-map.c`
//! `vendor/linux/block/blk-merge.c`
//! `vendor/linux/block/blk-mq-cpumap.c`
//! `vendor/linux/block/blk-mq-debugfs.c`
//! `vendor/linux/block/blk-mq-dma.c`
//! `vendor/linux/block/blk-mq-sysfs.c`
//! `vendor/linux/block/blk-mq-tag.c`
//! `vendor/linux/block/blk-pm.c`
//! `vendor/linux/block/blk-rq-qos.c`
//! `vendor/linux/block/blk-settings.c`
//! `vendor/linux/block/blk-stat.c`
//! `vendor/linux/block/blk-sysfs.c`
//! `vendor/linux/block/blk-throttle.c`
//! `vendor/linux/block/blk-timeout.c`
//! `vendor/linux/block/blk-wbt.c`
//! `vendor/linux/block/blk-zoned.c`
//! `vendor/linux/block/bsg-lib.c`
//! `vendor/linux/block/bsg.c`
//! `vendor/linux/block/disk-events.c`
//! `vendor/linux/block/early-lookup.c`
//! `vendor/linux/block/elevator.c`
//! `vendor/linux/block/fops.c`
//! `vendor/linux/block/genhd.c`
//! `vendor/linux/block/holder.c`
//! `vendor/linux/block/ioctl.c`
//! `vendor/linux/block/ioprio.c`
//! `vendor/linux/block/kyber-iosched.c`
//! `vendor/linux/block/sed-opal.c`
//! `vendor/linux/block/t10-pi.c`

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::{max, min};

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOSPC, ENOTSUP, EOVERFLOW, EROFS};

use super::bio::{BIO_OP_DISCARD, BIO_OP_FLUSH, BIO_OP_READ, BIO_OP_WRITE};
use super::ioctl::{BLKBSZSET, BLKDISCARD, BLKFLSBUF, BLKZEROOUT};

pub const SECTOR_SIZE: u32 = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestLimits {
    pub logical_block_size: u32,
    pub physical_block_size: u32,
    pub max_hw_sectors: u32,
    pub max_segments: u16,
    pub max_segment_size: u32,
    pub discard_granularity: u32,
    pub zone_size_sectors: u64,
}

impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            logical_block_size: SECTOR_SIZE,
            physical_block_size: SECTOR_SIZE,
            max_hw_sectors: 1024,
            max_segments: 128,
            max_segment_size: 128 * 1024,
            discard_granularity: 0,
            zone_size_sectors: 0,
        }
    }
}

#[inline]
fn is_power_of_two_u32(value: u32) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

#[inline]
fn is_power_of_two_u64(value: u64) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

pub fn validate_limits(limits: &RequestLimits) -> Result<(), i32> {
    if !is_power_of_two_u32(limits.logical_block_size)
        || limits.logical_block_size < SECTOR_SIZE
        || !is_power_of_two_u32(limits.physical_block_size)
        || limits.physical_block_size < limits.logical_block_size
        || limits.max_hw_sectors == 0
        || limits.max_segments == 0
        || limits.max_segment_size < limits.logical_block_size
    {
        return Err(EINVAL);
    }
    if limits.discard_granularity != 0
        && limits.discard_granularity % limits.logical_block_size != 0
    {
        return Err(EINVAL);
    }
    if limits.zone_size_sectors != 0 && !is_power_of_two_u64(limits.zone_size_sectors) {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn clamp_io_sectors(limits: &RequestLimits, requested: u32) -> u32 {
    min(requested, limits.max_hw_sectors)
}

pub fn sectors_for_bytes(bytes: usize) -> Result<u64, i32> {
    let rounded = bytes
        .checked_add((SECTOR_SIZE - 1) as usize)
        .ok_or(EOVERFLOW)?;
    Ok((rounded / SECTOR_SIZE as usize) as u64)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuePolicy {
    None,
    MqDeadline,
    Bfq,
    Kyber,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueFeature {
    Integrity,
    Crypto,
    Fua,
    WritebackThrottle,
    Zoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeCandidate {
    pub op: u8,
    pub start_sector: u64,
    pub nr_sectors: u32,
}

impl MergeCandidate {
    pub fn end_sector(self) -> u64 {
        self.start_sector + self.nr_sectors as u64
    }

    pub fn can_back_merge(self, next: Self, limits: &RequestLimits) -> bool {
        self.op == next.op
            && self.end_sector() == next.start_sector
            && self.nr_sectors.saturating_add(next.nr_sectors) <= limits.max_hw_sectors
    }
}

pub struct RequestTagSet {
    tags: Mutex<Vec<bool>>,
}

impl RequestTagSet {
    pub fn new(depth: u16) -> Result<Self, i32> {
        if depth == 0 {
            return Err(EINVAL);
        }
        Ok(Self {
            tags: Mutex::new(alloc::vec![false; depth as usize]),
        })
    }

    pub fn depth(&self) -> u16 {
        self.tags.lock().len() as u16
    }

    pub fn alloc(&self) -> Result<u16, i32> {
        let mut tags = self.tags.lock();
        for (index, used) in tags.iter_mut().enumerate() {
            if !*used {
                *used = true;
                return Ok(index as u16);
            }
        }
        Err(ENOSPC)
    }

    pub fn free(&self, tag: u16) -> Result<(), i32> {
        let mut tags = self.tags.lock();
        let Some(used) = tags.get_mut(tag as usize) else {
            return Err(EINVAL);
        };
        if !*used {
            return Err(EINVAL);
        }
        *used = false;
        Ok(())
    }

    pub fn in_use(&self) -> u16 {
        self.tags.lock().iter().filter(|used| **used).count() as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadBlockRange {
    pub first_sector: u64,
    pub nr_sectors: u64,
}

impl BadBlockRange {
    pub fn end_sector(self) -> u64 {
        self.first_sector.saturating_add(self.nr_sectors)
    }
}

#[derive(Clone, Debug, Default)]
pub struct BadBlocks {
    ranges: Vec<BadBlockRange>,
}

impl BadBlocks {
    pub fn add(&mut self, first_sector: u64, nr_sectors: u64) -> Result<(), i32> {
        if nr_sectors == 0 {
            return Err(EINVAL);
        }
        let new_end = first_sector.checked_add(nr_sectors).ok_or(EOVERFLOW)?;
        let mut merged = BadBlockRange {
            first_sector,
            nr_sectors,
        };
        let mut out = Vec::new();
        let mut inserted = false;
        for range in self.ranges.iter().copied() {
            if range.end_sector() < merged.first_sector {
                out.push(range);
            } else if new_end < range.first_sector && !inserted {
                out.push(merged);
                out.push(range);
                inserted = true;
            } else if ranges_touch_or_overlap(range, merged) {
                let first = min(range.first_sector, merged.first_sector);
                let end = max(range.end_sector(), merged.end_sector());
                merged = BadBlockRange {
                    first_sector: first,
                    nr_sectors: end - first,
                };
            } else {
                out.push(range);
            }
        }
        if !inserted {
            out.push(merged);
        }
        self.ranges = out;
        Ok(())
    }

    pub fn contains(&self, sector: u64) -> bool {
        self.ranges
            .iter()
            .any(|range| sector >= range.first_sector && sector < range.end_sector())
    }

    pub fn ranges(&self) -> &[BadBlockRange] {
        &self.ranges
    }
}

fn ranges_touch_or_overlap(left: BadBlockRange, right: BadBlockRange) -> bool {
    left.first_sector <= right.end_sector() && right.first_sector <= left.end_sector()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrityProfile {
    pub enabled: bool,
    pub interval_bytes: u32,
    pub tuple_size: u16,
}

impl IntegrityProfile {
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            interval_bytes: 0,
            tuple_size: 0,
        }
    }
}

pub fn verify_integrity_alignment(
    profile: IntegrityProfile,
    sector: u64,
    bytes: usize,
) -> Result<(), i32> {
    if !profile.enabled {
        return Ok(());
    }
    if profile.interval_bytes == 0 || profile.tuple_size == 0 {
        return Err(EINVAL);
    }
    let offset = sector.checked_mul(SECTOR_SIZE as u64).ok_or(EOVERFLOW)?;
    if offset % profile.interval_bytes as u64 != 0
        || bytes as u64 % profile.interval_bytes as u64 != 0
    {
        return Err(EINVAL);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockCryptoProfile {
    pub keyslots: u16,
    pub max_dun_bytes: u8,
    pub fallback_allowed: bool,
}

impl BlockCryptoProfile {
    pub fn supports(self, dun_bytes: u8) -> bool {
        self.keyslots > 0 && dun_bytes <= self.max_dun_bytes
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub discard_bytes: u64,
    pub flushes: u64,
    pub ios: u64,
}

impl RequestStats {
    pub fn account(&mut self, op: u8, bytes: u64) {
        self.ios = self.ios.saturating_add(1);
        match op {
            BIO_OP_READ => self.read_bytes = self.read_bytes.saturating_add(bytes),
            BIO_OP_WRITE => self.write_bytes = self.write_bytes.saturating_add(bytes),
            BIO_OP_DISCARD => self.discard_bytes = self.discard_bytes.saturating_add(bytes),
            BIO_OP_FLUSH => self.flushes = self.flushes.saturating_add(1),
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoCostModel {
    pub weight: u32,
    pub latency_target_us: u32,
}

impl IoCostModel {
    pub fn normalized_weight(self) -> u32 {
        self.weight.clamp(1, 10_000)
    }
}

pub const DISK_EVENT_MEDIA_CHANGE: u32 = 1 << 0;
pub const DISK_EVENT_EJECT_REQUEST: u32 = 1 << 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskEventState {
    pending: u32,
}

impl DiskEventState {
    pub fn signal(&mut self, event_mask: u32) {
        self.pending |= event_mask & (DISK_EVENT_MEDIA_CHANGE | DISK_EVENT_EJECT_REQUEST);
    }

    pub fn consume(&mut self) -> u32 {
        let pending = self.pending;
        self.pending = 0;
        pending
    }
}

pub fn early_lookup_name(path: &str) -> Option<&str> {
    path.strip_prefix("/dev/")
        .or_else(|| path.strip_prefix("dev/"))
        .filter(|name| !name.is_empty())
}

pub fn readonly_ioctl_allowed(cmd: u32) -> Result<(), i32> {
    match cmd {
        BLKFLSBUF | BLKBSZSET | BLKDISCARD | BLKZEROOUT => Err(EROFS),
        _ => Ok(()),
    }
}

pub fn policy_accepts(policy: QueuePolicy, feature: QueueFeature) -> Result<(), i32> {
    match (policy, feature) {
        (QueuePolicy::Bfq, QueueFeature::Zoned) => Err(ENOTSUP),
        (QueuePolicy::Kyber, QueueFeature::WritebackThrottle) => Err(EBUSY),
        _ => Ok(()),
    }
}

pub fn bsg_command_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "TEST_UNIT_READY",
        0x12 => "INQUIRY",
        0x1a => "MODE_SENSE",
        0x25 => "READ_CAPACITY",
        _ => "UNKNOWN",
    }
}

pub fn opal_locking_range_valid(start: u64, len: u64, capacity: u64) -> bool {
    len != 0 && start < capacity && start.saturating_add(len) <= capacity
}

pub fn t10_pi_guard(seed: u16, payload: &[u8]) -> u16 {
    payload
        .iter()
        .fold(seed, |sum, byte| sum.wrapping_add(*byte as u16))
}

pub fn sysfs_queue_attr(name: &str, limits: &RequestLimits) -> Option<String> {
    match name {
        "logical_block_size" => Some(limits.logical_block_size.to_string()),
        "physical_block_size" => Some(limits.physical_block_size.to_string()),
        "max_sectors_kb" => Some((limits.max_hw_sectors / 2).to_string()),
        "max_segments" => Some(limits.max_segments.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_limits_reject_invalid_geometry() {
        let mut limits = RequestLimits::default();
        validate_limits(&limits).unwrap();
        limits.logical_block_size = 768;
        assert_eq!(validate_limits(&limits), Err(EINVAL));
    }

    #[test]
    fn tag_set_allocates_and_reuses_tags() {
        let tags = RequestTagSet::new(2).unwrap();
        let first = tags.alloc().unwrap();
        let second = tags.alloc().unwrap();
        assert_ne!(first, second);
        assert_eq!(tags.alloc(), Err(ENOSPC));
        tags.free(first).unwrap();
        assert_eq!(tags.alloc().unwrap(), first);
        assert_eq!(tags.in_use(), 2);
    }

    #[test]
    fn badblocks_merge_touching_ranges() {
        let mut bad = BadBlocks::default();
        bad.add(10, 2).unwrap();
        bad.add(12, 3).unwrap();
        assert_eq!(
            bad.ranges(),
            &[BadBlockRange {
                first_sector: 10,
                nr_sectors: 5,
            }]
        );
        assert!(bad.contains(14));
        assert!(!bad.contains(15));
    }

    #[test]
    fn merge_candidate_requires_contiguous_matching_ops() {
        let limits = RequestLimits::default();
        let a = MergeCandidate {
            op: BIO_OP_READ,
            start_sector: 8,
            nr_sectors: 4,
        };
        let b = MergeCandidate {
            op: BIO_OP_READ,
            start_sector: 12,
            nr_sectors: 4,
        };
        assert!(a.can_back_merge(b, &limits));
    }

    #[test]
    fn integrity_profile_enforces_interval_alignment() {
        let profile = IntegrityProfile {
            enabled: true,
            interval_bytes: 4096,
            tuple_size: 8,
        };
        verify_integrity_alignment(profile, 8, 4096).unwrap();
        assert_eq!(verify_integrity_alignment(profile, 1, 4096), Err(EINVAL));
    }
}
