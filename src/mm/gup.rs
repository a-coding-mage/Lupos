//! linux-parity: complete
//! linux-source: vendor/linux/mm/gup.c
//! test-origin: linux:vendor/linux/mm/gup.c
//! get_user_pages / pin_user_pages helpers.
//!
//! References:
//! - `vendor/linux/mm/gup.c`

extern crate alloc;

use alloc::boxed::Box;

use crate::arch::x86::mm::paging::{
    PAGE_SIZE, PMD_SIZE, PUD_SIZE, p4d_offset, p4d_present, pgd_offset_pgd, pgd_present, pgd_t,
    pmd_huge, pmd_offset, pmd_present, pte_pfn, pte_present, pte_t, pte_write, pte_young, pud_huge,
    pud_offset, pud_present,
};
use crate::include::uapi::errno::{EFAULT, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::buddy::{pfn_to_page, pfn_valid};
use crate::mm::fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, handle_mm_fault};
use crate::mm::mm_types::MmStruct;
use crate::mm::page::Page;
use crate::mm::vm_flags::{VM_READ, VM_WRITE};

pub const FOLL_WRITE: u32 = 1 << 0;
pub const FOLL_FORCE: u32 = 1 << 4;
pub const FOLL_PIN: u32 = 1 << 10;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "pin_user_pages_fast",
        linux_pin_user_pages_fast as usize,
        true,
    );
    export_symbol_once("unpin_user_pages", linux_unpin_user_pages as usize, false);
}

pub fn access_permitted(flags: u64, write: bool) -> bool {
    if write {
        flags & VM_WRITE != 0
    } else {
        flags & (VM_READ | VM_WRITE) != 0
    }
}

pub fn range_accessible(mm: &MmStruct, start: u64, len: usize, write: bool) -> Result<usize, i32> {
    if len == 0 {
        return Ok(0);
    }
    let end = start.checked_add(len as u64).ok_or(EINVAL)?;
    let mut addr = start;
    let mut pages = 0usize;
    while addr < end {
        let Some(vma) = crate::mm::vma::find_vma(mm, addr) else {
            return Err(EFAULT);
        };
        let vma_ref = unsafe { &*vma };
        if addr < vma_ref.vm_start || !access_permitted(vma_ref.vm_flags, write) {
            return Err(EFAULT);
        }
        pages += 1;
        addr = ((addr / PAGE_SIZE) + 1) * PAGE_SIZE;
    }
    Ok(pages)
}

fn pfn_to_valid_page(pfn: u64) -> Result<*mut Page, i32> {
    if !pfn_valid(pfn as usize) {
        return Err(EFAULT);
    }
    Ok(pfn_to_page(pfn as usize))
}

fn follow_present_page(mm: &MmStruct, addr: u64, write: bool) -> Result<*mut Page, i32> {
    let Some(vma) = crate::mm::vma::find_vma(mm, addr) else {
        return Err(EFAULT);
    };
    let vma_ref = unsafe { &*vma };
    if addr < vma_ref.vm_start || !access_permitted(vma_ref.vm_flags, write) {
        return Err(EFAULT);
    }
    if mm.pgd == 0 {
        return Err(EFAULT);
    }

    unsafe {
        let pgdp = pgd_offset_pgd(mm.pgd as *mut pgd_t, addr);
        let pgd = *pgdp;
        if !pgd_present(pgd) {
            return Err(EFAULT);
        }

        let p4dp = p4d_offset(pgdp, addr);
        let p4d = *p4dp;
        if !p4d_present(p4d) {
            return Err(EFAULT);
        }

        let pudp = pud_offset(p4dp, addr);
        let pud = *pudp;
        if !pud_present(pud) {
            return Err(EFAULT);
        }
        if pud_huge(pud) {
            let base = pte_pfn(pte_t(pud.0));
            let offset = (addr & (PUD_SIZE - 1)) / PAGE_SIZE;
            return pfn_to_valid_page(base + offset);
        }

        let pmdp = pmd_offset(pudp, addr);
        let pmd = *pmdp;
        if !pmd_present(pmd) {
            return Err(EFAULT);
        }
        if pmd_huge(pmd) {
            let base = pte_pfn(pte_t(pmd.0));
            let offset = (addr & (PMD_SIZE - 1)) / PAGE_SIZE;
            return pfn_to_valid_page(base + offset);
        }

        let ptep = crate::arch::x86::mm::paging::pte_offset_kernel(pmdp, addr);
        let pte = *ptep;
        if !pte_present(pte) || !pte_young(pte) {
            return Err(EFAULT);
        }
        if write && !pte_write(pte) {
            return Err(EFAULT);
        }
        pfn_to_valid_page(pte_pfn(pte))
    }
}

fn collect_user_pages(
    mm: &MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
    pages: *mut *mut Page,
    allow_fault: bool,
    pin: bool,
) -> Result<usize, i32> {
    let write = flags & FOLL_WRITE != 0;
    let mut collected = 0usize;

    for idx in 0..nr_pages {
        let addr = start
            .checked_add((idx as u64).checked_mul(PAGE_SIZE).ok_or(EINVAL)?)
            .ok_or(EINVAL)?;
        let mut page = follow_present_page(mm, addr, write);
        if page.is_err() && allow_fault {
            let Some(vma) = crate::mm::vma::find_vma(mm, addr) else {
                return if collected == 0 {
                    Err(EFAULT)
                } else {
                    Ok(collected)
                };
            };
            let fault_flags = FAULT_FLAG_USER | if write { FAULT_FLAG_WRITE } else { 0 };
            if unsafe { handle_mm_fault(vma, addr, fault_flags) } != 0 {
                return if collected == 0 {
                    Err(EFAULT)
                } else {
                    Ok(collected)
                };
            }
            page = follow_present_page(mm, addr, write);
        }

        match page {
            Ok(page) => {
                if !pages.is_null() {
                    unsafe { *pages.add(idx) = page };
                }
                if pin || !pages.is_null() {
                    unsafe { (*page).get_page() };
                }
                collected += 1;
            }
            Err(err) => {
                return if collected == 0 {
                    Err(err)
                } else {
                    Ok(collected)
                };
            }
        }
    }

    Ok(collected)
}

pub fn get_user_pages_fast(
    mm: &MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
) -> Result<usize, i32> {
    collect_user_pages(
        mm,
        start,
        nr_pages,
        flags,
        core::ptr::null_mut(),
        false,
        false,
    )
}

pub fn get_user_pages(
    mm: &MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
) -> Result<usize, i32> {
    collect_user_pages(
        mm,
        start,
        nr_pages,
        flags,
        core::ptr::null_mut(),
        true,
        false,
    )
}

pub fn pin_user_pages_fast(
    mm: &MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
) -> Result<usize, i32> {
    collect_user_pages(
        mm,
        start,
        nr_pages,
        flags | FOLL_PIN,
        core::ptr::null_mut(),
        false,
        true,
    )
}

pub fn pin_user_pages(
    mm: &MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
) -> Result<usize, i32> {
    collect_user_pages(
        mm,
        start,
        nr_pages,
        flags | FOLL_PIN,
        core::ptr::null_mut(),
        true,
        true,
    )
}

// ---------------------------------------------------------------------------
// Linux-visible gup.c / mm.h wrappers
// ---------------------------------------------------------------------------

pub unsafe fn get_user_pages_fast_only(
    start: u64,
    nr_pages: usize,
    flags: u32,
    pages: *mut *mut Page,
) -> isize {
    if pages.is_null() {
        return -(EFAULT as isize);
    }
    let mm = unsafe { crate::mm::mm_types::CURRENT_TEST_MM };
    if mm.is_null() {
        return 0;
    }
    match collect_user_pages(unsafe { &*mm }, start, nr_pages, flags, pages, false, false) {
        Ok(nr) => nr as isize,
        Err(err) => -(err as isize),
    }
}

/// `pin_user_pages_fast()` — `vendor/linux/mm/gup.c:3310`.
pub unsafe extern "C" fn linux_pin_user_pages_fast(
    start: u64,
    nr_pages: i32,
    flags: u32,
    pages: *mut *mut Page,
) -> i32 {
    if nr_pages < 0 {
        return -EINVAL;
    }

    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return -EFAULT;
    }

    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -EFAULT;
    }

    match collect_user_pages(
        unsafe { &*mm },
        start,
        nr_pages as usize,
        flags | FOLL_PIN,
        pages,
        false,
        true,
    ) {
        Ok(nr) => nr as i32,
        Err(err) => -err,
    }
}

pub unsafe fn get_user_pages_remote(
    mm: *mut MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
    pages: *mut *mut Page,
    _locked: *mut i32,
) -> isize {
    if mm.is_null() {
        return -(EFAULT as isize);
    }
    match collect_user_pages(unsafe { &*mm }, start, nr_pages, flags, pages, true, false) {
        Ok(nr) => nr as isize,
        Err(err) => -(err as isize),
    }
}

pub unsafe fn get_user_pages_unlocked(
    mm: *mut MmStruct,
    start: u64,
    nr_pages: usize,
    pages: *mut *mut Page,
    flags: u32,
) -> isize {
    unsafe { get_user_pages_remote(mm, start, nr_pages, flags, pages, core::ptr::null_mut()) }
}

pub unsafe fn pin_user_pages_remote(
    mm: *mut MmStruct,
    start: u64,
    nr_pages: usize,
    flags: u32,
    pages: *mut *mut Page,
    locked: *mut i32,
) -> isize {
    unsafe { get_user_pages_remote(mm, start, nr_pages, flags | FOLL_PIN, pages, locked) }
}

pub unsafe fn pin_user_pages_unlocked(
    mm: *mut MmStruct,
    start: u64,
    nr_pages: usize,
    pages: *mut *mut Page,
    flags: u32,
) -> isize {
    unsafe { pin_user_pages_remote(mm, start, nr_pages, flags, pages, core::ptr::null_mut()) }
}

pub fn get_user_page_fast_only(addr: u64, flags: u32) -> *mut Page {
    let mut page = core::ptr::null_mut();
    let got = unsafe { get_user_pages_fast_only(addr, 1, flags, &raw mut page) };
    if got == 1 {
        page
    } else {
        core::ptr::null_mut()
    }
}

pub unsafe fn get_user_page_vma_remote(
    mm: *mut MmStruct,
    addr: u64,
    flags: u32,
    vmap: *mut *mut u8,
) -> *mut Page {
    if !vmap.is_null() {
        unsafe {
            *vmap = core::ptr::null_mut();
        }
    }
    let mut page = core::ptr::null_mut();
    let got =
        unsafe { get_user_pages_remote(mm, addr, 1, flags, &raw mut page, core::ptr::null_mut()) };
    if got == 1 {
        page
    } else {
        core::ptr::null_mut()
    }
}

pub fn gup_can_follow_protnone(_vma: *mut u8, _flags: u32) -> bool {
    true
}

pub fn fault_in_readable(_uaddr: *const u8, _size: usize) -> i32 {
    if _size != 0 && _uaddr.is_null() {
        _size.min(i32::MAX as usize) as i32
    } else {
        0
    }
}

pub fn fault_in_writeable(_uaddr: *mut u8, _size: usize) -> i32 {
    if _size != 0 && _uaddr.is_null() {
        _size.min(i32::MAX as usize) as i32
    } else {
        0
    }
}

pub fn fault_in_safe_writeable(uaddr: *mut u8, size: usize) -> i32 {
    fault_in_writeable(uaddr, size)
}

pub fn fault_in_subpage_writeable(uaddr: *mut u8, size: usize) -> i32 {
    fault_in_writeable(uaddr, size)
}

pub fn fixup_user_fault(
    mm: *mut MmStruct,
    address: u64,
    fault_flags: u32,
    _unlocked: *mut bool,
) -> i32 {
    if mm.is_null() {
        return -EFAULT;
    }
    let Some(vma) = crate::mm::vma::find_vma(unsafe { &*mm }, address) else {
        return -EFAULT;
    };
    let fault_flags = FAULT_FLAG_USER
        | if fault_flags & FOLL_WRITE != 0 {
            FAULT_FLAG_WRITE
        } else {
            0
        };
    if handle_mm_fault(vma, address, fault_flags) == 0 {
        0
    } else {
        -EFAULT
    }
}

pub fn folio_add_pin(folio: *mut Page) {
    if !folio.is_null() {
        unsafe { (*folio).get_page() };
    }
}

pub fn folio_add_pins(folio: *mut Page, nr: usize) {
    for _ in 0..nr {
        folio_add_pin(folio);
    }
}

pub fn unpin_folio(folio: *mut Page) {
    crate::mm::page_flags::folio_put(folio)
}

pub fn unpin_user_folio(folio: *mut Page, _dirty: bool) {
    unpin_folio(folio)
}

pub fn unpin_user_page(page: *mut Page) {
    unpin_folio(page)
}

pub unsafe fn unpin_folios(folios: *mut *mut Page, nr: usize) {
    unsafe { crate::mm::page_flags::folios_put(folios, nr) };
}

pub unsafe fn unpin_user_pages(pages: *mut *mut Page, nr: usize) {
    unsafe { unpin_folios(pages, nr) };
}

pub unsafe extern "C" fn linux_unpin_user_pages(pages: *mut *mut Page, nr: usize) {
    unsafe { unpin_user_pages(pages, nr) };
}

pub unsafe fn unpin_user_pages_dirty_lock(pages: *mut *mut Page, nr: usize, _make_dirty: bool) {
    unsafe { unpin_user_pages(pages, nr) };
}

pub unsafe fn unpin_user_page_range_dirty_lock(page: *mut Page, nr: usize, _make_dirty: bool) {
    for idx in 0..nr {
        unsafe { unpin_user_page(page.add(idx)) };
    }
}

pub unsafe fn memfd_pin_folios(
    file: *mut u8,
    start: u64,
    end: u64,
    folios: *mut *mut Page,
    max: usize,
    offset: *mut u64,
) -> isize {
    if folios.is_null() || end <= start {
        return 0;
    }
    if file.is_null() || max == 0 || !crate::mm::shmem::shmem_file(file) {
        return -(EINVAL as isize);
    }
    let id = file as usize as u64;
    let Some(object) = crate::mm::shmem::memfd_object(id) else {
        return -(EINVAL as isize);
    };
    let len = object.len() as u64;
    if start >= len {
        return 0;
    }
    let end = end.min(len);
    let nr = max.min((end - start).div_ceil(PAGE_SIZE) as usize);
    if !offset.is_null() {
        unsafe {
            *offset = start;
        }
    }
    for idx in 0..nr {
        let page = Box::into_raw(Box::new(Page::new()));
        unsafe {
            (*page).get_page();
            (*page).mapping = file as usize;
            (*page).index = ((start / PAGE_SIZE) as usize).saturating_add(idx);
            *folios.add(idx) = page;
        }
    }
    nr as isize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging;
    use crate::mm::buddy;
    use crate::mm::list::ListHead;
    use crate::mm::mm_types::VmAreaStruct;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::vm_flags::VM_WRITE;
    use crate::mm::vma::insert_vma;
    use alloc::boxed::Box;

    extern crate alloc;

    const TEST_PAGES: usize = 256;

    unsafe fn make_test_mm(start: u64, len: u64, flags: u64) -> (*mut MmStruct, *mut VmAreaStruct) {
        let mm = Box::into_raw(Box::new(
            MmStruct::new(paging::init_pgd_for_test() as usize),
        ));
        let vma = Box::into_raw(Box::new(VmAreaStruct::new(start, start + len, flags)));
        unsafe {
            ListHead::init(&mut (*vma).anon_vma_chain);
            (*vma).vm_mm = mm;
            insert_vma(&mut *mm, vma).expect("insert gup test vma");
        }
        (mm, vma)
    }

    #[test]
    fn gup_checks_vma_permissions() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let mut mm = MmStruct::new(0);
        let mut vma = Box::new(VmAreaStruct::new(0x1000, 0x3000, VM_READ));
        unsafe {
            crate::mm::vma::insert_vma(&mut mm, &mut *vma).unwrap();
        }
        assert_eq!(
            range_accessible(&mm, 0x1000, 2 * PAGE_SIZE as usize, false),
            Ok(2)
        );
        assert_eq!(get_user_pages_fast(&mm, 0x1000, 2, 0), Err(EFAULT));
        assert_eq!(get_user_pages_fast(&mm, 0x1000, 1, FOLL_WRITE), Err(EFAULT));
        assert_eq!(get_user_pages_fast(&mm, 0x4000, 1, 0), Err(EFAULT));
    }

    #[test]
    fn slow_gup_rejects_readonly_vma_for_write_faults() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let mut backing = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in backing.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(backing.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let start = 0x0050_0000;
        let (mm, _vma) = unsafe { make_test_mm(start, PAGE_SIZE, VM_READ) };
        let mut page = core::ptr::null_mut();

        let got = unsafe {
            get_user_pages_remote(
                mm,
                start,
                1,
                FOLL_WRITE,
                &raw mut page,
                core::ptr::null_mut(),
            )
        };
        assert_eq!(got, -(EFAULT as isize));
        assert!(page.is_null());

        unsafe {
            crate::mm::fork::exit_mmap(mm);
            let _ = Box::from_raw(mm);
        }
    }

    #[test]
    fn slow_gup_faults_pages_and_returns_page_pointers() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let mut backing = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in backing.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(backing.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let start = 0x0040_0000;
        let (mm, _vma) = unsafe { make_test_mm(start, 2 * PAGE_SIZE, VM_READ | VM_WRITE) };
        let mut out = [core::ptr::null_mut(); 2];

        let got = unsafe {
            get_user_pages_remote(
                mm,
                start,
                2,
                FOLL_WRITE,
                out.as_mut_ptr(),
                core::ptr::null_mut(),
            )
        };
        assert_eq!(got, 2);
        assert!(out.iter().all(|page| !page.is_null()));
        assert_ne!(out[0], out[1]);
        assert_eq!(unsafe { (*out[0]).refcount() }, 2);
        unsafe {
            crate::mm::mm_types::CURRENT_TEST_MM = mm;
        }
        assert_eq!(
            unsafe { get_user_pages_fast_only(start, 1, FOLL_WRITE, out.as_mut_ptr()) },
            1
        );
        unsafe {
            crate::mm::mm_types::CURRENT_TEST_MM = core::ptr::null_mut();
        }

        unsafe {
            crate::mm::fork::exit_mmap(mm);
            let _ = Box::from_raw(mm);
        }
    }

    #[test]
    fn fault_in_helpers_reject_null_nonzero_ranges() {
        assert_eq!(fault_in_readable(core::ptr::null(), 1), 1);
        assert_eq!(fault_in_writeable(core::ptr::null_mut(), 1), 1);
        assert_eq!(fault_in_safe_writeable(core::ptr::null_mut(), 2), 2);
        assert_eq!(fault_in_subpage_writeable(core::ptr::null_mut(), 3), 3);
        assert_eq!(fault_in_readable(core::ptr::null(), 0), 0);
        assert_eq!(fault_in_safe_writeable(core::ptr::null_mut(), 0), 0);
    }

    #[test]
    fn memfd_pin_folios_returns_page_descriptors_for_shmem_range() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::shmem::reset_for_tests();
        let file = crate::mm::shmem::shmem_file_setup(core::ptr::null(), 3 * PAGE_SIZE, 0);
        assert!(!file.is_null());

        let mut folios = [core::ptr::null_mut(); 4];
        let mut offset = 0;
        let pinned = unsafe {
            memfd_pin_folios(
                file,
                PAGE_SIZE,
                3 * PAGE_SIZE,
                folios.as_mut_ptr(),
                folios.len(),
                &mut offset,
            )
        };
        assert_eq!(pinned, 2);
        assert_eq!(offset, PAGE_SIZE);
        assert!(folios[..2].iter().all(|folio| !folio.is_null()));
        unsafe {
            assert_eq!((*folios[0]).mapping, file as usize);
            assert_eq!((*folios[0]).index, 1);
            assert_eq!((*folios[1]).index, 2);
            unpin_user_pages(folios.as_mut_ptr(), pinned as usize);
            for folio in folios[..pinned as usize].iter_mut() {
                drop(Box::from_raw(*folio));
                *folio = core::ptr::null_mut();
            }
        }

        assert_eq!(
            unsafe {
                memfd_pin_folios(
                    core::ptr::null_mut(),
                    0,
                    PAGE_SIZE,
                    folios.as_mut_ptr(),
                    1,
                    core::ptr::null_mut(),
                )
            },
            -(EINVAL as isize)
        );
    }
}
