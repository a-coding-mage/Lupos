//! linux-parity: partial
//! linux-source: vendor/linux/mm/highmem.c
//! test-origin: linux:vendor/linux/mm/highmem.c
//! Highmem page counters, kmap-local indices, and zeroing helpers.

use crate::include::uapi::errno::EINVAL;

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const KM_MAX_IDX: i32 = 16;
pub const KM_INCR: i32 = 1;
pub const LAST_PKMAP: usize = 1024;
pub const LAST_PKMAP_MASK: usize = LAST_PKMAP - 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HighmemZone {
    pub highmem: bool,
    pub free_pages: u64,
    pub managed_pages: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KmapCtrl {
    pub idx: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KunmapResult {
    pub count: u32,
    pub wake_waiters: bool,
}

pub fn nr_free_highpages(zones: &[HighmemZone]) -> u64 {
    zones
        .iter()
        .filter(|zone| zone.highmem)
        .map(|zone| zone.free_pages)
        .sum()
}

pub fn totalhigh_pages(zones: &[HighmemZone]) -> u64 {
    zones
        .iter()
        .filter(|zone| zone.highmem)
        .map(|zone| zone.managed_pages)
        .sum()
}

pub const fn kmap_local_calc_idx(idx: i32, cpu: i32) -> i32 {
    idx + KM_MAX_IDX * cpu
}

pub const fn kmap_local_idx_push(ctrl: &mut KmapCtrl) -> Result<i32, i32> {
    ctrl.idx += KM_INCR;
    if ctrl.idx >= KM_MAX_IDX {
        return Err(-EINVAL);
    }
    Ok(ctrl.idx - 1)
}

pub const fn kmap_local_idx(ctrl: KmapCtrl) -> i32 {
    ctrl.idx - 1
}

pub const fn kmap_local_idx_pop(ctrl: &mut KmapCtrl) -> Result<(), i32> {
    ctrl.idx -= KM_INCR;
    if ctrl.idx < 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn kmap_high_count_after_map(existing_count: u32) -> u32 {
    if existing_count == 0 {
        2
    } else {
        existing_count + 1
    }
}

pub const fn kunmap_high_count_after_unmap(
    existing_count: u32,
    waitqueue_active: bool,
) -> Result<KunmapResult, i32> {
    if existing_count <= 1 {
        return Err(-EINVAL);
    }
    let count = existing_count - 1;
    Ok(KunmapResult {
        count,
        wake_waiters: count == 1 && waitqueue_active,
    })
}

pub fn zero_user_segments(
    page: &mut [u8],
    mut start1: usize,
    mut end1: usize,
    mut start2: usize,
    mut end2: usize,
) -> Result<(), i32> {
    if end1 > page.len() || end2 > page.len() {
        return Err(-EINVAL);
    }
    if start1 >= end1 {
        start1 = 0;
        end1 = 0;
    }
    if start2 >= end2 {
        start2 = 0;
        end2 = 0;
    }
    if end1 > start1 {
        page[start1..end1].fill(0);
    }
    if end2 > start2 {
        page[start2..end2].fill(0);
    }
    Ok(())
}

pub const fn pkmap_nr(vaddr: usize, pkmap_base: usize) -> usize {
    ((vaddr - pkmap_base) >> PAGE_SHIFT) & LAST_PKMAP_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highmem_counts_and_kmap_indices_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/highmem.c"
        ));
        let x86_highmem = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/pgtable_32_areas.h"
        ));

        assert!(source.contains("return idx + KM_MAX_IDX * smp_processor_id();"));
        assert!(source.contains("unsigned long __nr_free_highpages(void)"));
        assert!(source.contains("unsigned long __totalhigh_pages(void)"));
        assert!(source.contains("if (is_highmem(zone))"));
        assert!(source.contains("Virtual_count is not a pure \"count\""));
        assert!(source.contains("pkmap_count[last_pkmap_nr] = 1;"));
        assert!(source.contains("pkmap_count[PKMAP_NR(vaddr)]++;"));
        assert!(source.contains("switch (--pkmap_count[nr])"));
        assert!(source.contains("void zero_user_segments(struct page *page"));
        assert!(source.contains("kaddr = kmap_local_page(page + i);"));
        assert!(source.contains("kunmap_local(kaddr);"));
        assert!(x86_highmem.contains("#define LAST_PKMAP 1024"));

        let zones = [
            HighmemZone {
                highmem: false,
                free_pages: 9,
                managed_pages: 10,
            },
            HighmemZone {
                highmem: true,
                free_pages: 3,
                managed_pages: 7,
            },
            HighmemZone {
                highmem: true,
                free_pages: 4,
                managed_pages: 11,
            },
        ];
        assert_eq!(nr_free_highpages(&zones), 7);
        assert_eq!(totalhigh_pages(&zones), 18);
        assert_eq!(kmap_local_calc_idx(2, 3), 50);
    }

    #[test]
    fn highmem_kmap_counts_and_zeroing_follow_linux_rules() {
        assert_eq!(kmap_high_count_after_map(0), 2);
        assert_eq!(kmap_high_count_after_map(2), 3);
        assert_eq!(
            kunmap_high_count_after_unmap(2, true),
            Ok(KunmapResult {
                count: 1,
                wake_waiters: true,
            })
        );
        assert_eq!(kunmap_high_count_after_unmap(1, false), Err(-EINVAL));

        let mut ctrl = KmapCtrl::default();
        assert_eq!(kmap_local_idx_push(&mut ctrl), Ok(0));
        assert_eq!(kmap_local_idx(ctrl), 0);
        assert_eq!(kmap_local_idx_pop(&mut ctrl), Ok(()));

        let mut bytes = [1_u8; PAGE_SIZE * 2];
        assert_eq!(
            zero_user_segments(&mut bytes, 2, 5, PAGE_SIZE + 1, PAGE_SIZE + 3),
            Ok(())
        );
        assert_eq!(&bytes[2..5], &[0, 0, 0]);
        assert_eq!(&bytes[PAGE_SIZE + 1..PAGE_SIZE + 3], &[0, 0]);
        assert_eq!(bytes[1], 1);
    }
}
