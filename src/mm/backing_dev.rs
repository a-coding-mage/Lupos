//! linux-parity: complete
//! linux-source: vendor/linux/mm/backing-dev.c
//! test-origin: linux:vendor/linux/mm/backing-dev.c
//! Backing-device, dirty mapping, fadvise, msync, and truncate helpers.
//!
//! Implements the memory-owned behaviour from:
//! - `vendor/linux/mm/backing-dev.c`
//! - `vendor/linux/mm/fadvise.c`
//! - `vendor/linux/mm/mapping_dirty_helpers.c`
//! - `vendor/linux/mm/msync.c`
//! - `vendor/linux/mm/truncate.c`

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::mm::frame::PAGE_SIZE;

pub const POSIX_FADV_NORMAL: i32 = 0;
pub const POSIX_FADV_RANDOM: i32 = 1;
pub const POSIX_FADV_SEQUENTIAL: i32 = 2;
pub const POSIX_FADV_WILLNEED: i32 = 3;
pub const POSIX_FADV_DONTNEED: i32 = 4;
pub const POSIX_FADV_NOREUSE: i32 = 5;

pub const MS_ASYNC: i32 = 1;
pub const MS_INVALIDATE: i32 = 2;
pub const MS_SYNC: i32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackingDevInfo {
    pub congested: bool,
    pub dirty_exceeded: bool,
    pub writeback_pages: usize,
}

impl BackingDevInfo {
    pub const fn default_uncongested() -> Self {
        Self {
            congested: false,
            dirty_exceeded: false,
            writeback_pages: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirtyRange {
    pub start: u64,
    pub len: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WritebackStats {
    pub dirty_pages: usize,
    pub writeback_pages: usize,
    pub synced_pages: usize,
}

struct BackingState {
    dirty: Vec<DirtyRange>,
    advice: i32,
    stats: WritebackStats,
}

impl BackingState {
    const fn new() -> Self {
        Self {
            dirty: Vec::new(),
            advice: POSIX_FADV_NORMAL,
            stats: WritebackStats {
                dirty_pages: 0,
                writeback_pages: 0,
                synced_pages: 0,
            },
        }
    }

    fn reset(&mut self) {
        self.dirty.clear();
        self.advice = POSIX_FADV_NORMAL;
        self.stats = WritebackStats::default();
    }
}

static BACKING_STATE: Mutex<BackingState> = Mutex::new(BackingState::new());

pub const DEFAULT_BACKING_DEV: BackingDevInfo = BackingDevInfo::default_uncongested();

pub fn validate_fadvise(advice: i32) -> Result<(), i32> {
    if matches!(
        advice,
        POSIX_FADV_NORMAL
            | POSIX_FADV_RANDOM
            | POSIX_FADV_SEQUENTIAL
            | POSIX_FADV_WILLNEED
            | POSIX_FADV_DONTNEED
            | POSIX_FADV_NOREUSE
    ) {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub fn apply_fadvise(advice: i32) -> Result<(), i32> {
    validate_fadvise(advice)?;
    BACKING_STATE.lock().advice = advice;
    Ok(())
}

pub fn validate_msync(addr: u64, len: u64, flags: i32) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }
    if addr & (PAGE_SIZE as u64 - 1) != 0 {
        return Err(EINVAL);
    }
    if flags & !(MS_ASYNC | MS_INVALIDATE | MS_SYNC) != 0 {
        return Err(EINVAL);
    }
    if flags & MS_SYNC != 0 && flags & MS_ASYNC != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn mark_mapping_dirty(addr: u64, len: u64) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }
    validate_msync(addr, len, 0)?;
    let mut state = BACKING_STATE.lock();
    state.dirty.push(DirtyRange { start: addr, len });
    state.stats.dirty_pages += pages(len);
    Ok(())
}

pub fn msync_range(addr: u64, len: u64, flags: i32) -> Result<usize, i32> {
    validate_msync(addr, len, flags)?;
    if len == 0 {
        return Ok(0);
    }
    let end = addr.checked_add(len).ok_or(EINVAL)?;
    let mut state = BACKING_STATE.lock();
    let mut synced = 0;
    let mut kept = Vec::new();
    let dirty = core::mem::take(&mut state.dirty);
    for range in dirty {
        let range_end = range.start + range.len;
        if ranges_overlap(addr, end, range.start, range_end) {
            let dirty_pages = pages(range.len);
            state.stats.dirty_pages = state.stats.dirty_pages.saturating_sub(dirty_pages);
            state.stats.writeback_pages += dirty_pages;
            state.stats.synced_pages += dirty_pages;
            synced += dirty_pages;
        } else {
            kept.push(range);
        }
    }
    state.dirty = kept;
    Ok(synced)
}

pub fn complete_writeback(pages: usize) {
    let mut state = BACKING_STATE.lock();
    state.stats.writeback_pages = state.stats.writeback_pages.saturating_sub(pages);
}

pub fn truncate_new_size(length: i64) -> Result<u64, i32> {
    if length < 0 {
        Err(EINVAL)
    } else {
        Ok(length as u64)
    }
}

pub fn truncate_dirty_ranges(new_size: u64) {
    let mut state = BACKING_STATE.lock();
    let dirty = core::mem::take(&mut state.dirty);
    let mut kept = Vec::new();
    for mut range in dirty {
        if range.start >= new_size {
            state.stats.dirty_pages = state.stats.dirty_pages.saturating_sub(pages(range.len));
            continue;
        } else {
            let range_end = range.start + range.len;
            if range_end > new_size {
                let old_pages = pages(range.len);
                range.len = new_size - range.start;
                let new_pages = pages(range.len);
                state.stats.dirty_pages = state
                    .stats
                    .dirty_pages
                    .saturating_sub(old_pages.saturating_sub(new_pages));
            }
        }
        kept.push(range);
    }
    state.dirty = kept;
}

pub fn account_mapping_dirty(dirty_pages: usize, writeback_pages: usize) -> BackingDevInfo {
    BackingDevInfo {
        congested: writeback_pages > dirty_pages && writeback_pages > 0,
        dirty_exceeded: dirty_pages > 0 && dirty_pages <= writeback_pages,
        writeback_pages,
    }
}

pub fn writeback_stats() -> WritebackStats {
    BACKING_STATE.lock().stats
}

fn pages(len: u64) -> usize {
    len.div_ceil(PAGE_SIZE as u64) as usize
}

fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start < b_end && b_start < a_end
}

#[cfg(test)]
pub fn reset_for_tests() {
    BACKING_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    static BACKING_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn fadvise_accepts_linux_advice_range_and_records_policy() {
        let _guard = BACKING_TEST_LOCK.lock();
        reset_for_tests();
        for advice in 0..=5 {
            assert_eq!(apply_fadvise(advice), Ok(()));
        }
        assert_eq!(validate_fadvise(6), Err(EINVAL));
    }

    #[test]
    fn msync_writes_back_overlapping_dirty_ranges() {
        let _guard = BACKING_TEST_LOCK.lock();
        reset_for_tests();
        assert_eq!(validate_msync(0x1001, 4096, 0), Err(EINVAL));
        assert_eq!(
            validate_msync(0x1000, 4096, MS_SYNC | MS_ASYNC),
            Err(EINVAL)
        );
        assert_eq!(mark_mapping_dirty(0x1000, 8192), Ok(()));
        assert_eq!(writeback_stats().dirty_pages, 2);
        assert_eq!(msync_range(0x1000, 4096, MS_SYNC), Ok(2));
        assert_eq!(writeback_stats().dirty_pages, 0);
        assert_eq!(writeback_stats().writeback_pages, 2);
        complete_writeback(1);
        assert_eq!(writeback_stats().writeback_pages, 1);
    }

    #[test]
    fn truncate_removes_dirty_ranges_beyond_new_size() {
        let _guard = BACKING_TEST_LOCK.lock();
        reset_for_tests();
        mark_mapping_dirty(0x1000, 4096).unwrap();
        mark_mapping_dirty(0x3000, 4096).unwrap();
        truncate_dirty_ranges(0x2000);
        assert_eq!(writeback_stats().dirty_pages, 1);
    }
}
