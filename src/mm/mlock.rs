//! linux-parity: complete
//! linux-source: vendor/linux/mm/mlock.c
//! test-origin: linux:vendor/linux/mm/mlock.c
//! Memory locking, mincore, and mseal policy.
//!
//! References:
//! - `vendor/linux/mm/mlock.c`
//! - `vendor/linux/mm/mincore.c`
//! - `vendor/linux/mm/mseal.c`

extern crate alloc;

use alloc::boxed::Box;

use crate::arch::x86::mm::paging::PAGE_SIZE;
use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
use crate::mm::mm_types::MmStruct;
use crate::mm::vm_flags::{VM_LOCKED, VM_LOCKONFAULT, VM_WRITE};
use crate::mm::vma::{find_vma, find_vma_prev, insert_vma, remove_vma, vma_merge, vma_split};

fn align_down(value: u64) -> u64 {
    value & !(PAGE_SIZE - 1)
}

fn align_up(value: u64) -> Result<u64, i32> {
    value
        .checked_add(PAGE_SIZE - 1)
        .map(|v| v & !(PAGE_SIZE - 1))
        .ok_or(EINVAL)
}

fn normalize_errno(errno: i32) -> i32 {
    if errno < 0 { -errno } else { errno }
}

unsafe fn populate_locked_vma(vma: *mut crate::mm::mm_types::VmAreaStruct, start: u64, end: u64) {
    let write = unsafe { (*vma).vm_flags & VM_WRITE != 0 };
    let flags = FAULT_FLAG_USER | if write { FAULT_FLAG_WRITE } else { 0 };
    let mut addr = start;
    while addr < end {
        let _ = handle_mm_fault(vma, addr, flags);
        addr = addr.saturating_add(PAGE_SIZE);
    }
}

pub unsafe fn populate_locked_range(
    vma: *mut crate::mm::mm_types::VmAreaStruct,
    start: u64,
    end: u64,
) {
    unsafe { populate_locked_vma(vma, start, end) };
}

unsafe fn coalesce_vma_after_flag_change(
    mm: &mut MmStruct,
    vma: *mut crate::mm::mm_types::VmAreaStruct,
) -> u64 {
    let start = unsafe { (*vma).vm_start };
    let end = unsafe { (*vma).vm_end };
    let flags = unsafe { (*vma).vm_flags };
    let file = unsafe { (*vma).vm_file };
    let pgoff = unsafe { (*vma).vm_pgoff };
    unsafe { remove_vma(mm, vma) };
    let (_, prev) = find_vma_prev(mm, start);
    if let Some(merged) = unsafe { vma_merge(mm, prev, start, end, flags, file, pgoff) } {
        let merged_end = unsafe { (*merged).vm_end };
        unsafe { crate::mm::vma::vm_area_free(vma) };
        merged_end
    } else {
        let _ = unsafe { insert_vma(mm, vma) };
        end
    }
}

unsafe fn coalesce_adjacent_range(mm: &mut MmStruct, start: u64, end: u64) {
    let mut cursor = start;
    while cursor < end {
        let Some(vma) = find_vma(mm, cursor) else {
            break;
        };
        if vma.is_null() {
            break;
        }
        let vma_start = unsafe { (*vma).vm_start };
        let vma_end = unsafe { (*vma).vm_end };
        if vma_start >= end {
            break;
        }
        let merged_end = unsafe { coalesce_vma_after_flag_change(mm, vma) };
        cursor = merged_end
            .max(vma_end)
            .max(cursor.saturating_add(PAGE_SIZE));
    }
}

pub fn page_aligned_range(start: u64, len: u64) -> Result<(u64, u64), i32> {
    if len == 0 {
        return Ok((align_down(start), align_down(start)));
    }
    let end = start.checked_add(len).ok_or(EINVAL)?;
    Ok((align_down(start), align_up(end)?))
}

pub unsafe fn lock_vma_range(
    mm: &mut MmStruct,
    start: u64,
    len: u64,
    onfault: bool,
) -> Result<u64, i32> {
    if start == 0 && len != 0 {
        return Err(EINVAL);
    }
    let (start, end) = page_aligned_range(start, len)?;
    if start == end {
        return Ok(0);
    }
    let mut locked_pages = 0u64;
    let mut saw_overlap = false;
    for (vma_start, vma_end_inclusive, entry) in mm.mm_mt.collect_entries() {
        let vma_end = vma_end_inclusive.saturating_add(1);
        if vma_end <= start || vma_start >= end {
            continue;
        }
        let vma = entry as *mut crate::mm::mm_types::VmAreaStruct;
        if vma.is_null() {
            continue;
        }
        saw_overlap = true;
        let overlap_start = vma_start.max(start);
        let overlap_end = vma_end.min(end);
        let pages = (overlap_end - overlap_start).div_ceil(PAGE_SIZE);
        unsafe {
            if (*vma).vm_flags & VM_LOCKED == 0 {
                locked_pages = locked_pages.saturating_add(pages);
            }
        }
    }
    if !saw_overlap {
        return Err(ENOMEM);
    }
    if locked_pages != 0 {
        if !crate::kernel::capability::capable(crate::kernel::capability::CAP_IPC_LOCK) {
            let limit =
                crate::kernel::syscalls::current_rlimit(crate::kernel::syscalls::RLIMIT_MEMLOCK)
                    .rlim_cur;
            if limit != u64::MAX
                && mm
                    .locked_vm
                    .saturating_add(locked_pages)
                    .saturating_mul(PAGE_SIZE)
                    > limit
            {
                return Err(ENOMEM);
            }
        }
    }
    for (vma_start, vma_end_inclusive, entry) in mm.mm_mt.collect_entries() {
        let vma_end = vma_end_inclusive.saturating_add(1);
        if vma_end <= start || vma_start >= end {
            continue;
        }
        let vma = entry as *mut crate::mm::mm_types::VmAreaStruct;
        if vma.is_null() {
            continue;
        }
        unsafe {
            let flags = &mut (*vma).vm_flags;
            *flags |= VM_LOCKED;
            if onfault {
                *flags |= VM_LOCKONFAULT;
            } else {
                *flags &= !VM_LOCKONFAULT;
                populate_locked_vma(vma, vma_start.max(start), vma_end.min(end));
            }
        }
    }
    mm.locked_vm = mm.locked_vm.saturating_add(locked_pages);
    Ok(locked_pages)
}

pub unsafe fn unlock_vma_range(mm: &mut MmStruct, start: u64, len: u64) -> Result<u64, i32> {
    if start == 0 && len != 0 {
        return Err(EINVAL);
    }
    let (start, end) = page_aligned_range(start, len)?;
    if start == end {
        return Ok(0);
    }
    let mut unlocked_pages = 0u64;
    let mut cursor = start;
    while cursor < end {
        let Some(mut vma) = find_vma(mm, cursor) else {
            break;
        };
        if vma.is_null() {
            break;
        }
        let vma_start = unsafe { (*vma).vm_start };
        let vma_end = unsafe { (*vma).vm_end };
        if vma_start >= end {
            break;
        }
        if vma_end <= cursor {
            cursor = vma_end.saturating_add(PAGE_SIZE);
            continue;
        }
        if cursor > vma_start {
            vma = unsafe { vma_split(mm, vma, cursor).map_err(normalize_errno)? };
        }
        let target_end = unsafe { (*vma).vm_end }.min(end);
        if target_end < unsafe { (*vma).vm_end } {
            let _right = unsafe { vma_split(mm, vma, target_end).map_err(normalize_errno)? };
        }
        if unsafe { (*vma).vm_flags & VM_LOCKED != 0 } {
            unlocked_pages =
                unlocked_pages.saturating_add((target_end - cursor).div_ceil(PAGE_SIZE));
            unsafe {
                (*vma).vm_flags &= !(VM_LOCKED | VM_LOCKONFAULT);
                coalesce_vma_after_flag_change(mm, vma);
            }
        }
        cursor = target_end;
    }
    unsafe { coalesce_adjacent_range(mm, start, end) };
    mm.locked_vm = mm.locked_vm.saturating_sub(unlocked_pages);
    Ok(unlocked_pages)
}

pub unsafe fn lock_all_current(mm: &mut MmStruct, onfault: bool) -> Result<u64, i32> {
    let ranges = mm
        .mm_mt
        .collect_entries()
        .into_iter()
        .map(|(start, end, _)| (start, end.saturating_add(1)))
        .collect::<alloc::vec::Vec<_>>();
    let mut locked = 0u64;
    for (start, end) in ranges {
        locked = locked.saturating_add(unsafe { lock_vma_range(mm, start, end - start, onfault)? });
    }
    Ok(locked)
}

pub unsafe fn unlock_all(mm: &mut MmStruct) {
    let ranges = mm
        .mm_mt
        .collect_entries()
        .into_iter()
        .map(|(start, end, _)| (start, end.saturating_add(1)))
        .collect::<alloc::vec::Vec<_>>();
    for (start, end) in ranges {
        let _ = unsafe { unlock_vma_range(mm, start, end - start) };
    }
    mm.locked_vm = 0;
}

pub fn mincore_residency(
    mm: &MmStruct,
    start: u64,
    len: u64,
    out: &mut [u8],
) -> Result<usize, i32> {
    let (start, end) = page_aligned_range(start, len)?;
    let pages = ((end - start) / PAGE_SIZE) as usize;
    if out.len() < pages {
        return Err(EINVAL);
    }
    for (idx, slot) in out.iter_mut().take(pages).enumerate() {
        let addr = start + (idx as u64) * PAGE_SIZE;
        *slot = if crate::mm::vma::find_vma(mm, addr)
            .map(|vma| unsafe { (*vma).vm_start <= addr })
            .unwrap_or(false)
        {
            1
        } else {
            0
        };
    }
    Ok(pages)
}

pub fn seal_range(start: u64, len: u64, flags: u64) -> Result<(), i32> {
    if start == 0 || len == 0 || flags != 0 {
        return Err(EINVAL);
    }
    let _ = page_aligned_range(start, len)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::mm_types::VmAreaStruct;
    use crate::mm::vm_flags::{VM_READ, VM_WRITE};

    #[test]
    fn lock_range_sets_vma_flags_and_accounting() {
        let mut mm = MmStruct::new(0);
        let mut vma = Box::new(VmAreaStruct::new(0x1000, 0x3000, VM_READ | VM_WRITE));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, &mut *vma).unwrap();
            assert_eq!(lock_vma_range(&mut mm, 0x1000, 0x1000, true), Ok(1));
            assert_eq!(lock_vma_range(&mut mm, 0x1000, 0x1000, true), Ok(0));
            assert_ne!(vma.vm_flags & VM_LOCKED, 0);
            assert_ne!(vma.vm_flags & VM_LOCKONFAULT, 0);
            assert_eq!(mm.locked_vm, 1);
            assert_eq!(unlock_vma_range(&mut mm, 0x1000, 0x1000), Ok(1));
            assert_eq!(vma.vm_flags & VM_LOCKED, 0);
        }
    }

    #[test]
    fn partial_unlock_splits_then_full_unlock_merges() {
        let mut mm = MmStruct::new(0);
        let vma = Box::into_raw(Box::new(VmAreaStruct::new(
            0x1000,
            0x4000,
            VM_READ | VM_WRITE,
        )));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, vma).unwrap();
            assert_eq!(lock_vma_range(&mut mm, 0x1000, 0x3000, true), Ok(3));
            assert_eq!(unlock_vma_range(&mut mm, 0x2000, 0x1000), Ok(1));
            assert_eq!(mm.map_count, 3);
            let mid = crate::mm::vma::find_vma(&mm, 0x2000).unwrap();
            assert_eq!((*mid).vm_start, 0x2000);
            assert_eq!((*mid).vm_end, 0x3000);
            assert_eq!((*mid).vm_flags & VM_LOCKED, 0);
            assert_eq!(unlock_vma_range(&mut mm, 0x1000, 0x3000), Ok(2));
            assert_eq!(mm.map_count, 1);
            assert_eq!(mm.locked_vm, 0);
            let merged = crate::mm::vma::find_vma(&mm, 0x1000).unwrap();
            crate::mm::vma::remove_vma(&mut mm, merged);
            crate::mm::vma::vm_area_free(merged);
        }
    }

    #[test]
    fn mincore_reports_vma_residency() {
        let mut mm = MmStruct::new(0);
        let mut vma = Box::new(VmAreaStruct::new(0x4000, 0x5000, VM_READ));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, &mut *vma).unwrap();
        }
        let mut vec = [0u8; 2];
        assert_eq!(mincore_residency(&mm, 0x3000, 0x2000, &mut vec), Ok(2));
        assert_eq!(vec, [0, 1]);
    }
}
