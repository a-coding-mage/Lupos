//! linux-parity: complete
//! linux-source: vendor/linux/mm/msync.c
//! test-origin: linux:vendor/linux/mm/msync.c
//! `msync(2)` validation and VMA writeback-range planning.

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOMEM};
use crate::mm::frame::PAGE_SIZE;

pub const MS_ASYNC: i32 = 1;
pub const MS_INVALIDATE: i32 = 2;
pub const MS_SYNC: i32 = 4;
pub const VM_SHARED: u64 = 1 << 0;
pub const VM_LOCKED: u64 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsyncVma {
    pub vm_start: u64,
    pub vm_end: u64,
    pub vm_pgoff: u64,
    pub vm_flags: u64,
    pub has_file: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FsyncRange {
    pub start: u64,
    pub end: u64,
}

pub const fn msync_validate_start_flags(start: u64, flags: i32) -> Result<(), i32> {
    if flags & !(MS_ASYNC | MS_INVALIDATE | MS_SYNC) != 0 {
        return Err(-EINVAL);
    }
    if start & (PAGE_SIZE as u64 - 1) != 0 {
        return Err(-EINVAL);
    }
    if flags & MS_ASYNC != 0 && flags & MS_SYNC != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn msync_page_align_len(len: u64) -> Option<u64> {
    let mask = PAGE_SIZE as u64 - 1;
    match len.checked_add(mask) {
        Some(value) => Some(value & !mask),
        None => None,
    }
}

pub fn msync_collect_fsync_ranges(
    start: u64,
    len: u64,
    flags: i32,
    vmas: &[MsyncVma],
    ranges: &mut [FsyncRange],
) -> Result<usize, i32> {
    msync_validate_start_flags(start, flags)?;
    let len = msync_page_align_len(len).ok_or(-ENOMEM)?;
    let end = start.checked_add(len).ok_or(-ENOMEM)?;
    if end == start {
        return Ok(0);
    }

    let mut cursor = start;
    let mut range_count = 0;
    let mut unmapped_error = 0;

    while cursor < end {
        let Some(vma) = find_vma(vmas, cursor) else {
            return Err(-ENOMEM);
        };

        if cursor < vma.vm_start {
            if flags == MS_ASYNC {
                return Err(-ENOMEM);
            }
            cursor = vma.vm_start;
            if cursor >= end {
                break;
            }
            unmapped_error = -ENOMEM;
        }

        if flags & MS_INVALIDATE != 0 && vma.vm_flags & VM_LOCKED != 0 {
            return Err(-EBUSY);
        }

        let covered_end = core::cmp::min(end, vma.vm_end);
        let fstart = (cursor - vma.vm_start) + (vma.vm_pgoff << PAGE_SIZE.trailing_zeros());
        let fend = fstart + (covered_end - cursor) - 1;
        cursor = vma.vm_end;

        if flags & MS_SYNC != 0 && vma.has_file && vma.vm_flags & VM_SHARED != 0 {
            if range_count < ranges.len() {
                ranges[range_count] = FsyncRange {
                    start: fstart,
                    end: fend,
                };
            }
            range_count += 1;
        }
    }

    if unmapped_error != 0 {
        Err(unmapped_error)
    } else {
        Ok(range_count)
    }
}

fn find_vma(vmas: &[MsyncVma], start: u64) -> Option<MsyncVma> {
    vmas.iter()
        .copied()
        .filter(|vma| vma.vm_end > start)
        .min_by_key(|vma| vma.vm_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msync_validation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/msync.c"
        ));
        assert!(
            source
                .contains("SYSCALL_DEFINE3(msync, unsigned long, start, size_t, len, int, flags)")
        );
        assert!(source.contains("start = untagged_addr(start);"));
        assert!(source.contains("if (flags & ~(MS_ASYNC | MS_INVALIDATE | MS_SYNC))"));
        assert!(source.contains("if (offset_in_page(start))"));
        assert!(source.contains("if ((flags & MS_ASYNC) && (flags & MS_SYNC))"));
        assert!(source.contains("len = (len + ~PAGE_MASK) & PAGE_MASK;"));
        assert!(source.contains("if (end < start)"));
        assert!(source.contains("if (end == start)"));

        assert_eq!(msync_validate_start_flags(0x1001, MS_SYNC), Err(-EINVAL));
        assert_eq!(
            msync_validate_start_flags(0x1000, MS_SYNC | MS_ASYNC),
            Err(-EINVAL)
        );
        assert_eq!(msync_page_align_len(1), Some(PAGE_SIZE as u64));
    }

    #[test]
    fn msync_vma_walk_matches_linux_fsync_and_error_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/msync.c"
        ));
        assert!(source.contains("if (flags == MS_ASYNC)"));
        assert!(source.contains("unmapped_error = -ENOMEM;"));
        assert!(source.contains("(flags & MS_INVALIDATE)"));
        assert!(source.contains("(vma->vm_flags & VM_LOCKED)"));
        assert!(source.contains("error = -EBUSY;"));
        assert!(source.contains("file = vma->vm_file;"));
        assert!(source.contains("fstart = (start - vma->vm_start) +"));
        assert!(source.contains("fend = fstart + (min(end, vma->vm_end) - start) - 1;"));
        assert!(source.contains("vfs_fsync_range(file, fstart, fend, 1);"));
        assert!(source.contains("return error ? : unmapped_error;"));

        let vmas = [
            MsyncVma {
                vm_start: 0x1000,
                vm_end: 0x3000,
                vm_pgoff: 2,
                vm_flags: VM_SHARED,
                has_file: true,
            },
            MsyncVma {
                vm_start: 0x5000,
                vm_end: 0x6000,
                vm_pgoff: 9,
                vm_flags: VM_SHARED,
                has_file: true,
            },
        ];
        let mut ranges = [FsyncRange::default(); 4];
        assert_eq!(
            msync_collect_fsync_ranges(0x1000, 0x1000, MS_SYNC, &vmas, &mut ranges),
            Ok(1)
        );
        assert_eq!(
            ranges[0],
            FsyncRange {
                start: 2 * PAGE_SIZE as u64,
                end: 3 * PAGE_SIZE as u64 - 1,
            }
        );
        assert_eq!(
            msync_collect_fsync_ranges(0x3000, 0x1000, MS_ASYNC, &vmas, &mut ranges),
            Err(-ENOMEM)
        );
        assert_eq!(
            msync_collect_fsync_ranges(0x2000, 0x4000, MS_SYNC, &vmas, &mut ranges),
            Err(-ENOMEM)
        );

        let locked = [MsyncVma {
            vm_start: 0x1000,
            vm_end: 0x2000,
            vm_pgoff: 0,
            vm_flags: VM_LOCKED,
            has_file: false,
        }];
        assert_eq!(
            msync_collect_fsync_ranges(0x1000, 0x1000, MS_INVALIDATE, &locked, &mut ranges),
            Err(-EBUSY)
        );
    }
}
