//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pat/memtype_interval.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pat/memtype_interval.c
//! PAT memtype interval conflict checks.
//!
//! Mirrors the interval tree behavior from
//! `vendor/linux/arch/x86/mm/pat/memtype_interval.c`. The implementation here
//! is slice-backed and deterministic: callers own storage, and this module
//! owns conflict classification.

use crate::arch::x86::mm::pat::PageCacheMode;
use crate::include::uapi::errno::{EBUSY, EINVAL, ENOENT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Memtype {
    pub start: u64,
    pub end: u64,
    pub cache_mode: PageCacheMode,
}

impl Memtype {
    pub const fn new(start: u64, end: u64, cache_mode: PageCacheMode) -> Result<Self, i32> {
        if end <= start {
            return Err(EINVAL);
        }
        Ok(Self {
            start,
            end,
            cache_mode,
        })
    }
}

pub const fn overlaps(a: Memtype, b: Memtype) -> bool {
    a.start < b.end && b.start < a.end
}

pub const fn compatible(existing: PageCacheMode, requested: PageCacheMode) -> bool {
    matches!(
        (existing, requested),
        (PageCacheMode::WriteBack, PageCacheMode::WriteBack)
            | (PageCacheMode::WriteCombining, PageCacheMode::WriteCombining)
            | (PageCacheMode::UncachedMinus, PageCacheMode::UncachedMinus)
            | (PageCacheMode::Uncached, PageCacheMode::Uncached)
            | (PageCacheMode::WriteThrough, PageCacheMode::WriteThrough)
            | (PageCacheMode::WriteProtected, PageCacheMode::WriteProtected)
    )
}

pub fn memtype_check_insert(
    existing: &[Memtype],
    entry_new: Memtype,
) -> Result<PageCacheMode, i32> {
    let mut i = 0;
    while i < existing.len() {
        let entry = existing[i];
        if overlaps(entry, entry_new) {
            if compatible(entry.cache_mode, entry_new.cache_mode) {
                return Ok(entry.cache_mode);
            }
            return Err(EBUSY);
        }
        i += 1;
    }
    Ok(entry_new.cache_mode)
}

pub fn memtype_lookup(existing: &[Memtype], addr: u64) -> Option<Memtype> {
    let mut i = 0;
    while i < existing.len() {
        let entry = existing[i];
        if addr >= entry.start && addr < entry.end {
            return Some(entry);
        }
        i += 1;
    }
    None
}

pub fn memtype_erase(existing: &[Memtype], start: u64, end: u64) -> Result<usize, i32> {
    if end <= start {
        return Err(EINVAL);
    }
    let mut i = 0;
    while i < existing.len() {
        let entry = existing[i];
        if entry.start == start && entry.end == end {
            return Ok(i);
        }
        i += 1;
    }
    Err(ENOENT)
}

pub fn memtype_copy_nth_element(existing: &[Memtype], pos: usize) -> Result<Memtype, i32> {
    existing.get(pos).copied().ok_or(ENOENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_same_type_is_compatible() {
        let existing = [Memtype::new(0x1000, 0x3000, PageCacheMode::WriteCombining).unwrap()];
        let new = Memtype::new(0x2000, 0x4000, PageCacheMode::WriteCombining).unwrap();
        assert_eq!(
            memtype_check_insert(&existing, new),
            Ok(PageCacheMode::WriteCombining)
        );
    }

    #[test]
    fn overlapping_different_type_is_busy() {
        let existing = [Memtype::new(0x1000, 0x3000, PageCacheMode::WriteCombining).unwrap()];
        let new = Memtype::new(0x2000, 0x4000, PageCacheMode::Uncached).unwrap();
        assert_eq!(memtype_check_insert(&existing, new), Err(EBUSY));
    }

    #[test]
    fn lookup_and_erase_are_range_exact() {
        let existing = [Memtype::new(0x1000, 0x3000, PageCacheMode::Uncached).unwrap()];
        assert_eq!(
            memtype_lookup(&existing, 0x2000).unwrap().cache_mode,
            PageCacheMode::Uncached
        );
        assert_eq!(memtype_erase(&existing, 0x1000, 0x3000), Ok(0));
        assert_eq!(memtype_erase(&existing, 0x1000, 0x2000), Err(ENOENT));
    }
}
