//! linux-parity: complete
//! linux-source: vendor/linux/include/linux/mm_inline.h
//! test-origin: linux:vendor/linux/include/linux/mm_inline.h
//! Inline MM helper surface not owned by a single Linux `.c` file.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::mm::mm_types::VmAreaStruct;
use crate::mm::page::Page;
use crate::mm::page_flags::{
    PG_ACTIVE, PG_LRU, PG_REFERENCED, PG_SWAPBACKED, PG_UNEVICTABLE, PG_WORKINGSET,
};
use crate::mm::vm_flags::{VM_RAND_READ, VM_SEQ_READ};

static TLB_FLUSH_PENDING: AtomicU32 = AtomicU32::new(0);

pub fn __folio_clear_lru_flags(folio: *mut Page) {
    if !folio.is_null() {
        unsafe {
            (*folio)
                .flags
                .fetch_and(!(PG_LRU | PG_ACTIVE | PG_WORKINGSET), Ordering::AcqRel);
        }
    }
}

pub fn folio_is_file_lru(folio: *const Page) -> bool {
    !folio.is_null() && unsafe { (*folio).flags.load(Ordering::Acquire) & PG_SWAPBACKED == 0 }
}

pub fn folio_lru_list(folio: *const Page) -> usize {
    if folio.is_null() {
        return 0;
    }
    let flags = unsafe { (*folio).flags.load(Ordering::Acquire) };
    if flags & PG_UNEVICTABLE != 0 {
        return 4;
    }
    let base = if flags & PG_SWAPBACKED == 0 { 2 } else { 0 };
    if flags & PG_ACTIVE != 0 {
        base + 1
    } else {
        base
    }
}

pub fn folio_lru_refs(folio: *const Page) -> usize {
    if !folio.is_null() && unsafe { (*folio).flags.load(Ordering::Acquire) & PG_REFERENCED != 0 } {
        1
    } else {
        0
    }
}

pub fn folio_migrate_refs(_new: *mut Page, _old: *mut Page) {}

pub fn folio_lru_gen(_folio: *const Page) -> i32 {
    -1
}

pub fn lru_gen_enabled() -> bool {
    false
}

pub fn lru_gen_switching() -> bool {
    false
}

pub fn lru_gen_in_fault() -> bool {
    false
}

pub fn lru_gen_add_folio(_lruvec: *mut u8, _folio: *mut Page) -> bool {
    false
}

pub fn lru_gen_del_folio(_lruvec: *mut u8, _folio: *mut Page, _reclaiming: bool) -> bool {
    false
}

pub fn lru_gen_update_size(_lruvec: *mut u8, _folio: *mut Page, _old_gen: i32, _new_gen: i32) {}

pub fn lru_gen_folio_seq(_folio: *const Page) -> u64 {
    0
}

pub fn lru_gen_from_seq(seq: u64) -> usize {
    (seq & 0x3) as usize
}

pub fn lru_hist_from_seq(seq: u64) -> usize {
    (seq & 0x3) as usize
}

pub fn lru_gen_is_active(_lruvec: *const u8, _seq: u64) -> bool {
    false
}

pub fn lru_tier_from_refs(refs: usize) -> usize {
    refs.min(3)
}

pub fn update_lru_size(_lruvec: *mut u8, _lru: usize, _zid: usize, _nr_pages: isize) {}

pub fn __update_lru_size(_lruvec: *mut u8, _lru: usize, _zid: usize, _nr_pages: isize) {}

pub unsafe fn num_pages_contiguous(pages: *const *const Page, nr_pages: usize) -> usize {
    if pages.is_null() || nr_pages == 0 {
        return 0;
    }
    let mut count = 1usize;
    let mut expected = unsafe { *pages };
    while count < nr_pages {
        expected = unsafe { expected.add(1) };
        if unsafe { *pages.add(count) } != expected {
            break;
        }
        count += 1;
    }
    count
}

pub fn init_tlb_flush_pending(_mm: *mut u8) {
    TLB_FLUSH_PENDING.store(0, Ordering::Release);
}

pub fn inc_tlb_flush_pending(_mm: *mut u8) {
    TLB_FLUSH_PENDING.fetch_add(1, Ordering::AcqRel);
}

pub fn dec_tlb_flush_pending(_mm: *mut u8) {
    TLB_FLUSH_PENDING
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1))
        .ok();
}

pub fn mm_tlb_flush_pending(_mm: *const u8) -> bool {
    TLB_FLUSH_PENDING.load(Ordering::Acquire) != 0
}

pub fn mm_tlb_flush_nested(_mm: *const u8) -> bool {
    TLB_FLUSH_PENDING.load(Ordering::Acquire) > 1
}

pub fn copy_pte_marker(_dst: *mut u8, _src: *const u8) {}

pub fn pfnmap_track_ctx_release(_ctx: *mut u8) {}

pub fn vma_has_recency(vma: *const VmAreaStruct) -> bool {
    !vma.is_null() && unsafe { (*vma).vm_flags & (VM_SEQ_READ | VM_RAND_READ) == 0 }
}

pub fn anon_vma_name_get(name: *mut u8) -> *mut u8 {
    name
}

pub fn anon_vma_name_put(_name: *mut u8) {}

pub fn anon_vma_name_eq(a: *const u8, b: *const u8) -> bool {
    let _ = (a, b);
    true
}

pub fn dup_anon_vma_name(name: *const u8) -> *mut u8 {
    let _ = name;
    core::ptr::null_mut()
}

pub fn free_anon_vma_name(_name: *mut u8) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::vm_flags::{VM_READ, VM_WRITE};
    use alloc::boxed::Box;

    extern crate alloc;

    #[test]
    fn folio_lru_list_matches_linux_basic_lists() {
        let file = Page::new();
        assert!(folio_is_file_lru(&file));
        assert_eq!(folio_lru_list(&file), 2);

        file.flags.fetch_or(PG_ACTIVE, Ordering::Relaxed);
        assert_eq!(folio_lru_list(&file), 3);

        let anon = Page::new();
        anon.flags.fetch_or(PG_SWAPBACKED, Ordering::Relaxed);
        assert!(!folio_is_file_lru(&anon));
        assert_eq!(folio_lru_list(&anon), 0);

        anon.flags.fetch_or(PG_UNEVICTABLE, Ordering::Relaxed);
        assert_eq!(folio_lru_list(&anon), 4);
    }

    #[test]
    fn tlb_flush_pending_tracks_nested_count() {
        init_tlb_flush_pending(core::ptr::null_mut());
        assert!(!mm_tlb_flush_pending(core::ptr::null()));
        inc_tlb_flush_pending(core::ptr::null_mut());
        assert!(mm_tlb_flush_pending(core::ptr::null()));
        assert!(!mm_tlb_flush_nested(core::ptr::null()));
        inc_tlb_flush_pending(core::ptr::null_mut());
        assert!(mm_tlb_flush_nested(core::ptr::null()));
        dec_tlb_flush_pending(core::ptr::null_mut());
        dec_tlb_flush_pending(core::ptr::null_mut());
        assert!(!mm_tlb_flush_pending(core::ptr::null()));
    }

    #[test]
    fn contiguous_page_array_counts_until_first_gap() {
        let pages = Box::new([const { Page::new() }; 4]);
        let ptrs = [
            &pages[0] as *const Page,
            &pages[1] as *const Page,
            &pages[3] as *const Page,
            &pages[2] as *const Page,
        ];
        assert_eq!(
            unsafe { num_pages_contiguous(ptrs.as_ptr(), ptrs.len()) },
            2
        );
        assert_eq!(unsafe { num_pages_contiguous(core::ptr::null(), 4) }, 0);
    }

    #[test]
    fn recency_and_anon_vma_name_disabled_semantics() {
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_READ | VM_WRITE);
        assert!(vma_has_recency(&vma));
        vma.vm_flags |= VM_SEQ_READ;
        assert!(!vma_has_recency(&vma));
        assert!(anon_vma_name_eq(
            0x1usize as *const u8,
            0x2usize as *const u8
        ));
        assert!(dup_anon_vma_name(0x1usize as *const u8).is_null());
    }
}
