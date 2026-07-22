//! linux-parity: partial
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Platform-independent page-fault handler — demand paging state machine.
///
/// This module implements the generic (architecture-independent) half of the
/// Linux page-fault path.  The architecture layer (`arch/x86/fault.rs`)
/// decodes the CPU error code and performs the VMA lookup; it then calls
/// [`handle_mm_fault`] here, which walks/allocates page-table levels and
/// dispatches to the appropriate leaf handler.
///
/// | Lupos function        | Linux equivalent           | Source              |
/// |-----------------------|----------------------------|---------------------|
/// | `handle_mm_fault`     | `handle_mm_fault()`        | `mm/memory.c:6589`  |
/// | `__handle_mm_fault`   | `__handle_mm_fault()`      | `mm/memory.c:6355`  |
/// | `handle_pte_fault`    | `handle_pte_fault()`       | `mm/memory.c:6273`  |
/// | `do_pte_missing`      | `do_pte_missing()`         | `mm/memory.c`       |
/// | `do_anonymous_page`   | `do_anonymous_page()`      | `mm/memory.c:5217`  |
/// | `do_wp_page`          | `do_wp_page()`             | `mm/memory.c:4149`  |
/// | `do_fault`            | `do_fault()`               | `mm/memory.c:5903`  |
///
/// ## References
///
/// - Linux `mm/memory.c` — primary reference for all functions here
/// - Linux `include/linux/mm_types.h` — `vm_fault_t`, `enum vm_fault_reason`
/// - Linux `include/linux/mm.h` — `struct vm_fault`, `FAULT_FLAG_*`
extern crate alloc;

use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::Ordering;

use crate::arch::x86::mm::paging::{
    self, __pte, _PAGE_ACCESSED, _PAGE_NX, _PAGE_PRESENT, _PAGE_TABLE, _PAGE_USER, PAGE_MASK,
    PAGE_SHIFT, PAGE_SIZE, flush_tlb_page, flush_tlb_range, p4d_offset, pfn_pte, pfn_to_virt,
    pgd_offset_pgd, pgd_t, pgprot_t, pmd_alloc, pmd_huge, pmd_none, pmd_offset, pmd_t, pte_alloc,
    pte_mkclean, pte_mkdirty, pte_mkold, pte_mkspecial, pte_mkwrite, pte_mkyoung, pte_none,
    pte_offset_kernel, pte_pfn, pte_present, pte_special, pte_t, pte_write, pte_wrprotect,
    ptep_get, ptep_get_and_clear, pud_alloc, pud_huge, pud_none, pud_offset, pud_t, set_pte_at,
};
use crate::mm::address_space::{AS_SHARED_ANON, AddressSpace, wait_on_page_writeback};
use crate::mm::buddy::{page_to_pfn, pfn_to_page, pfn_valid, with_global_buddy};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::page::Page;
use crate::mm::page_flags::{GFP_KERNEL, GfpFlags, PG_SWAPBACKED};
use crate::mm::rmap::anon_vma_prepare;
use crate::mm::vm_flags::{
    VM_DONTDUMP, VM_DONTEXPAND, VM_IO, VM_MAYSHARE, VM_MAYWRITE, VM_MIXEDMAP, VM_PFNMAP, VM_SHARED,
    VM_WRITE, VmFlags,
};

#[cfg(test)]
unsafe fn fault_page_kaddr(page: *mut Page) -> *mut u8 {
    if page.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if (*page).private == 0 {
            let buf: Box<[u8; PAGE_SIZE as usize]> = Box::new([0; PAGE_SIZE as usize]);
            (*page).private = Box::into_raw(buf) as usize;
        }
        (*page).private as *mut u8
    }
}

#[cfg(not(test))]
#[inline]
unsafe fn fault_page_kaddr(page: *mut Page) -> *mut u8 {
    if page.is_null() {
        return ptr::null_mut();
    }
    unsafe { pfn_to_virt(page_to_pfn(page)) }
}

unsafe fn free_unmapped_fault_page(page: *mut Page) {
    if page.is_null() {
        return;
    }
    #[cfg(test)]
    unsafe {
        if (*page).private != 0 {
            drop(Box::from_raw(
                (*page).private as *mut [u8; PAGE_SIZE as usize],
            ));
            (*page).private = 0;
        }
    }
    unsafe {
        with_global_buddy(|b| b.free_pages(page, 0));
    }
}

// ---------------------------------------------------------------------------
// VM_FAULT_* return codes — `vm_fault_t`
//
// Bitflags returned by the fault handler chain to signal the outcome.
// Values match Linux `enum vm_fault_reason` exactly.
//
// Ref: Linux `include/linux/mm_types.h` lines 1619-1631
// ---------------------------------------------------------------------------

/// Fault handler return type — a bitmask of `VM_FAULT_*` constants.
///
/// Ref: Linux `vm_fault_t` — `include/linux/mm_types.h`
pub type VmFaultFlags = u32;

pub const VM_FAULT_OOM: VmFaultFlags = 0x0001;
pub const VM_FAULT_SIGBUS: VmFaultFlags = 0x0002;
pub const VM_FAULT_MAJOR: VmFaultFlags = 0x0004;
pub const VM_FAULT_HWPOISON: VmFaultFlags = 0x0010;
pub const VM_FAULT_HWPOISON_LARGE: VmFaultFlags = 0x0020;
pub const VM_FAULT_SIGSEGV: VmFaultFlags = 0x0040;
pub const VM_FAULT_NOPAGE: VmFaultFlags = 0x0100;
pub const VM_FAULT_LOCKED: VmFaultFlags = 0x0200;
pub const VM_FAULT_RETRY: VmFaultFlags = 0x0400;
pub const VM_FAULT_FALLBACK: VmFaultFlags = 0x0800;
pub const VM_FAULT_DONE_COW: VmFaultFlags = 0x1000;
pub const VM_FAULT_NEEDDSYNC: VmFaultFlags = 0x2000;
pub const VM_FAULT_COMPLETED: VmFaultFlags = 0x4000;

/// Composite mask of all fatal error conditions.
///
/// Ref: Linux `VM_FAULT_ERROR` — `include/linux/mm_types.h`
pub const VM_FAULT_ERROR: VmFaultFlags = VM_FAULT_OOM
    | VM_FAULT_SIGBUS
    | VM_FAULT_SIGSEGV
    | VM_FAULT_HWPOISON
    | VM_FAULT_HWPOISON_LARGE
    | VM_FAULT_FALLBACK;

// ---------------------------------------------------------------------------
// FAULT_FLAG_* input flags
//
// Passed into the fault handler to describe the nature of the access.
// Values match Linux `enum fault_flag` exactly.
//
// Ref: Linux `include/linux/mm_types.h` lines 1736-1748
// ---------------------------------------------------------------------------

/// Fault-handler input flag type.
///
/// Ref: Linux `unsigned int flags` in `do_user_addr_fault` / `handle_mm_fault`
pub type FaultFlags = u32;

pub const FAULT_FLAG_WRITE: FaultFlags = 1 << 0;
pub const FAULT_FLAG_MKWRITE: FaultFlags = 1 << 1;
pub const FAULT_FLAG_ALLOW_RETRY: FaultFlags = 1 << 2;
pub const FAULT_FLAG_RETRY_NOWAIT: FaultFlags = 1 << 3;
pub const FAULT_FLAG_KILLABLE: FaultFlags = 1 << 4;
pub const FAULT_FLAG_TRIED: FaultFlags = 1 << 5;
pub const FAULT_FLAG_USER: FaultFlags = 1 << 6;
pub const FAULT_FLAG_REMOTE: FaultFlags = 1 << 7;
pub const FAULT_FLAG_INSTRUCTION: FaultFlags = 1 << 8;
pub const FAULT_FLAG_INTERRUPTIBLE: FaultFlags = 1 << 9;
pub const FAULT_FLAG_UNSHARE: FaultFlags = 1 << 10;
pub const FAULT_FLAG_ORIG_PTE_VALID: FaultFlags = 1 << 11;
pub const FAULT_FLAG_VMA_LOCK: FaultFlags = 1 << 12;

/// Default flags for a first-attempt fault.
///
/// Ref: Linux `FAULT_FLAG_DEFAULT` — `include/linux/mm_types.h`
pub const FAULT_FLAG_DEFAULT: FaultFlags =
    FAULT_FLAG_ALLOW_RETRY | FAULT_FLAG_KILLABLE | FAULT_FLAG_INTERRUPTIBLE;

// ---------------------------------------------------------------------------
// VmFault — per-fault state
//
// Carries the faulting address, VMA, page-table pointers, and the original
// PTE value through the handler chain.
//
// Ref: Linux `struct vm_fault` — `include/linux/mm.h:698`
// ---------------------------------------------------------------------------

/// Per-fault state passed through the handler chain.
///
/// All pointer fields are raw pointers to match the Linux C layout.  The
/// caller guarantees that pointed-to objects live for the duration of the
/// fault (held by mmap_lock — structurally assumed in M12).
#[repr(C)]
pub struct VmFault {
    /// Target VMA.
    pub vma: *mut VmAreaStruct,
    /// GFP mask for page allocations during this fault.
    pub gfp_mask: GfpFlags,
    /// Logical page offset within the VMA (for file-backed faults).
    pub pgoff: u64,
    /// Faulting virtual address, page-aligned.
    pub address: u64,
    /// Faulting virtual address, unmasked.
    pub real_address: u64,
    /// `FAULT_FLAG_*` input flags.
    pub flags: FaultFlags,
    /// Pointer to the PUD entry covering the faulting address.
    pub pud: *mut pud_t,
    /// Pointer to the PMD entry covering the faulting address.
    pub pmd: *mut pmd_t,
    /// Snapshot of the PTE at fault time.
    pub orig_pte: pte_t,
    /// Pointer to the live PTE entry (null if table not yet allocated).
    pub pte: *mut pte_t,
    /// Page allocated or returned by the handler.
    pub page: *mut Page,
    /// Pre-allocated COW page (M14 placeholder).
    pub cow_page: *mut Page,
}

// Safety: VmFault contains raw pointers but is only used within the fault
// handler, under mmap_lock, on a single CPU at a time.
unsafe impl Send for VmFault {}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Check if a VMA is anonymous (no file backing, no vm_ops).
///
/// Ref: Linux `include/linux/mm.h` — `vma_is_anonymous()`
#[inline]
pub fn vma_is_anonymous(vma: *const VmAreaStruct) -> bool {
    unsafe { (*vma).vm_ops == 0 }
}

#[inline]
fn vma_is_shared_anonymous(vma: *const VmAreaStruct) -> bool {
    unsafe { vma_is_anonymous(vma) && (*vma).vm_file == 0 && ((*vma).vm_flags & VM_SHARED) != 0 }
}

unsafe fn ensure_shared_anon_mapping(vma: *mut VmAreaStruct) -> Result<*mut AddressSpace, i32> {
    unsafe {
        if (*vma).vm_private_data != 0 {
            return Ok((*vma).vm_private_data as *mut AddressSpace);
        }

        let mut mapping = Box::new(AddressSpace::new());
        mapping.flags.fetch_or(AS_SHARED_ANON, Ordering::Relaxed);
        let raw = Box::into_raw(mapping);
        (*vma).vm_private_data = raw as usize;
        Ok(raw)
    }
}

/// Compute the GFP mask for allocations during a fault on this VMA.
///
/// Ref: Linux `mm/memory.c` — `__get_fault_gfp_mask()`
#[inline]
fn get_fault_gfp_mask(_vma: *const VmAreaStruct) -> GfpFlags {
    // Future milestones add memcg / zone-specific logic here.
    // NOTE: __GFP_ZERO is intentionally omitted here.  In Linux, anonymous
    // page zeroing uses `clear_user_highpage` (via kmap_atomic + clear_page)
    // rather than buddy-level zeroing — M13 adds that.
    GFP_KERNEL
}

/// Compute the page offset of `address` within `vma`.
///
/// Ref: Linux `include/linux/pagemap.h` — `linear_page_index()`
#[inline]
fn linear_page_index(vma: *const VmAreaStruct, address: u64) -> u64 {
    unsafe {
        let pgoff = (*vma).vm_pgoff;
        pgoff + ((address - (*vma).vm_start) >> PAGE_SHIFT)
    }
}

/// Increment the mm_struct's RSS counter.
///
/// Simplified for M12: no per-type breakdown (MM_ANONPAGES etc.) yet.
///
/// Ref: Linux `include/linux/mm.h` — `add_mm_counter()`
fn add_mm_rss(mm: *mut MmStruct, nr_pages: u64) {
    unsafe {
        (*mm).hiwater_rss += nr_pages;
    }
}

// ---------------------------------------------------------------------------
// Entry point — handle_mm_fault
// ---------------------------------------------------------------------------

/// Entry point from the architecture-specific fault handler.
///
/// Called after the VMA has been found and the access has been validated.
/// Delegates to `__handle_mm_fault` for the page-table walk.
///
/// Ref: Linux `mm/memory.c` — `handle_mm_fault()` line 6589
pub fn handle_mm_fault(vma: *mut VmAreaStruct, address: u64, flags: FaultFlags) -> VmFaultFlags {
    // Future milestones add hugetlb dispatch and memcg accounting here.
    unsafe { __handle_mm_fault(vma, address, flags) }
}

// ---------------------------------------------------------------------------
// Page-table walk — __handle_mm_fault
// ---------------------------------------------------------------------------

/// Walk (and allocate if necessary) PGD → P4D → PUD → PMD, then call
/// `handle_pte_fault` for the leaf-level decision.
///
/// Ref: Linux `mm/memory.c` — `__handle_mm_fault()` line 6355
unsafe fn __handle_mm_fault(
    vma: *mut VmAreaStruct,
    address: u64,
    flags: FaultFlags,
) -> VmFaultFlags {
    unsafe {
        let mm = (*vma).vm_mm;
        let pgd_base = (*mm).pgd as *mut pgd_t;

        let mut vmf = VmFault {
            vma,
            gfp_mask: get_fault_gfp_mask(vma),
            pgoff: linear_page_index(vma, address),
            address: address & PAGE_MASK,
            real_address: address,
            flags,
            pud: ptr::null_mut(),
            pmd: ptr::null_mut(),
            orig_pte: __pte(0),
            pte: ptr::null_mut(),
            page: ptr::null_mut(),
            cow_page: ptr::null_mut(),
        };

        // PGD (PML4) — always present; it is the page-table root.
        let pgdp = pgd_offset_pgd(pgd_base, address);

        // P4D — folded into PGD on 4-level x86_64.
        let p4dp = p4d_offset(pgdp, address);

        // PUD — allocate the page if absent.
        // _PAGE_TABLE includes _PAGE_USER so user-space can traverse it.
        let pudp = match pud_alloc(p4dp as *mut pgd_t, address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                return VM_FAULT_OOM;
            }
        };
        vmf.pud = pudp;

        // PMD — allocate if absent.
        let pmdp = match pmd_alloc(pudp, address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                return VM_FAULT_OOM;
            }
        };
        vmf.pmd = pmdp;

        // THP / huge-page dispatch is deferred to a later milestone.

        handle_pte_fault(&mut vmf)
    }
}

// ---------------------------------------------------------------------------
// PTE-level routing — handle_pte_fault
// ---------------------------------------------------------------------------

/// Determine the nature of the fault at the PTE level and dispatch.
///
/// Ref: Linux `mm/memory.c` — `handle_pte_fault()` line 6273
fn handle_pte_fault(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        if pmd_none(*vmf.pmd) || paging::pmd_huge(*vmf.pmd) {
            // PTE table does not exist — entry is implicitly absent.
            vmf.pte = ptr::null_mut();
            vmf.flags &= !FAULT_FLAG_ORIG_PTE_VALID;
        } else {
            // PTE table exists — read the entry.
            vmf.pte = pte_offset_kernel(vmf.pmd, vmf.address);
            vmf.orig_pte = ptep_get(vmf.pte);
            vmf.flags |= FAULT_FLAG_ORIG_PTE_VALID;

            if pte_none(vmf.orig_pte) {
                vmf.pte = ptr::null_mut();
            }
        }

        if vmf.pte.is_null() {
            // PTE absent — first access, must allocate a page.
            return do_pte_missing(vmf);
        }

        if !pte_present(vmf.orig_pte) {
            // PTE exists but page is swapped out.
            // Swap support is M17; return bus error for now.
            return do_swap_page(vmf);
        }

        // PTE is present.  Write-protect fault → COW.
        let pfn = pte_pfn(vmf.orig_pte) as usize;
        if !pte_special(vmf.orig_pte) && !pfn_valid(pfn) {
            // A stale or poisoned userspace PTE must not crash the kernel.
            // Clear it and let the normal missing-PTE path rebuild the mapping.
            set_pte_at((*vmf.vma).vm_mm as *mut (), vmf.address, vmf.pte, __pte(0));
            flush_tlb_page(vmf.address);
            vmf.orig_pte = __pte(0);
            vmf.pte = ptr::null_mut();
            vmf.flags &= !FAULT_FLAG_ORIG_PTE_VALID;
            return do_pte_missing(vmf);
        }

        if (vmf.flags & FAULT_FLAG_WRITE) != 0
            && !pte_write(vmf.orig_pte)
            && ((*vmf.vma).vm_flags & (VM_SHARED | VM_WRITE)) == (VM_SHARED | VM_WRITE)
        {
            if pte_special(vmf.orig_pte) {
                return wp_pfn_shared(vmf);
            }
            if (*vmf.vma).vm_file != 0 {
                crate::mm::filemap::set_page_dirty(pfn_to_page(pfn));
            }
            let entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(vmf.orig_pte)));
            set_pte_at((*vmf.vma).vm_mm as *mut (), vmf.address, vmf.pte, entry);
            flush_tlb_page(vmf.address);
            return 0;
        }

        if (vmf.flags & FAULT_FLAG_WRITE) != 0 && !pte_write(vmf.orig_pte) {
            return do_wp_page(vmf);
        }

        // Spurious fault or access-bit update: mark young (and dirty on write).
        let mut entry = vmf.orig_pte;
        entry = pte_mkyoung(entry);
        if (vmf.flags & FAULT_FLAG_WRITE) != 0 {
            entry = pte_mkdirty(entry);
        }
        set_pte_at((*vmf.vma).vm_mm as *mut (), vmf.address, vmf.pte, entry);
        flush_tlb_page(vmf.address);

        0 // minor fault, success
    }
}

// ---------------------------------------------------------------------------
// do_pte_missing — route to anonymous or file-backed handler
// ---------------------------------------------------------------------------

/// Route a missing-PTE fault based on VMA type.
///
/// Ref: Linux `mm/memory.c` — `do_pte_missing()`
fn do_pte_missing(vmf: &mut VmFault) -> VmFaultFlags {
    if vma_is_anonymous(vmf.vma) {
        do_anonymous_page(vmf)
    } else {
        do_fault(vmf)
    }
}

// ---------------------------------------------------------------------------
// do_anonymous_page — fresh zeroed page for anonymous VMAs
// ---------------------------------------------------------------------------

/// Handle a fault on an anonymous (not file-backed) page.
///
/// Allocates a fresh zeroed physical frame, builds the PTE with the
/// appropriate protection bits, and installs it.
///
/// Private anonymous mappings stay read-only on first read fault so later
/// writes still flow through COW. Shared anonymous mappings are handled by a
/// separate backing-store path and inherit writable shared protections.
///
/// Ref: Linux `mm/memory.c` — `do_anonymous_page()` line 5217
fn do_anonymous_page(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        let mm = (*vma).vm_mm;

        if vma_is_shared_anonymous(vma) {
            return do_shared_anonymous_page(vmf);
        }

        // Attach an AnonVma to this VMA on its first fault.
        // This enables copy_page_range() to detect that the VMA has pages
        // and must be COW-protected during fork.
        if anon_vma_prepare(vma).is_err() {
            return VM_FAULT_OOM;
        }

        // Ensure the PTE page table exists (_PAGE_TABLE = user-accessible).
        let ptep = match pte_alloc(vmf.pmd, vmf.address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                return VM_FAULT_OOM;
            }
        };

        // Allocate a zeroed physical page from the buddy allocator. Linux
        // anonymous memory is zero-filled; libc allocators depend on this for
        // fresh brk/mmap metadata pages.
        let page_ptr = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
            Some(p) => p,
            None => {
                return VM_FAULT_OOM;
            }
        };
        let pfn = page_to_pfn(page_ptr) as u64;
        #[cfg(not(test))]
        {
            let page_virt = pfn_to_virt(pfn as usize);
            core::ptr::write_bytes(page_virt as *mut u8, 0, PAGE_SIZE as usize);
        }

        // The freshly allocated page is owned by exactly one PTE.  Keep the
        // refcount exclusive so a later write fault can reuse the page in
        // place; fork's COW path increments it when the page is truly shared.
        (*page_ptr)._refcount.store(1, Ordering::Relaxed);
        // mapcount = 0 means exactly one PTE maps this page (exclusive).
        (*page_ptr)._mapcount().store(0, Ordering::Relaxed);

        // Store the AnonVma pointer in page.mapping so try_to_unmap can walk
        // back to this page's VMA when the page is evicted to swap.
        // Also mark as swap-backed (anonymous) and add to the anon LRU.
        if !(*vma).anon_vma.is_null() {
            (*page_ptr).mapping = (*vma).anon_vma as usize;
        }
        (*page_ptr).set_flag(PG_SWAPBACKED);
        crate::mm::lru::lru_cache_add(page_ptr);

        // Build the protection bits from vm_page_prot.
        // Fall back to sensible defaults if vm_page_prot was not set up yet.
        let base_prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };

        let mut entry = pfn_pte(pfn, base_prot);
        entry = pte_mkyoung(entry);

        // Only make writable when the fault is a write on a writable VMA.
        if (vmf.flags & FAULT_FLAG_WRITE) != 0 && ((*vma).vm_flags & VM_WRITE) != 0 {
            entry = pte_mkwrite(entry);
            entry = pte_mkdirty(entry);
        }

        // Install the PTE.
        // No invlpg needed — the PTE was absent, so there is no stale TLB entry.
        set_pte_at(mm as *mut (), vmf.address, ptep, entry);

        // Update RSS (resident set size).
        add_mm_rss(mm, 1);

        vmf.pte = ptep;
        vmf.page = page_ptr;

        0 // success
    }
}

fn do_shared_anonymous_page(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        let mm = (*vma).vm_mm;
        let index = vmf.pgoff;

        let ptep = match pte_alloc(vmf.pmd, vmf.address, _PAGE_TABLE) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };

        let mapping = match ensure_shared_anon_mapping(vma) {
            Ok(mapping) => mapping,
            Err(_) => return VM_FAULT_OOM,
        };

        let page_ptr = if let Some(existing) = (*mapping).i_pages.xa_load(index) {
            existing.as_ptr()
        } else {
            let page = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
                Some(p) => p,
                None => return VM_FAULT_OOM,
            };
            let pfn = page_to_pfn(page) as u64;
            #[cfg(not(test))]
            {
                let page_virt = pfn_to_virt(pfn as usize);
                core::ptr::write_bytes(page_virt as *mut u8, 0, PAGE_SIZE as usize);
            }

            (*page)._refcount.store(1, Ordering::Relaxed);
            (*page)._mapcount().store(-1, Ordering::Relaxed);
            (*page).mapping = mapping as usize;
            (*page).index = index as usize;
            (*page).set_flag(PG_SWAPBACKED);

            let inserted = (*mapping)
                .i_pages
                .xa_store(index, core::ptr::NonNull::new(page).unwrap());
            if let Some(existing) = inserted {
                with_global_buddy(|b| b.free_pages(page, 0));
                existing.as_ptr()
            } else {
                crate::mm::lru::lru_cache_add(page);
                page
            }
        };

        (*page_ptr).get_page();
        (*page_ptr)._mapcount().fetch_add(1, Ordering::Relaxed);

        let base_prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };

        let mut entry = pte_mkyoung(pfn_pte(page_to_pfn(page_ptr) as u64, base_prot));
        if (*vma).vm_flags & VM_WRITE != 0 {
            entry = pte_mkwrite(entry);
            if (vmf.flags & FAULT_FLAG_WRITE) != 0 {
                entry = pte_mkdirty(entry);
            }
        }

        set_pte_at(mm as *mut (), vmf.address, ptep, entry);
        add_mm_rss(mm, 1);

        vmf.pte = ptep;
        vmf.page = page_ptr;

        0
    }
}

// ---------------------------------------------------------------------------
// do_wp_page + wp_page_copy + wp_page_reuse — copy-on-write (M14)
// ---------------------------------------------------------------------------

/// Handle a shared write-protect fault on a raw PFN mapping.
///
/// This is Linux `wp_pfn_shared()`: honor an optional `pfn_mkwrite` callback,
/// then reuse the existing PTE instead of ever COW-copying a shared mapping.
fn wp_pfn_shared(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        if vma.is_null() {
            return VM_FAULT_SIGBUS;
        }

        if (*vma).vm_ops != 0 {
            let ops = &*((*vma).vm_ops as *const VmOperationsStruct);
            if let Some(pfn_mkwrite) = ops.pfn_mkwrite {
                vmf.flags |= FAULT_FLAG_MKWRITE;
                let ret = pfn_mkwrite(vmf as *mut VmFault);
                if ret & (VM_FAULT_ERROR | VM_FAULT_NOPAGE) != 0 {
                    return ret;
                }
            }
        }

        if ptep_get(vmf.pte) != vmf.orig_pte {
            return VM_FAULT_NOPAGE;
        }
        let entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(vmf.orig_pte)));
        set_pte_at((*vma).vm_mm as *mut (), vmf.address, vmf.pte, entry);
        flush_tlb_page(vmf.address);
        0
    }
}

unsafe fn wp_can_reuse_lupos_anon_page(vma: *const VmAreaStruct, page: *const Page) -> bool {
    if vma.is_null() || page.is_null() {
        return false;
    }
    unsafe {
        (*vma).vm_file == 0
            && !(*vma).anon_vma.is_null()
            && (*page).mapping == (*vma).anon_vma as usize
            && (*page).test_flag(PG_SWAPBACKED)
            && (*page).refcount() <= 1
    }
}

/// Handle a write-protect fault on a present PTE.
///
/// Dispatches between:
/// - `wp_page_reuse`: the page is anonymous and exclusively reusable.
/// - `wp_page_copy`: file-backed, raw PFN, or non-exclusive anonymous pages.
///
/// Ref: Linux `mm/memory.c` — `do_wp_page()`
fn do_wp_page(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        // vm_normal_page() returns NULL for a PTE-special VM_PFNMAP entry.
        // Shared mappings reuse the raw PFN; only a private mapping allocates
        // an anonymous copy because there is no struct-page refcount to test.
        if pte_special(vmf.orig_pte) {
            if (*vmf.vma).vm_flags & (VM_SHARED | VM_MAYSHARE) != 0 {
                return wp_pfn_shared(vmf);
            }
            return wp_page_copy(vmf);
        }

        let pfn = paging::pte_pfn(vmf.orig_pte) as usize;
        let page_ptr = pfn_to_page(pfn);

        if wp_can_reuse_lupos_anon_page(vmf.vma, page_ptr) {
            let vma = vmf.vma;
            let mm = (*vma).vm_mm;
            let entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(vmf.orig_pte)));
            set_pte_at(mm as *mut (), vmf.address, vmf.pte, entry);
            flush_tlb_page(vmf.address);
            return 0;
        }

        wp_page_copy(vmf)
    }
}

/// Full COW copy path — allocate a new page, copy the old content, install a
/// writable PTE in the faulting mm, and release the old page's references.
///
/// Called from `do_wp_page` when a normal page has `refcount > 1`, or when a
/// private PFNMAP PTE has no `struct page` and must become anonymous.
///
/// # Return value
/// `VM_FAULT_DONE_COW` on success, `VM_FAULT_OOM` if allocation fails.
///
/// Ref: Linux `mm/memory.c` — `wp_page_copy()` line 3758
fn wp_page_copy(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        let mm = (*vma).vm_mm;

        if anon_vma_prepare(vma).is_err() {
            return VM_FAULT_OOM;
        }

        // 1. Allocate a fresh page for the private copy.
        let new_page = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };

        // 2. Copy the old mapping's content into the new page. Linux's
        //    __wp_page_copy_user() reads through the userspace address when
        //    vm_normal_page() returned NULL for a special PFN mapping; there
        //    is deliberately no pfn_to_page() in that case.
        let old_pfn = paging::pte_pfn(vmf.orig_pte) as usize;
        let old_page = if pte_special(vmf.orig_pte) {
            core::ptr::null_mut()
        } else {
            pfn_to_page(old_pfn)
        };
        let dst = fault_page_kaddr(new_page);
        if dst.is_null() {
            free_unmapped_fault_page(new_page);
            return VM_FAULT_OOM;
        }
        if old_page.is_null() {
            let mut not_copied = crate::arch::x86::kernel::uaccess::copy_from_user(
                dst as *mut u8,
                vmf.address as *const u8,
                PAGE_SIZE as usize,
            );
            if not_copied != 0 {
                // Linux retries under the PTL, then zero-fills an unreadable
                // but still-stable PFN source. Lupos has no per-PTE lock yet;
                // revalidate the PTE around the retry so a concurrent change
                // causes a harmless fault retry instead of copying stale data.
                if ptep_get(vmf.pte) != vmf.orig_pte {
                    free_unmapped_fault_page(new_page);
                    return 0;
                }
                not_copied = crate::arch::x86::kernel::uaccess::copy_from_user(
                    dst as *mut u8,
                    vmf.address as *const u8,
                    PAGE_SIZE as usize,
                );
                if ptep_get(vmf.pte) != vmf.orig_pte {
                    free_unmapped_fault_page(new_page);
                    return 0;
                }
                if not_copied != 0 {
                    core::ptr::write_bytes(dst as *mut u8, 0, PAGE_SIZE as usize);
                }
            }
        } else {
            let src = fault_page_kaddr(old_page);
            if src.is_null() {
                free_unmapped_fault_page(new_page);
                return VM_FAULT_SIGBUS;
            }
            core::ptr::copy_nonoverlapping(src, dst, PAGE_SIZE as usize);
        }

        // 3. Initialise the new page's refcount/mapcount.
        //    One PTE (below) will map it, so mapcount = 0 (exclusive).
        (*new_page)._refcount.store(1, Ordering::Relaxed);
        (*new_page)._mapcount().store(0, Ordering::Relaxed);
        if !(*vma).anon_vma.is_null() {
            (*new_page).mapping = (*vma).anon_vma as usize;
        }
        (*new_page).set_flag(PG_SWAPBACKED);
        (*new_page).init_lru();
        crate::mm::lru::lru_cache_add(new_page);

        // 4. Build a new writable PTE for the new page.
        let new_pfn = page_to_pfn(new_page) as u64;
        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(
                crate::arch::x86::mm::paging::_PAGE_PRESENT
                    | crate::arch::x86::mm::paging::_PAGE_USER
                    | crate::arch::x86::mm::paging::_PAGE_ACCESSED,
            )
        };
        let new_entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(pfn_pte(new_pfn, prot))));

        // 5. Replace the old PTE with the new one using Linux's
        //    ptep_clear_flush-before-set ordering.
        if ptep_get(vmf.pte) != vmf.orig_pte {
            crate::mm::lru::remove_lru_page(new_page);
            free_unmapped_fault_page(new_page);
            return 0;
        }
        ptep_get_and_clear(mm as *mut (), vmf.address, vmf.pte);
        flush_tlb_page(vmf.address);
        set_pte_at(mm as *mut (), vmf.address, vmf.pte, new_entry);

        // 6. Release the old page's references from this mm.
        //    mapcount: one fewer PTE maps it.
        if !old_page.is_null() {
            (*old_page)._mapcount().fetch_sub(1, Ordering::Relaxed);
            //    refcount: this mm no longer holds a reference.
            let rc = (*old_page).put_page();
            if rc <= 0 {
                // No remaining references — return the page to the buddy allocator.
                crate::mm::lru::remove_lru_page(old_page);
                with_global_buddy(|b| b.free_pages(old_page, 0));
            }
        }

        VM_FAULT_DONE_COW
    }
}

// ---------------------------------------------------------------------------
// copy_page_range + copy_pte_range — page table copying for fork
// ---------------------------------------------------------------------------

/// Copy the page tables for one VMA from the source mm to the destination mm.
///
/// For private VMAs that may be written (`VM_MAYWRITE && !VM_SHARED`), each
/// writable present PTE is write-protected in both parent and child. Other
/// mappings retain their source write permission; shared child PTEs are copied
/// clean, and all child PTEs are copied old, matching Linux.
///
/// For every copied normal present PTE:
/// - The backing page's refcount and mapcount are both incremented.
///
/// Called from `dup_mmap()` for each source VMA during `fork`.
///
/// # Safety
/// `dst_vma` and `src_vma` must be valid VMAs. `dst_vma` must belong to the
/// destination mm and `src_vma` must belong to the source mm.
///
/// Ref: Linux `mm/memory.c` — `copy_page_range()` line 1504
pub unsafe fn copy_page_range(
    dst_vma: *mut VmAreaStruct,
    src_vma: *mut VmAreaStruct,
) -> Result<(), i32> {
    unsafe {
        if !vma_needs_copy(dst_vma, src_vma) {
            return Ok(());
        }

        let dst_mm = (*dst_vma).vm_mm;
        let src_mm = (*src_vma).vm_mm;
        let flags = (*src_vma).vm_flags;

        let src_pgd = (*src_mm).pgd as *mut pgd_t;
        let dst_pgd = (*dst_mm).pgd as *mut pgd_t;
        if src_pgd.is_null() || dst_pgd.is_null() {
            return Ok(());
        }

        let mut addr = (*src_vma).vm_start;
        let end = (*src_vma).vm_end;

        while addr < end {
            // Locate PGD entries for this address in both mm structs.
            let src_pgdp = pgd_offset_pgd(src_pgd, addr);
            if paging::pgd_none(*src_pgdp) {
                // Whole PGD range absent in source → skip.
                addr = pgd_addr_end(addr);
                continue;
            }

            let dst_pgdp = pgd_offset_pgd(dst_pgd, addr);

            // PGD is present — walk down to PUD level.
            // (P4D is folded into PGD on 4-level x86-64.)
            let src_pudp = {
                let p4dp = p4d_offset(src_pgdp, addr);
                pud_offset(p4dp, addr)
            };
            if pud_none(*src_pudp) || pud_huge(*src_pudp) {
                addr = pud_addr_end(addr);
                continue;
            }

            // Allocate destination PUD if absent.
            let dst_pudp = pud_alloc(dst_pgdp, addr, crate::arch::x86::mm::paging::_PAGE_TABLE)
                .ok_or(-12i32)?;

            // Walk down to PMD level.
            let src_pmdp = pmd_offset(src_pudp, addr);
            if pmd_none(*src_pmdp) || pmd_huge(*src_pmdp) {
                addr = pmd_addr_end(addr);
                continue;
            }

            // Allocate destination PMD if absent.
            let dst_pmdp = pmd_alloc(dst_pudp, addr, crate::arch::x86::mm::paging::_PAGE_TABLE)
                .ok_or(-12i32)?;

            // Compute the end of this PMD's range.
            let next = pmd_addr_end(addr).min(end);

            // Copy PTE-level entries.
            copy_pte_range(dst_mm, src_mm, dst_pmdp, src_pmdp, addr, next, flags)?;

            addr = next;
        }

        Ok(())
    }
}

/// Linux `vma_needs_copy()`: fork copies PTEs only when metadata cannot be
/// reconstructed by a later page fault.
///
/// Lupos currently models the `VM_COPY_ON_FORK` subset that exists in
/// `vm_flags.rs` (`VM_PFNMAP | VM_MIXEDMAP`). `VM_UFFD_WP` and
/// `VM_MAYBE_GUARD` are not represented in this tree yet.
#[inline]
fn vma_needs_copy(dst_vma: *const VmAreaStruct, src_vma: *const VmAreaStruct) -> bool {
    unsafe {
        const VM_COPY_ON_FORK: VmFlags = VM_PFNMAP | VM_MIXEDMAP;
        if (*dst_vma).vm_flags & VM_COPY_ON_FORK != 0 {
            return true;
        }
        if !(*src_vma).anon_vma.is_null() {
            return true;
        }
        false
    }
}

/// Copy PTE entries from `[addr, end)` in the source PMD to the destination PMD.
///
/// For each present source PTE, apply Linux's `__copy_present_ptes()` policy:
/// write-protect both copies only for a COW mapping, mark a shared child clean,
/// mark every child old, and increment a normal backing page's `_refcount` and
/// `_mapcount`. Raw PTE-special PFN mappings have no `struct page` and skip
/// accounting.
///
/// Source PTE permission downgrades are intentionally not invalidated here.
/// Linux batches them and performs one full `flush_tlb_mm(oldmm)` from
/// `dup_mmap()`, including its partial-failure path.
///
/// # Safety
/// `dst_mm`, `src_mm`, `dst_pmd`, `src_pmd` must be valid.
/// `addr` and `end` must be PMD-aligned within the VMA's range.
///
/// Ref: Linux `mm/memory.c` — `copy_pte_range()` line 1221
unsafe fn copy_pte_range(
    dst_mm: *mut MmStruct,
    src_mm: *mut MmStruct,
    dst_pmd: *mut pmd_t,
    src_pmd: *mut pmd_t,
    addr: u64,
    end: u64,
    vm_flags: VmFlags,
) -> Result<(), i32> {
    unsafe {
        // Allocate destination PTE table if absent.
        pte_alloc(dst_pmd, addr, crate::arch::x86::mm::paging::_PAGE_TABLE).ok_or(-12i32)?;

        let mut cur = addr;
        while cur < end {
            // Read the source PTE.
            let src_ptep = pte_offset_kernel(src_pmd, cur);
            let src_pte = ptep_get(src_ptep);

            if pte_none(src_pte) || !paging::pte_present(src_pte) {
                cur += PAGE_SIZE as u64;
                continue;
            }

            let (source_update, child_pte) = fork_present_pte(src_pte, vm_flags);
            if let Some(source_pte) = source_update {
                set_pte_at(src_mm as *mut (), cur, src_ptep, source_pte);
            }

            // Install the Linux-adjusted PTE in the destination. Parent
            // permission downgrades are flushed once by dup_mmap().
            let dst_ptep = pte_offset_kernel(dst_pmd, cur);
            set_pte_at(dst_mm as *mut (), cur, dst_ptep, child_pte);

            // vm_normal_page() returns NULL for PTE-special PFNMAP entries.
            // Linux copies the raw PTE to the child but deliberately performs
            // no struct-page refcount, mapcount, rmap, or RSS accounting.
            if pte_special(src_pte) {
                cur += PAGE_SIZE as u64;
                continue;
            }

            let pfn = paging::pte_pfn(src_pte) as usize;
            let page = pfn_to_page(pfn);

            // Bump refcount: destination mm now holds a reference.
            (*page).get_page();
            // Bump mapcount: one more PTE maps this page.
            (*page)._mapcount().fetch_add(1, Ordering::Relaxed);

            // Update destination RSS.
            add_mm_rss(dst_mm, 1);

            cur += PAGE_SIZE as u64;
        }

        Ok(())
    }
}

/// Compute Linux's `__copy_present_ptes()` parent/child PTE updates.
///
/// The optional first result is the only source-PTE write: Linux modifies the
/// parent only when a writable PTE belongs to a private mapping that may be
/// written. The child additionally starts clean for a shared mapping and old
/// for every mapping.
#[inline]
fn fork_present_pte(src_pte: pte_t, vm_flags: VmFlags) -> (Option<pte_t>, pte_t) {
    let is_cow = vm_flags & (VM_SHARED | VM_MAYWRITE) == VM_MAYWRITE;
    let (source_update, mut child_pte) = if is_cow && pte_write(src_pte) {
        let read_only = pte_wrprotect(src_pte);
        (Some(read_only), read_only)
    } else {
        (None, src_pte)
    };

    if vm_flags & VM_SHARED != 0 {
        child_pte = pte_mkclean(child_pte);
    }
    child_pte = pte_mkold(child_pte);

    (source_update, child_pte)
}

// ---------------------------------------------------------------------------
// Address-range rounding helpers (mirrors Linux pXd_addr_end macros)
// ---------------------------------------------------------------------------

/// Round `addr` up to the next PGD boundary, capped at `u64::MAX`.
#[inline]
fn pgd_addr_end(addr: u64) -> u64 {
    // PGD covers 512 GiB (39-bit shift on 4-level x86-64).
    const PGDIR_SIZE: u64 = 1 << 39;
    addr.wrapping_add(PGDIR_SIZE) & !(PGDIR_SIZE - 1)
}

/// Round `addr` up to the next PUD boundary, capped at `u64::MAX`.
#[inline]
fn pud_addr_end(addr: u64) -> u64 {
    const PUD_SIZE: u64 = 1 << 30; // 1 GiB
    addr.wrapping_add(PUD_SIZE) & !(PUD_SIZE - 1)
}

/// Round `addr` up to the next PMD boundary, capped at `u64::MAX`.
#[inline]
fn pmd_addr_end(addr: u64) -> u64 {
    const PMD_SIZE: u64 = 1 << 21; // 2 MiB
    addr.wrapping_add(PMD_SIZE) & !(PMD_SIZE - 1)
}

// ---------------------------------------------------------------------------
// VmOperationsStruct — per-VMA vtable (M15)
// ---------------------------------------------------------------------------

/// Per-VMA vtable — mirrors Linux `struct vm_operations_struct`.
///
/// Filesystems and device drivers populate this struct with callbacks.
/// `VmAreaStruct::vm_ops` points to an instance of this struct.
///
/// Ref: Linux `struct vm_operations_struct` — `include/linux/mm.h:576`
#[repr(C)]
pub struct VmOperationsStruct {
    /// Called when a new VMA is opened (e.g. after fork or mremap).
    pub open: Option<unsafe extern "C" fn(*mut VmAreaStruct)>,
    /// Called when a VMA is closed.
    pub close: Option<unsafe extern "C" fn(*mut VmAreaStruct)>,
    /// Handle a page fault on this VMA.
    pub fault: Option<unsafe extern "C" fn(*mut VmFault) -> VmFaultFlags>,
    /// Pre-map a range of pages (readahead optimisation).
    pub map_pages: Option<unsafe extern "C" fn(*mut VmFault, u64, u64)>,
    /// Called on write-protect fault when `VM_PFNMAP` is set.
    pub pfn_mkwrite: Option<unsafe extern "C" fn(*mut VmFault) -> VmFaultFlags>,
    /// Called by `access_process_vm` to copy data in/out of this VMA.
    pub access: Option<unsafe extern "C" fn(*mut VmAreaStruct, u64, *mut u8, i32, u32) -> i32>,
}

/// Complete a page fault whose page was selected by a Linux-built module.
///
/// Vendor callbacks follow Linux's `->fault` contract: they return a
/// referenced `struct page` in `vmf->page`, while Linux's generic fault layer
/// installs the PTE afterward. Lupos fault callbacks normally install their
/// own PTEs, so the char-device ABI bridge calls this adapter to perform that
/// missing generic step.
pub unsafe fn finish_linux_module_page_fault(
    vmf: *mut VmFault,
    page: *mut Page,
    result: VmFaultFlags,
) -> VmFaultFlags {
    if vmf.is_null() || result & VM_FAULT_ERROR != 0 {
        return result;
    }
    if page.is_null() {
        return if result != 0 { result } else { VM_FAULT_SIGBUS };
    }

    unsafe {
        let vma = (*vmf).vma;
        if vma.is_null() {
            (*page).put_page();
            return VM_FAULT_SIGBUS;
        }
        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(ptep) => ptep,
            None => {
                (*page).put_page();
                return VM_FAULT_OOM;
            }
        };
        if !pte_none(ptep_get(ptep)) {
            (*page).put_page();
            return VM_FAULT_NOPAGE;
        }

        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let mut entry = pte_mkyoung(pfn_pte(page_to_pfn(page) as u64, prot));
        if (*vmf).flags & FAULT_FLAG_WRITE != 0 && (*vma).vm_flags & VM_WRITE != 0 {
            entry = pte_mkwrite(pte_mkdirty(entry));
        }
        set_pte_at((*vma).vm_mm as *mut (), (*vmf).address, ptep, entry);
        (*page)._mapcount().fetch_add(1, Ordering::Relaxed);
        (*vmf).page = page;
        (*vmf).pte = ptep;
        add_mm_rss((*vma).vm_mm, 1);
        result
    }
}

// ---------------------------------------------------------------------------
// filemap_fault — page-cache backed fault handler
// ---------------------------------------------------------------------------

/// Fault handler for file-backed VMAs.
///
/// Looks up (or allocates) the page at the faulting index in the mapping's
/// page cache, optionally calls `a_ops->read_folio` to fill it, installs a
/// read-only PTE, and returns `VM_FAULT_LOCKED` with the page still locked.
///
/// The VMA's `vm_file` field is cast directly to `*mut AddressSpace` (M15
/// simplification; M38 introduces a proper `struct file`).
///
/// Ref: Linux `mm/filemap.c` — `filemap_fault()` line 3225
/// Minimal VFS-backed MAP_PRIVATE fault ops for dynamically linked userland.
pub static LUPOS_FILE_VM_OPS: VmOperationsStruct = VmOperationsStruct {
    open: None,
    close: None,
    fault: Some(lupos_file_fault),
    map_pages: None,
    pfn_mkwrite: None,
    access: None,
};

unsafe extern "C" fn lupos_file_fault(vmf: *mut VmFault) -> VmFaultFlags {
    use alloc::sync::Arc;

    if vmf.is_null() {
        return VM_FAULT_SIGBUS;
    }

    unsafe {
        let vma = (*vmf).vma;
        if vma.is_null() || (*vma).vm_file == 0 {
            return VM_FAULT_SIGBUS;
        }

        let file_ptr = (*vma).vm_file as *const crate::fs::types::File;
        Arc::increment_strong_count(file_ptr);
        let file = Arc::from_raw(file_ptr);
        let cacheable =
            file.fops.read.is_some() && file.inode().is_some_and(|inode| inode.is_reg());
        let ret = if cacheable {
            lupos_cached_file_fault(vmf, &file)
        } else {
            lupos_uncached_file_fault(vmf, &file)
        };
        drop(file);
        ret
    }
}

unsafe fn lupos_file_cow_fault(vmf: *mut VmFault) -> VmFaultFlags {
    use alloc::sync::Arc;

    if vmf.is_null() {
        return VM_FAULT_SIGBUS;
    }

    unsafe {
        let vma = (*vmf).vma;
        if vma.is_null() || (*vma).vm_file == 0 {
            return VM_FAULT_SIGBUS;
        }

        let file_ptr = (*vma).vm_file as *const crate::fs::types::File;
        Arc::increment_strong_count(file_ptr);
        let file = Arc::from_raw(file_ptr);
        let cacheable =
            file.fops.read.is_some() && file.inode().is_some_and(|inode| inode.is_reg());
        let ret = if cacheable {
            lupos_cached_file_cow_fault(vmf, &file)
        } else {
            lupos_uncached_file_cow_fault(vmf, &file)
        };
        drop(file);
        ret
    }
}

unsafe fn lupos_lookup_cached_file_page(
    vmf: *mut VmFault,
    file: &crate::fs::types::FileRef,
) -> Result<*mut Page, VmFaultFlags> {
    use crate::mm::address_space::{page_uptodate, set_page_uptodate, unlock_page};
    use crate::mm::filemap::{filemap_grab_folio, find_lock_page};

    unsafe {
        let vma = (*vmf).vma;
        let Some(inode) = file.inode() else {
            return Err(VM_FAULT_SIGBUS);
        };
        let mapping = inode.mapping();
        let index = (*vmf).pgoff;
        let max_idx = inode
            .size
            .load(Ordering::Acquire)
            .saturating_add(PAGE_SIZE - 1)
            / PAGE_SIZE;
        if index >= max_idx {
            return Err(VM_FAULT_SIGBUS);
        }

        let mut page = find_lock_page(mapping, index);
        if page.is_null() {
            page = filemap_grab_folio(mapping, index);
        }
        if page.is_null() {
            return Err(VM_FAULT_OOM);
        }

        if !page_uptodate(page) {
            let page_virt = fault_page_kaddr(page);
            if page_virt.is_null() {
                unlock_page(page);
                (*page).put_page();
                return Err(VM_FAULT_SIGBUS);
            }
            core::ptr::write_bytes(page_virt, 0, PAGE_SIZE as usize);
            let Some(read) = file.fops.read else {
                unlock_page(page);
                (*page).put_page();
                return Err(VM_FAULT_SIGBUS);
            };
            let mut pos = index.saturating_mul(PAGE_SIZE);
            let buf = core::slice::from_raw_parts_mut(page_virt, PAGE_SIZE as usize);
            if read(file, buf, &mut pos).is_err() {
                unlock_page(page);
                (*page).put_page();
                return Err(VM_FAULT_SIGBUS);
            }
            set_page_uptodate(page);
        }

        // Linux rechecks both page identity and i_size under the folio lock;
        // truncate or invalidation may have raced the backing read.
        let max_idx = inode
            .size
            .load(Ordering::Acquire)
            .saturating_add(PAGE_SIZE - 1)
            / PAGE_SIZE;
        if (*page).mapping != mapping as usize
            || (*page).index != index as usize
            || index >= max_idx
        {
            unlock_page(page);
            (*page).put_page();
            return Err(VM_FAULT_SIGBUS);
        }

        if vma.is_null() {
            unlock_page(page);
            (*page).put_page();
            return Err(VM_FAULT_SIGBUS);
        }

        Ok(page)
    }
}

/// Current Rust-VFS equivalent of Linux's `filemap_fault()` plus
/// `finish_fault()` for a regular-file mapping.
///
/// Lupos installs PTEs inside the VMA fault callback, so this consumes the
/// lookup reference as the PTE reference and unlocks the cache page before
/// returning instead of propagating `VM_FAULT_LOCKED` to a missing upper
/// `finish_fault()` layer.
unsafe fn lupos_cached_file_fault(
    vmf: *mut VmFault,
    file: &crate::fs::types::FileRef,
) -> VmFaultFlags {
    use crate::mm::address_space::unlock_page;

    unsafe {
        let vma = (*vmf).vma;
        let page = match lupos_lookup_cached_file_page(vmf, file) {
            Ok(page) => page,
            Err(fault) => return fault,
        };

        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                unlock_page(page);
                (*page).put_page();
                return VM_FAULT_OOM;
            }
        };
        if !pte_none(ptep_get(ptep)) {
            unlock_page(page);
            (*page).put_page();
            return VM_FAULT_NOPAGE;
        }

        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let mut entry = pte_mkyoung(pfn_pte(page_to_pfn(page) as u64, prot));
        let shared_write = (*vma).vm_flags & (VM_SHARED | VM_WRITE) == (VM_SHARED | VM_WRITE);
        if shared_write && (*vmf).flags & FAULT_FLAG_WRITE != 0 {
            // Linux page_mkwrite semantics: with the folio locked, revalidate
            // the cache identity after any read/truncate race, wait for
            // conflicting writeback, then dirty the cache folio once before
            // installing the writable PTE.
            wait_on_page_writeback(page);
            let expected_mapping = file.mapping().map_or(0usize, |mapping| mapping as usize);
            if (*page).mapping != expected_mapping || (*page).index != (*vmf).pgoff as usize {
                unlock_page(page);
                (*page).put_page();
                return VM_FAULT_SIGBUS;
            }
            crate::mm::filemap::set_page_dirty(page);
            entry = pte_mkwrite(pte_mkdirty(entry));
        } else {
            // Private mappings must COW on a later write. Writable shared
            // mappings also start read-only after a read fault so their first
            // write passes through the dirty-page bookkeeping above.
            entry = pte_wrprotect(entry);
        }
        set_pte_at((*vma).vm_mm as *mut (), (*vmf).address, ptep, entry);
        (*page)._mapcount().fetch_add(1, Ordering::Relaxed);
        (*vmf).page = page;
        (*vmf).pte = ptep;
        add_mm_rss((*vma).vm_mm, 1);
        crate::mm::lru::mark_page_accessed(page);
        unlock_page(page);

        // Do not put `page`: the caller reference is now owned by this PTE.
        0
    }
}

unsafe fn lupos_cached_file_cow_fault(
    vmf: *mut VmFault,
    file: &crate::fs::types::FileRef,
) -> VmFaultFlags {
    use crate::mm::address_space::unlock_page;

    unsafe {
        let vma = (*vmf).vma;
        let mm = (*vma).vm_mm;
        if anon_vma_prepare(vma).is_err() {
            return VM_FAULT_OOM;
        }

        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };
        if !pte_none(ptep_get(ptep)) {
            return VM_FAULT_NOPAGE;
        }

        let file_page = match lupos_lookup_cached_file_page(vmf, file) {
            Ok(page) => page,
            Err(fault) => return fault,
        };

        let new_page = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
            Some(page) => page,
            None => {
                unlock_page(file_page);
                (*file_page).put_page();
                return VM_FAULT_OOM;
            }
        };

        let src = fault_page_kaddr(file_page);
        let dst = fault_page_kaddr(new_page);
        if src.is_null() || dst.is_null() {
            free_unmapped_fault_page(new_page);
            unlock_page(file_page);
            (*file_page).put_page();
            return VM_FAULT_SIGBUS;
        }
        core::ptr::copy_nonoverlapping(src, dst, PAGE_SIZE as usize);

        (*new_page)._refcount.store(1, Ordering::Relaxed);
        (*new_page)._mapcount().store(0, Ordering::Relaxed);
        if !(*vma).anon_vma.is_null() {
            (*new_page).mapping = (*vma).anon_vma as usize;
        }
        (*new_page).set_flag(PG_SWAPBACKED);
        (*new_page).init_lru();
        crate::mm::lru::lru_cache_add(new_page);

        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(pfn_pte(
            page_to_pfn(new_page) as u64,
            prot,
        ))));
        set_pte_at(mm as *mut (), (*vmf).address, ptep, entry);
        (*vmf).cow_page = new_page;
        (*vmf).page = new_page;
        (*vmf).pte = ptep;
        add_mm_rss(mm, 1);

        crate::mm::lru::mark_page_accessed(file_page);
        unlock_page(file_page);
        (*file_page).put_page();
        0
    }
}

/// Legacy one-page-per-VMA-fault path retained for shared/device-like files
/// until those mappings have Linux page_mkwrite/writeback coherence.
unsafe fn lupos_uncached_file_fault(
    vmf: *mut VmFault,
    file: &crate::fs::types::FileRef,
) -> VmFaultFlags {
    unsafe {
        let vma = (*vmf).vma;
        let page_ptr = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };
        (*page_ptr)._refcount.store(1, Ordering::Relaxed);
        (*page_ptr)._mapcount().store(0, Ordering::Relaxed);

        let page_virt = pfn_to_virt(page_to_pfn(page_ptr)) as *mut u8;
        core::ptr::write_bytes(page_virt, 0, PAGE_SIZE as usize);

        if let Some(read) = file.fops.read {
            let mut pos = (*vmf).pgoff.saturating_mul(PAGE_SIZE);
            let buf = core::slice::from_raw_parts_mut(page_virt, PAGE_SIZE as usize);
            let _ = read(file, buf, &mut pos);
        }

        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                with_global_buddy(|b| b.free_pages(page_ptr, 0));
                return VM_FAULT_OOM;
            }
        };

        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let mut entry = pte_mkyoung(pfn_pte(page_to_pfn(page_ptr) as u64, prot));
        if ((*vmf).flags & FAULT_FLAG_WRITE) != 0 && ((*vma).vm_flags & VM_WRITE) != 0 {
            entry = pte_mkwrite(pte_mkdirty(entry));
        }
        set_pte_at((*vma).vm_mm as *mut (), (*vmf).address, ptep, entry);
        (*vmf).page = page_ptr;
        (*vmf).pte = ptep;
        add_mm_rss((*vma).vm_mm, 1);
        0
    }
}

unsafe fn lupos_uncached_file_cow_fault(
    vmf: *mut VmFault,
    file: &crate::fs::types::FileRef,
) -> VmFaultFlags {
    unsafe {
        let vma = (*vmf).vma;
        let mm = (*vma).vm_mm;
        if anon_vma_prepare(vma).is_err() {
            return VM_FAULT_OOM;
        }

        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };
        if !pte_none(ptep_get(ptep)) {
            return VM_FAULT_NOPAGE;
        }

        let page_ptr = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };
        (*page_ptr)._refcount.store(1, Ordering::Relaxed);
        (*page_ptr)._mapcount().store(0, Ordering::Relaxed);
        if !(*vma).anon_vma.is_null() {
            (*page_ptr).mapping = (*vma).anon_vma as usize;
        }
        (*page_ptr).set_flag(PG_SWAPBACKED);
        (*page_ptr).init_lru();

        let page_virt = fault_page_kaddr(page_ptr);
        if page_virt.is_null() {
            free_unmapped_fault_page(page_ptr);
            return VM_FAULT_SIGBUS;
        }
        core::ptr::write_bytes(page_virt, 0, PAGE_SIZE as usize);

        if let Some(read) = file.fops.read {
            let mut pos = (*vmf).pgoff.saturating_mul(PAGE_SIZE);
            let buf = core::slice::from_raw_parts_mut(page_virt, PAGE_SIZE as usize);
            if read(file, buf, &mut pos).is_err() {
                free_unmapped_fault_page(page_ptr);
                return VM_FAULT_SIGBUS;
            }
        }

        crate::mm::lru::lru_cache_add(page_ptr);

        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let entry = pte_mkwrite(pte_mkdirty(pte_mkyoung(pfn_pte(
            page_to_pfn(page_ptr) as u64,
            prot,
        ))));
        set_pte_at(mm as *mut (), (*vmf).address, ptep, entry);
        (*vmf).cow_page = page_ptr;
        (*vmf).page = page_ptr;
        (*vmf).pte = ptep;
        add_mm_rss(mm, 1);
        0
    }
}

// ---------------------------------------------------------------------------
// lupos_device_pfn_fault — direct MMIO/PFN device mappings (fbdev, …)
// ---------------------------------------------------------------------------

/// vm_ops for `VM_PFNMAP` device mappings such as `/dev/fb0`.
///
/// Unlike a page-cache mapping, a fault here does **not** allocate a fresh
/// page: it maps the device's own physical aperture straight into the calling
/// process, so userspace writes reach the hardware (a framebuffer's scanout
/// memory, in the fbdev case) instead of a throwaway private copy.
///
/// Ref: Linux `remap_pfn_range()` plus a driver `->mmap` that sets `VM_PFNMAP`.
pub static LUPOS_DEVICE_PFN_VM_OPS: VmOperationsStruct = VmOperationsStruct {
    open: None,
    close: None,
    fault: Some(lupos_device_pfn_fault),
    map_pages: None,
    pfn_mkwrite: None,
    access: None,
};

/// Record the PFN mapping prepared by a file's one-time `->mmap` callback.
///
/// Linux's `remap_pfn_range()` installs the complete PTE range while mmap is
/// creating the VMA. Lupos currently materializes those PTEs lazily, so the
/// equivalent immutable mapping state is retained in the VMA and consumed by
/// `lupos_device_pfn_fault()`. `vm_private_data` stores a PFN-address bias so
/// VMA splits can advance `vm_pgoff` without needing to rewrite private state.
pub fn prepare_lupos_device_pfn_mapping(vma: &mut VmAreaStruct, mapped_phys: u64) {
    // remap_pfn_range() replaces vm_pgoff with the first mapped PFN for a
    // private COW mapping. vm_normal_page() and /proc VMA reporting rely on
    // this rule; shared mappings retain the file-provided offset.
    if vma.vm_flags & (VM_SHARED | VM_MAYWRITE) == VM_MAYWRITE {
        vma.vm_pgoff = mapped_phys >> PAGE_SHIFT;
    }
    let byte_off = vma.vm_pgoff.wrapping_shl(PAGE_SHIFT as u32);
    vma.vm_private_data = mapped_phys.wrapping_sub(byte_off) as usize;
    vma.vm_flags |= VM_IO | VM_PFNMAP | VM_DONTEXPAND | VM_DONTDUMP;
    vma.vm_ops = &LUPOS_DEVICE_PFN_VM_OPS as *const VmOperationsStruct as usize;
}

unsafe extern "C" fn lupos_device_pfn_fault(vmf: *mut VmFault) -> VmFaultFlags {
    if vmf.is_null() {
        return VM_FAULT_SIGBUS;
    }

    unsafe {
        let vma = (*vmf).vma;
        if vma.is_null() || (*vma).vm_file == 0 {
            return VM_FAULT_SIGBUS;
        }

        if (*vmf).address < (*vma).vm_start || (*vmf).address >= (*vma).vm_end {
            return VM_FAULT_SIGBUS;
        }

        // The file's mmap callback already validated the complete VMA and
        // recorded the physical-address bias. Faults only materialize the PTE
        // selected by the VMA's page offset; they never call ->mmap again.
        let byte_off = (*vmf).pgoff.wrapping_shl(PAGE_SHIFT as u32);
        let phys = ((*vma).vm_private_data as u64).wrapping_add(byte_off);

        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => return VM_FAULT_OOM,
        };

        // `remap_pfn_range()` uses the VMA's page protection unchanged and
        // marks the PTE special. In particular, do not grant write access or
        // select a cache mode that the mmap callback did not request.
        let prot = pgprot_t((*vma).vm_page_prot);
        let pfn = phys >> PAGE_SHIFT;
        let entry = pte_mkspecial(pfn_pte(pfn, prot));
        set_pte_at((*vma).vm_mm as *mut (), (*vmf).address, ptep, entry);
        (*vmf).pte = ptep;
        VM_FAULT_NOPAGE
    }
}

pub unsafe extern "C" fn filemap_fault(vmf: *mut VmFault) -> VmFaultFlags {
    use crate::mm::address_space::{page_uptodate, unlock_page, wait_on_page_locked};
    use crate::mm::filemap::{filemap_grab_folio, find_lock_page};

    if vmf.is_null() {
        return VM_FAULT_SIGBUS;
    }

    unsafe {
        let vma = (*vmf).vma;
        if vma.is_null() {
            return VM_FAULT_SIGBUS;
        }

        let mapping = (*vma).vm_file as *mut AddressSpace;
        if mapping.is_null() {
            return VM_FAULT_SIGBUS;
        }

        // Compute the page index within the file.
        let index = (*vmf).pgoff;

        // Get the page — fast path from cache, slow path via allocation.
        let page = {
            let cached = find_lock_page(mapping, index);
            if !cached.is_null() {
                cached
            } else {
                filemap_grab_folio(mapping, index)
            }
        };

        if page.is_null() {
            return VM_FAULT_OOM;
        }

        // If not uptodate, ask the filesystem to fill the page.
        if !page_uptodate(page) {
            let a_ops = (*mapping).a_ops;
            if !a_ops.is_null() {
                if let Some(read_fn) = (*a_ops).read_folio {
                    let err = read_fn(mapping, page);
                    if err != 0 {
                        unlock_page(page);
                        (*page).put_page();
                        return VM_FAULT_SIGBUS;
                    }
                    wait_on_page_locked(page);
                    // Re-lock after waiting.
                    crate::mm::address_space::lock_page(page);
                }
            }
        }

        if !page_uptodate(page) {
            unlock_page(page);
            (*page).put_page();
            return VM_FAULT_SIGBUS;
        }

        // Install a read-only PTE for the page (COW-ready).
        // The caller (handle_pte_fault) will unlock the page when done.
        let mm = (*vma).vm_mm;
        let ptep = match pte_alloc((*vmf).pmd, (*vmf).address, _PAGE_TABLE) {
            Some(p) => p,
            None => {
                unlock_page(page);
                (*page).put_page();
                return VM_FAULT_OOM;
            }
        };

        let pfn = page_to_pfn(page) as u64;
        let prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX)
        };
        let entry = pte_mkyoung(pfn_pte(pfn, prot)); // read-only by default
        set_pte_at(mm as *mut (), (*vmf).address, ptep, entry);
        (*page)._mapcount().fetch_add(1, Ordering::Relaxed);

        (*vmf).page = page;
        (*vmf).pte = ptep;

        add_mm_rss(mm, 1);

        // Page remains locked — caller is responsible for unlock_page.
        VM_FAULT_LOCKED
    }
}

// ---------------------------------------------------------------------------
// do_fault — file-backed fault dispatcher
// ---------------------------------------------------------------------------

/// Handle a fault on a file-backed VMA.
///
/// Dispatches like Linux `do_fault()`: read faults call `->fault` directly,
/// private write faults use the COW path, and shared write faults call the
/// shared fault path.
///
/// Ref: Linux `mm/memory.c` — `do_fault()` line 5903
fn do_fault(vmf: &mut VmFault) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        let vm_ops = (*vma).vm_ops as *const VmOperationsStruct;
        if vm_ops.is_null() {
            return VM_FAULT_SIGBUS;
        }
        let Some(fault_fn) = (*vm_ops).fault else {
            return VM_FAULT_SIGBUS;
        };

        if (vmf.flags & FAULT_FLAG_WRITE) == 0 {
            return fault_fn(vmf as *mut VmFault);
        }

        if (*vma).vm_flags & VM_SHARED == 0 {
            return do_cow_file_fault(vmf, fault_fn);
        }

        fault_fn(vmf as *mut VmFault)
    }
}

fn do_cow_file_fault(
    vmf: &mut VmFault,
    fallback_fault: unsafe extern "C" fn(*mut VmFault) -> VmFaultFlags,
) -> VmFaultFlags {
    unsafe {
        let vma = vmf.vma;
        if (*vma).vm_ops == &LUPOS_FILE_VM_OPS as *const VmOperationsStruct as usize {
            return lupos_file_cow_fault(vmf as *mut VmFault);
        }

        // Lupos module/device callbacks still use the local "callback installs
        // its own PTE" contract. Preserve that path until those callbacks are
        // converted to Linux's generic finish_fault() contract.
        fallback_fault(vmf as *mut VmFault)
    }
}

// ---------------------------------------------------------------------------
// do_swap_page — swap-in (M17)
// ---------------------------------------------------------------------------

/// Handle a fault on a swapped-out page.
///
/// `vmf.orig_pte` contains a non-present swap PTE.  This function:
/// 1. Decodes the `SwpEntry` from the PTE.
/// 2. Looks up the swap cache (fast path: page already read in).
/// 3. On cache miss: allocates a fresh page and calls `swap_readpage`.
/// 4. Installs a new present PTE, replaces the swap PTE, flushes TLB.
/// 5. Removes from swap cache and frees the swap slot.
///
/// Returns `VM_FAULT_MAJOR` on success (I/O performed or cache hit with
/// major-fault semantics), or a `VM_FAULT_*` error flag.
///
/// Ref: Linux `mm/memory.c` — `do_swap_page()` line 4013
fn do_swap_page(vmf: &mut VmFault) -> VmFaultFlags {
    use crate::arch::x86::mm::paging::is_swap_pte;
    use crate::mm::swap::{
        PTE_MARKER_GUARD, PTE_MARKER_POISONED, free_swap_slot, pte_marker_flags, pte_to_swp_entry,
        swap_cache_add, swap_cache_delete, swap_cache_get, swap_readpage, swp_entry_to_pte,
    };

    // Verify this is actually a swap PTE (not zero, not present).
    if !is_swap_pte(vmf.orig_pte) {
        return VM_FAULT_SIGBUS;
    }

    if let Some(marker) = pte_marker_flags(vmf.orig_pte) {
        if marker == 0 {
            return VM_FAULT_SIGBUS;
        }
        if marker & PTE_MARKER_POISONED != 0 {
            return VM_FAULT_HWPOISON;
        }
        if marker & PTE_MARKER_GUARD != 0 {
            return VM_FAULT_SIGSEGV;
        }
        return VM_FAULT_SIGBUS;
    }

    let entry = pte_to_swp_entry(vmf.orig_pte);
    if entry.is_null() {
        return VM_FAULT_SIGBUS;
    }

    unsafe {
        let vma = vmf.vma;
        let mm = (*vma).vm_mm;

        // --- Step 1: look up the swap cache ---
        let page = match swap_cache_get(entry) {
            Some(p) => p,
            None => {
                // Cache miss: allocate a new page and read from swap.
                let new_page = match with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) {
                    Some(p) => p,
                    None => return VM_FAULT_OOM,
                };
                (*new_page).get_page();

                if swap_readpage(new_page, entry) != 0 {
                    (*new_page).put_page();
                    with_global_buddy(|b| b.free_pages(new_page, 0));
                    return VM_FAULT_SIGBUS;
                }

                // Add to swap cache so concurrent faults on the same entry
                // see this page rather than reading it again.
                swap_cache_add(new_page, entry);
                new_page
            }
        };

        // --- Step 2: build the new present PTE ---
        let pfn = page_to_pfn(page as *const _) as u64;
        let base_prot = if (*vma).vm_page_prot != 0 {
            pgprot_t((*vma).vm_page_prot)
        } else {
            pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED)
        };
        let mut new_pte = pfn_pte(pfn, base_prot);
        new_pte = pte_mkyoung(new_pte);
        if (vmf.flags & FAULT_FLAG_WRITE) != 0 && ((*vma).vm_flags & VM_WRITE) != 0 {
            new_pte = pte_mkwrite(pte_mkdirty(new_pte));
        }

        // --- Step 3: install the PTE (vmf.pte already points to the slot) ---
        let ptep = if !vmf.pte.is_null() {
            vmf.pte
        } else {
            // Fallback: get the PTE pointer from the PMD.
            match pte_alloc(vmf.pmd, vmf.address, _PAGE_TABLE) {
                Some(p) => p,
                None => {
                    swap_cache_delete(page);
                    free_swap_slot(entry);
                    (*page).put_page();
                    return VM_FAULT_OOM;
                }
            }
        };

        // Atomically clear the swap PTE and install the real PTE.
        ptep_get_and_clear(mm as *mut (), vmf.address, ptep);
        set_pte_at(mm as *mut (), vmf.address, ptep, new_pte);
        flush_tlb_page(vmf.address);
        (*page)._mapcount().fetch_add(1, Ordering::Relaxed);

        // --- Step 4: clean up swap slot ---
        swap_cache_delete(page);
        free_swap_slot(entry);

        // Update RSS.
        add_mm_rss(mm, 1);

        vmf.pte = ptep;
        vmf.page = page;

        VM_FAULT_MAJOR
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging;
    use crate::mm::vm_flags::{VM_MAYSHARE, VM_MAYWRITE, VM_READ};

    // ── Constant parity with Linux ───────────────────────────────────────────

    #[test]
    fn vm_fault_codes_match_linux() {
        assert_eq!(VM_FAULT_OOM, 0x0001);
        assert_eq!(VM_FAULT_SIGBUS, 0x0002);
        assert_eq!(VM_FAULT_MAJOR, 0x0004);
        assert_eq!(VM_FAULT_HWPOISON, 0x0010);
        assert_eq!(VM_FAULT_HWPOISON_LARGE, 0x0020);
        assert_eq!(VM_FAULT_SIGSEGV, 0x0040);
        assert_eq!(VM_FAULT_NOPAGE, 0x0100);
        assert_eq!(VM_FAULT_LOCKED, 0x0200);
        assert_eq!(VM_FAULT_RETRY, 0x0400);
        assert_eq!(VM_FAULT_FALLBACK, 0x0800);
        assert_eq!(VM_FAULT_DONE_COW, 0x1000);
        assert_eq!(VM_FAULT_NEEDDSYNC, 0x2000);
        assert_eq!(VM_FAULT_COMPLETED, 0x4000);
    }

    #[test]
    fn vm_fault_error_mask_is_correct() {
        // All error conditions are in the mask.
        assert_ne!(VM_FAULT_ERROR & VM_FAULT_OOM, 0);
        assert_ne!(VM_FAULT_ERROR & VM_FAULT_SIGBUS, 0);
        assert_ne!(VM_FAULT_ERROR & VM_FAULT_SIGSEGV, 0);
        assert_ne!(VM_FAULT_ERROR & VM_FAULT_HWPOISON, 0);
        assert_ne!(VM_FAULT_ERROR & VM_FAULT_HWPOISON_LARGE, 0);
        // Non-error outcomes must NOT appear in the error mask.
        assert_eq!(VM_FAULT_ERROR & VM_FAULT_MAJOR, 0);
        assert_eq!(VM_FAULT_ERROR & VM_FAULT_RETRY, 0);
        assert_eq!(VM_FAULT_ERROR & VM_FAULT_NOPAGE, 0);
        assert_eq!(VM_FAULT_ERROR & VM_FAULT_DONE_COW, 0);
    }

    #[test]
    fn fault_flag_values_match_linux() {
        assert_eq!(FAULT_FLAG_WRITE, 1 << 0);
        assert_eq!(FAULT_FLAG_MKWRITE, 1 << 1);
        assert_eq!(FAULT_FLAG_ALLOW_RETRY, 1 << 2);
        assert_eq!(FAULT_FLAG_RETRY_NOWAIT, 1 << 3);
        assert_eq!(FAULT_FLAG_KILLABLE, 1 << 4);
        assert_eq!(FAULT_FLAG_TRIED, 1 << 5);
        assert_eq!(FAULT_FLAG_USER, 1 << 6);
        assert_eq!(FAULT_FLAG_REMOTE, 1 << 7);
        assert_eq!(FAULT_FLAG_INSTRUCTION, 1 << 8);
        assert_eq!(FAULT_FLAG_INTERRUPTIBLE, 1 << 9);
        assert_eq!(FAULT_FLAG_UNSHARE, 1 << 10);
        assert_eq!(FAULT_FLAG_ORIG_PTE_VALID, 1 << 11);
        assert_eq!(FAULT_FLAG_VMA_LOCK, 1 << 12);
    }

    #[test]
    fn fault_flag_default_composition() {
        assert_ne!(FAULT_FLAG_DEFAULT & FAULT_FLAG_ALLOW_RETRY, 0);
        assert_ne!(FAULT_FLAG_DEFAULT & FAULT_FLAG_KILLABLE, 0);
        assert_ne!(FAULT_FLAG_DEFAULT & FAULT_FLAG_INTERRUPTIBLE, 0);
        // Default must NOT include WRITE or USER.
        assert_eq!(FAULT_FLAG_DEFAULT & FAULT_FLAG_WRITE, 0);
        assert_eq!(FAULT_FLAG_DEFAULT & FAULT_FLAG_USER, 0);
    }

    /// Linux `wp_page_copy()` clears and flushes the old PTE before publishing
    /// the replacement PTE, so no CPU can retain the old read-only translation
    /// while another observes the new writable mapping.
    ///
    /// test-origin: linux:vendor/linux/mm/memory.c:wp_page_copy
    #[test]
    fn wp_page_copy_clears_and_flushes_before_installing_copy() {
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/memory.c"
        ));
        let linux_clear = linux
            .find("ptep_clear_flush(vma, vmf->address, vmf->pte);")
            .expect("Linux wp_page_copy must clear+flush the old PTE");
        let linux_set = linux
            .find("set_pte_at(mm, vmf->address, vmf->pte, entry);")
            .expect("Linux wp_page_copy must install the copied PTE");
        assert!(
            linux_clear < linux_set,
            "Linux must clear+flush before installing the copied PTE"
        );

        let lupos = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mm/fault.rs"));
        let clear = lupos
            .find("ptep_get_and_clear(mm as *mut (), vmf.address, vmf.pte);")
            .expect("Lupos wp_page_copy must clear the old PTE");
        let flush = lupos[clear..]
            .find("flush_tlb_page(vmf.address);")
            .map(|offset| clear + offset)
            .expect("Lupos wp_page_copy must flush after clearing the old PTE");
        let set = lupos[clear..]
            .find("set_pte_at(mm as *mut (), vmf.address, vmf.pte, new_entry);")
            .map(|offset| clear + offset)
            .expect("Lupos wp_page_copy must install the copied PTE");

        assert!(
            clear < flush && flush < set,
            "Lupos wp_page_copy must mirror Linux ptep_clear_flush before set_pte_at"
        );
    }

    // ── Fork PTE policy ──────────────────────────────────────────────────────

    #[test]
    fn fork_present_pte_write_protects_only_cow_mappings() {
        // Linux `is_cow_mapping()` keys off VM_MAYWRITE, not current VM_WRITE.
        let src = pte_mkyoung(pte_mkdirty(pte_mkwrite(__pte(_PAGE_PRESENT))));

        let (source_update, child) = fork_present_pte(src, VM_WRITE | VM_MAYWRITE);
        let parent = source_update.expect("writable COW source must be downgraded");
        assert_eq!(parent, pte_wrprotect(src));
        assert!(!pte_write(parent));
        assert!(!pte_write(child));
        assert!(
            paging::pte_dirty(child),
            "private child retains the source dirty state"
        );
        assert!(!paging::pte_young(child), "every child PTE starts old");

        let (source_update, child) = fork_present_pte(src, VM_WRITE);
        assert!(
            source_update.is_none(),
            "VM_WRITE without VM_MAYWRITE is not a Linux COW mapping"
        );
        assert!(
            pte_write(child),
            "non-COW child retains the source write permission"
        );
    }

    #[test]
    fn fork_present_pte_keeps_shared_mapping_writable_but_clean_and_old() {
        let src = pte_mkyoung(pte_mkdirty(pte_mkwrite(__pte(_PAGE_PRESENT))));
        let flags = VM_SHARED | VM_WRITE | VM_MAYWRITE | VM_MAYSHARE;

        let (source_update, child) = fork_present_pte(src, flags);

        assert!(
            source_update.is_none(),
            "shared source PTE must not be write-protected"
        );
        assert!(pte_write(child), "shared child must stay writable");
        assert!(!paging::pte_dirty(child), "shared child starts clean");
        assert!(!paging::pte_young(child), "shared child starts old");
    }

    /// test-origin: linux:vendor/linux/mm/memory.c:vma_needs_copy
    #[test]
    fn vma_needs_copy_skips_fault_reconstructable_mappings() {
        let mut src = VmAreaStruct::new(0x1000, 0x3000, VM_READ);
        let dst = VmAreaStruct::new(0x1000, 0x3000, VM_READ);

        assert!(
            !vma_needs_copy(&dst, &src),
            "no anon_vma and no copy-on-fork flags means child can fault lazily"
        );

        src.vm_file = 0x1234;
        assert!(
            !vma_needs_copy(&dst, &src),
            "file-backed mappings without copy-on-fork metadata are fault-reconstructable"
        );
    }

    /// test-origin: linux:vendor/linux/mm/memory.c:vma_needs_copy
    #[test]
    fn vma_needs_copy_keeps_anon_or_raw_pfn_metadata() {
        let src = VmAreaStruct::new(0x1000, 0x3000, VM_READ);
        let dst_pfnmap = VmAreaStruct::new(0x1000, 0x3000, VM_READ | VM_PFNMAP);
        let dst_mixed = VmAreaStruct::new(0x1000, 0x3000, VM_READ | VM_MIXEDMAP);

        assert!(vma_needs_copy(&dst_pfnmap, &src));
        assert!(vma_needs_copy(&dst_mixed, &src));

        let mut src_anon = VmAreaStruct::new(0x1000, 0x3000, VM_READ);
        let dst = VmAreaStruct::new(0x1000, 0x3000, VM_READ);
        src_anon.anon_vma = core::ptr::NonNull::<crate::mm::rmap::AnonVma>::dangling().as_ptr();

        assert!(vma_needs_copy(&dst, &src_anon));
    }

    // ── Routing helpers ──────────────────────────────────────────────────────

    #[test]
    fn vma_is_anonymous_no_ops() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        assert!(vma_is_anonymous(&vma));
    }

    #[test]
    fn vma_is_anonymous_with_ops() {
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        vma.vm_ops = 0xDEAD_BEEF; // non-zero ⇒ file-backed
        assert!(!vma_is_anonymous(&vma));
    }

    // ── Stub behaviour ───────────────────────────────────────────────────────

    #[test]
    fn do_fault_null_fault_fn_returns_sigbus() {
        // A file-backed VMA whose vtable has no fault callback → SIGBUS.
        static NO_FAULT_OPS: VmOperationsStruct = VmOperationsStruct {
            open: None,
            close: None,
            fault: None,
            map_pages: None,
            pfn_mkwrite: None,
            access: None,
        };
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        vma.vm_ops = &NO_FAULT_OPS as *const VmOperationsStruct as usize;
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        assert_eq!(do_fault(&mut vmf), VM_FAULT_SIGBUS);
    }

    #[test]
    fn do_fault_dispatches_to_vm_ops_fault() {
        unsafe extern "C" fn my_fault(_vmf: *mut VmFault) -> VmFaultFlags {
            0xCAFE // sentinel return value
        }
        static MY_OPS: VmOperationsStruct = VmOperationsStruct {
            open: None,
            close: None,
            fault: Some(my_fault),
            map_pages: None,
            pfn_mkwrite: None,
            access: None,
        };
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        vma.vm_ops = &MY_OPS as *const VmOperationsStruct as usize;
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        assert_eq!(do_fault(&mut vmf), 0xCAFE);
    }

    #[test]
    fn do_fault_returns_sigbus_when_vm_ops_ptr_is_null() {
        // vm_ops = 0 → null → SIGBUS
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        vma.vm_ops = 0;
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        assert_eq!(do_fault(&mut vmf), VM_FAULT_SIGBUS);
    }

    #[test]
    fn do_swap_page_rejects_non_swap_pte() {
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        assert_eq!(do_swap_page(&mut vmf), VM_FAULT_SIGBUS);
    }

    #[test]
    fn do_swap_page_routes_guard_marker_to_sigsegv() {
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        vmf.orig_pte = crate::mm::swap::make_pte_marker(crate::mm::swap::PTE_MARKER_GUARD);

        assert_eq!(do_swap_page(&mut vmf), VM_FAULT_SIGSEGV);
    }

    #[test]
    fn do_pte_missing_routes_to_anonymous() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_SHARED | VM_WRITE);
        assert!(vma_is_shared_anonymous(&vma));
    }

    #[test]
    fn do_pte_missing_routes_to_file() {
        static FILE_OPS: VmOperationsStruct = VmOperationsStruct {
            open: None,
            close: None,
            fault: None,
            map_pages: None,
            pfn_mkwrite: None,
            access: None,
        };
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        vma.vm_ops = &FILE_OPS as *const VmOperationsStruct as usize;
        let mut vmf = make_vmf(&mut vma, 0x1000, 0);
        // Route: file-backed → do_fault → fault=None → SIGBUS
        assert_eq!(do_pte_missing(&mut vmf), VM_FAULT_SIGBUS);
    }

    /// Linux `do_fault()` routes first writes to private file mappings through
    /// `do_cow_fault()`: the installed PTE maps an anonymous copy, and the VMA
    /// gains anon metadata so `copy_page_range()` must preserve it across fork.
    ///
    /// test-origin: linux:vendor/linux/mm/memory.c:do_cow_fault
    #[test]
    fn private_file_write_fault_installs_anon_copy_for_fork() {
        use crate::fs::dcache::{d_alloc, d_instantiate};
        use crate::fs::file::alloc_file;
        use crate::fs::ops::{FileOps, NOOP_INODE_OPS};
        use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate};
        use crate::mm::buddy;
        use crate::mm::filemap::find_lock_page;
        use crate::mm::mm_types::MmStruct;
        use crate::mm::page::Page;
        use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
        use crate::mm::vma::{vma_file_from_ref, vma_file_put_raw};

        fn cow_file_read(_file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
            for (index, byte) in buf.iter_mut().enumerate() {
                *byte = 0x40u8.wrapping_add((index + *pos as usize) as u8);
            }
            *pos = (*pos).saturating_add(buf.len() as u64);
            Ok(buf.len())
        }

        static COW_FILE_OPS: FileOps = FileOps {
            name: "private-file-cow-test",
            read: Some(cow_file_read),
            write: None,
            llseek: None,
            fsync: None,
            poll: None,
            ioctl: None,
            mmap: None,
            release: None,
            readdir: None,
        };

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let dentry = d_alloc("private-file-cow");
        let inode = Inode::new(
            1,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &COW_FILE_OPS,
            InodePrivate::None,
        );
        inode
            .size
            .store(PAGE_SIZE, core::sync::atomic::Ordering::Release);
        d_instantiate(&dentry, inode.clone());
        let file = alloc_file(dentry, 0, 0, &COW_FILE_OPS);
        let raw_file = vma_file_from_ref(file.clone());

        let test_addr = 0x00d0_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let flags = VM_READ | VM_WRITE | VM_MAYWRITE;
        let mut vma = VmAreaStruct::new(test_addr, test_addr + PAGE_SIZE, flags);
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_file = raw_file;
        vma.vm_ops = &LUPOS_FILE_VM_OPS as *const VmOperationsStruct as usize;
        vma.vm_page_prot = crate::mm::pgprot::vm_get_page_prot(flags);

        let ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(ret, 0, "private file write fault should succeed");
        assert!(
            !vma.anon_vma.is_null(),
            "private file write fault must attach anon_vma for fork"
        );

        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            paging::pte_offset_kernel(pmdp, test_addr)
        };
        let pte = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_present(pte));
        assert!(paging::pte_write(pte));
        assert!(paging::pte_dirty(pte));

        let anon_page = buddy::pfn_to_page(paging::pte_pfn(pte) as usize);
        assert_eq!(unsafe { (*anon_page).mapping }, vma.anon_vma as usize);
        assert!(
            unsafe { (*anon_page).test_flag(PG_SWAPBACKED) },
            "private file COW page must be anonymous/swap-backed"
        );

        let copied = unsafe { core::slice::from_raw_parts(fault_page_kaddr(anon_page), 16) };
        let expected: [u8; 16] = core::array::from_fn(|index| 0x40u8.wrapping_add(index as u8));
        assert_eq!(copied, expected.as_slice());

        let file_page = unsafe { find_lock_page(inode.mapping(), 0) };
        assert!(
            !file_page.is_null(),
            "source file page remains in page cache"
        );
        assert_ne!(
            paging::pte_pfn(pte),
            buddy::page_to_pfn(file_page) as u64,
            "private file write fault must not map the page-cache folio"
        );
        unsafe {
            crate::mm::address_space::unlock_page(file_page);
            (*file_page).put_page();
        }

        let dst = VmAreaStruct::new(test_addr, test_addr + PAGE_SIZE, flags);
        assert!(
            vma_needs_copy(&dst, &vma),
            "fork must copy PTEs after private file COW metadata is attached"
        );

        unsafe {
            vma_file_put_raw(raw_file);
        }
    }

    /// Linux `do_wp_page()` must copy a private file-backed PTE into an
    /// anonymous folio on the first write, even when the file page was installed
    /// by an earlier read fault.
    ///
    /// test-origin: linux:vendor/linux/mm/memory.c:do_wp_page
    #[test]
    fn private_file_read_then_write_fault_installs_anon_copy_for_fork() {
        use crate::fs::dcache::{d_alloc, d_instantiate};
        use crate::fs::file::alloc_file;
        use crate::fs::ops::{FileOps, NOOP_INODE_OPS};
        use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate};
        use crate::mm::buddy;
        use crate::mm::mm_types::MmStruct;
        use crate::mm::page::Page;
        use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
        use crate::mm::vma::{vma_file_from_ref, vma_file_put_raw};

        fn cow_file_read(_file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
            for (index, byte) in buf.iter_mut().enumerate() {
                *byte = 0x60u8.wrapping_add((index + *pos as usize) as u8);
            }
            *pos = (*pos).saturating_add(buf.len() as u64);
            Ok(buf.len())
        }

        static COW_FILE_OPS: FileOps = FileOps {
            name: "private-file-read-then-write-cow-test",
            read: Some(cow_file_read),
            write: None,
            llseek: None,
            fsync: None,
            poll: None,
            ioctl: None,
            mmap: None,
            release: None,
            readdir: None,
        };

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let dentry = d_alloc("private-file-read-then-write-cow");
        let inode = Inode::new(
            1,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &COW_FILE_OPS,
            InodePrivate::None,
        );
        inode
            .size
            .store(PAGE_SIZE, core::sync::atomic::Ordering::Release);
        d_instantiate(&dentry, inode.clone());
        let file = alloc_file(dentry, 0, 0, &COW_FILE_OPS);
        let raw_file = vma_file_from_ref(file.clone());

        let test_addr = 0x00d2_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let flags = VM_READ | VM_WRITE | VM_MAYWRITE;
        let mut vma = VmAreaStruct::new(test_addr, test_addr + PAGE_SIZE, flags);
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_file = raw_file;
        vma.vm_ops = &LUPOS_FILE_VM_OPS as *const VmOperationsStruct as usize;
        vma.vm_page_prot = crate::mm::pgprot::vm_get_page_prot(flags);

        let read_ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_USER);
        assert_eq!(read_ret, 0, "private file read fault should succeed");
        assert!(
            vma.anon_vma.is_null(),
            "read-only private file fault remains fault-reconstructable"
        );

        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            paging::pte_offset_kernel(pmdp, test_addr)
        };
        let read_pte = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_present(read_pte));
        assert!(
            !paging::pte_write(read_pte),
            "private file read fault must leave PTE write-protected"
        );
        let file_page = buddy::pfn_to_page(paging::pte_pfn(read_pte) as usize);
        assert_eq!(unsafe { (*file_page).mapping }, inode.mapping() as usize);
        assert!(
            !unsafe { (*file_page).test_flag(PG_SWAPBACKED) },
            "read fault initially maps the file-cache page"
        );

        let write_ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(
            write_ret, VM_FAULT_DONE_COW,
            "private file write-protect fault must run wp_page_copy"
        );
        assert!(
            !vma.anon_vma.is_null(),
            "write-protect COW must attach anon_vma for fork"
        );

        let cow_pte = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_present(cow_pte));
        assert!(paging::pte_write(cow_pte));
        assert!(paging::pte_dirty(cow_pte));
        assert_ne!(
            paging::pte_pfn(cow_pte),
            paging::pte_pfn(read_pte),
            "private file write fault must replace the file page with an anon copy"
        );

        let anon_page = buddy::pfn_to_page(paging::pte_pfn(cow_pte) as usize);
        assert_eq!(unsafe { (*anon_page).mapping }, vma.anon_vma as usize);
        assert!(
            unsafe { (*anon_page).test_flag(PG_SWAPBACKED) },
            "wp_page_copy result must be anonymous/swap-backed"
        );
        let copied = unsafe { core::slice::from_raw_parts(fault_page_kaddr(anon_page), 16) };
        let expected: [u8; 16] = core::array::from_fn(|index| 0x60u8.wrapping_add(index as u8));
        assert_eq!(copied, expected.as_slice());

        let dst = VmAreaStruct::new(test_addr, test_addr + PAGE_SIZE, flags);
        assert!(
            vma_needs_copy(&dst, &vma),
            "fork must preserve private file COW PTEs after write-protect COW"
        );

        unsafe {
            vma_file_put_raw(raw_file);
        }
    }

    /// Lupos-specific regression for the Rust translation of Linux
    /// `do_wp_page()`: Linux only reuses anonymous exclusive pages. A private
    /// file-backed page must be copied even if its local refcount looks
    /// exclusive.
    ///
    /// test-origin: lupos-specific: mirrors linux:vendor/linux/mm/memory.c:do_wp_page
    #[test]
    fn private_file_wp_fault_does_not_reuse_exclusive_file_page() {
        use crate::mm::buddy;
        use crate::mm::mm_types::MmStruct;
        use crate::mm::page::Page;
        use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr = 0x00d4_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let flags = VM_READ | VM_WRITE | VM_MAYWRITE;
        let mut vma = VmAreaStruct::new(test_addr, test_addr + PAGE_SIZE, flags);
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_file = 0x1234;
        vma.vm_page_prot = crate::mm::pgprot::vm_get_page_prot(flags);

        let source_page = unsafe {
            buddy::with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL))
                .expect("source page allocation")
        };
        unsafe {
            (*source_page)._refcount.store(1, Ordering::Relaxed);
            (*source_page)._mapcount().store(0, Ordering::Relaxed);
            (*source_page).mapping = 0xCAFE_0000;
            (*source_page).index = 0;
            (*source_page).init_lru();
            let src = fault_page_kaddr(source_page);
            for index in 0..16 {
                *src.add(index) = 0x90u8.wrapping_add(index as u8);
            }
        }

        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp =
                pud_alloc(p4dp as *mut pgd_t, test_addr, _PAGE_TABLE).expect("test PUD allocation");
            let pmdp = pmd_alloc(pudp, test_addr, _PAGE_TABLE).expect("test PMD allocation");
            pte_alloc(pmdp, test_addr, _PAGE_TABLE).expect("test PTE allocation")
        };
        let source_pte = unsafe {
            let prot = pgprot_t(vma.vm_page_prot);
            pte_wrprotect(pte_mkyoung(pfn_pte(
                buddy::page_to_pfn(source_page) as u64,
                prot,
            )))
        };
        unsafe {
            set_pte_at(mm.pgd as *mut (), test_addr, ptep, source_pte);
        }

        let ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(
            ret, VM_FAULT_DONE_COW,
            "private file PTE must copy instead of reusing by refcount alone"
        );

        let cow_pte = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_write(cow_pte));
        assert_ne!(
            paging::pte_pfn(cow_pte),
            buddy::page_to_pfn(source_page) as u64,
            "file-backed private page must not be promoted in-place"
        );
        assert!(!vma.anon_vma.is_null());

        let anon_page = buddy::pfn_to_page(paging::pte_pfn(cow_pte) as usize);
        assert_eq!(unsafe { (*anon_page).mapping }, vma.anon_vma as usize);
        assert!(
            unsafe { (*anon_page).test_flag(PG_SWAPBACKED) },
            "copied private page must be anonymous/swap-backed"
        );
        let copied = unsafe { core::slice::from_raw_parts(fault_page_kaddr(anon_page), 16) };
        let expected: [u8; 16] = core::array::from_fn(|index| 0x90u8.wrapping_add(index as u8));
        assert_eq!(copied, expected.as_slice());
    }

    // ── Integration: full anonymous write fault through the page-table walk ──
    //
    // These tests use the paging test pool for page-table pages AND the buddy
    // allocator for the user page.  Both require serialisation because they
    // share global state.

    use crate::mm::buddy;
    use crate::mm::mm_types::MmStruct;
    use crate::mm::page::Page;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    extern crate alloc;
    extern crate std;
    use alloc::boxed::Box;

    const TEST_PAGES: usize = 256;

    #[test]
    fn handle_mm_fault_write_installs_writable_pte() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // --- Set up the buddy allocator with a small test mem_map ---
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };

        // --- Set up the paging test pool for page-table pages ---
        unsafe { paging::reset_test_pool() };

        // --- Build a test mm_struct ---
        let test_addr: u64 = 0x0040_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE,
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        // --- Fire a write fault ---
        let ret = handle_mm_fault(
            &mut vma as *mut VmAreaStruct,
            test_addr,
            FAULT_FLAG_WRITE | FAULT_FLAG_USER,
        );
        assert_eq!(ret, 0, "write fault should succeed (ret=0)");

        // --- Walk the page tables and verify the PTE ---
        let pte = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            let ptep = paging::pte_offset_kernel(pmdp, test_addr);
            paging::ptep_get(ptep)
        };

        assert!(paging::pte_present(pte), "PTE must be present");
        assert!(
            paging::pte_write(pte),
            "PTE must be writable after write fault"
        );
        assert!(
            paging::pte_dirty(pte),
            "PTE must be dirty after write fault"
        );
        assert!(paging::pte_young(pte), "PTE must be young (accessed)");
        assert_eq!(mm.hiwater_rss, 1, "RSS must be 1 after one fault");
    }

    #[test]
    fn handle_mm_fault_rebuilds_invalid_present_pte() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x0060_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE,
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        let invalid_pfn = (TEST_PAGES + 4096) as u64;
        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_alloc(p4dp as *mut pgd_t, test_addr, _PAGE_TABLE)
                .expect("PUD allocation must succeed");
            let pmdp = paging::pmd_alloc(pudp, test_addr, _PAGE_TABLE)
                .expect("PMD allocation must succeed");
            paging::pte_alloc(pmdp, test_addr, _PAGE_TABLE).expect("PTE allocation must succeed")
        };
        unsafe {
            paging::set_pte_at(
                mm.pgd as *mut (),
                test_addr,
                ptep,
                paging::pfn_pte(
                    invalid_pfn,
                    pgprot_t(_PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX),
                ),
            );
        }

        let ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(ret, 0, "invalid present PTE should be rebuilt");

        let rebuilt = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_present(rebuilt), "rebuilt PTE must be present");
        assert_ne!(
            paging::pte_pfn(rebuilt),
            invalid_pfn,
            "rebuilt PTE must not keep the invalid PFN"
        );
        assert!(
            buddy::pfn_valid(paging::pte_pfn(rebuilt) as usize),
            "rebuilt PTE must point at a valid PFN"
        );
    }

    #[test]
    fn handle_mm_fault_read_installs_readonly_pte() {
        let _g = crate::mm::test_lock::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x0080_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE, // VMA writable, but this is a read fault
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        // Read fault — no FAULT_FLAG_WRITE.
        let ret = handle_mm_fault(&mut vma as *mut VmAreaStruct, test_addr, FAULT_FLAG_USER);
        assert_eq!(ret, 0);

        let pte = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            let ptep = paging::pte_offset_kernel(pmdp, test_addr);
            paging::ptep_get(ptep)
        };

        assert!(paging::pte_present(pte), "PTE must be present");
        assert!(
            !paging::pte_write(pte),
            "read fault must not produce writable PTE"
        );
        assert!(!paging::pte_dirty(pte), "read fault must not set dirty bit");
        assert!(paging::pte_young(pte), "PTE must be young");
        assert_eq!(mm.hiwater_rss, 1);
    }

    #[test]
    fn handle_mm_fault_shared_anonymous_reuses_same_page_across_mms() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x0090_0000;
        let flags = VM_SHARED | VM_WRITE;
        let prot = crate::mm::pgprot::vm_get_page_prot(flags);

        let mut mm_a = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma_a = VmAreaStruct::new(
            test_addr,
            test_addr + crate::mm::frame::PAGE_SIZE as u64,
            flags,
        );
        vma_a.vm_mm = &mut mm_a as *mut MmStruct;
        vma_a.vm_page_prot = prot;

        let ret_a = handle_mm_fault(&mut vma_a, test_addr, FAULT_FLAG_USER);
        assert_eq!(ret_a, 0, "first shared-anon fault should succeed");

        let pte_a = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm_a.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            let ptep = paging::pte_offset_kernel(pmdp, test_addr);
            paging::ptep_get(ptep)
        };
        assert!(
            paging::pte_present(pte_a),
            "first shared-anon PTE must be present"
        );
        assert!(
            paging::pte_write(pte_a),
            "shared writable PTE must be writable on first fault"
        );

        let mut mm_b = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma_b = VmAreaStruct::new(
            test_addr,
            test_addr + crate::mm::frame::PAGE_SIZE as u64,
            flags,
        );
        vma_b.vm_mm = &mut mm_b as *mut MmStruct;
        vma_b.vm_page_prot = prot;
        vma_b.vm_private_data = vma_a.vm_private_data;

        let ret_b = handle_mm_fault(&mut vma_b, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(
            ret_b, 0,
            "second shared-anon fault should reuse shared backing"
        );

        let pte_b = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm_b.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            let ptep = paging::pte_offset_kernel(pmdp, test_addr);
            paging::ptep_get(ptep)
        };

        assert_eq!(
            paging::pte_pfn(pte_a),
            paging::pte_pfn(pte_b),
            "shared-anon faults in two mm structs must map the same page",
        );
        assert!(
            paging::pte_write(pte_b),
            "shared-anon write fault must stay writable"
        );
    }

    // ── COW path: do_wp_page_reuse (exclusive page → promote writable) ────────
    //
    // This tests the "wp_page_reuse" path of do_wp_page: when a write-fault
    // hits a read-only PTE that maps an *exclusive* page (refcount ≤ 1), the
    // kernel upgrades the PTE in-place without allocating a new page.
    // This is the fast path that fires on the first write after fork when only
    // one mm still references the page.
    //
    // The "wp_page_copy" path (shared page → allocate private copy) requires
    // `pfn_to_virt()` to copy the page contents, which in test mode returns raw
    // physical addresses that are not accessible from userspace.  That path is
    // therefore tested only in QEMU (see `anon_mmap_boots_in_qemu`).

    #[test]
    fn do_wp_page_reuse_upgrades_exclusive_pte_to_writable() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // --- Set up allocator + page-table pool ---
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x00c0_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE,
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        // Step 1: Install the page via a write fault (PTE ends up writable + dirty).
        let ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(ret, 0, "initial write fault should succeed");

        // Step 2: Read back the installed PTE and verify it's writable.
        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            paging::pte_offset_kernel(pmdp, test_addr)
        };
        let pte_after_fault = unsafe { paging::ptep_get(ptep) };
        assert!(
            paging::pte_write(pte_after_fault),
            "initial PTE must be writable"
        );

        // Step 3: Write-protect the PTE (simulating what dup_mmap/copy_pte_range does).
        unsafe {
            paging::set_pte_at(
                mm.pgd as *mut (),
                test_addr,
                ptep,
                paging::pte_wrprotect(pte_after_fault),
            )
        };
        let pte_ro = unsafe { paging::ptep_get(ptep) };
        assert!(
            !paging::pte_write(pte_ro),
            "PTE must be read-only after write-protect"
        );

        // Verify the page is exclusive (refcount == 1) — the reuse path requires this.
        let pfn = paging::pte_pfn(pte_ro) as usize;
        let page_ptr = crate::mm::buddy::pfn_to_page(pfn);
        let rc = unsafe { (*page_ptr).refcount() };
        assert_eq!(rc, 1, "page must be exclusive before COW reuse test");

        // Step 4: Fire a second write fault on the same address.
        // handle_pte_fault sees a present RO PTE + FAULT_FLAG_WRITE → do_wp_page
        // do_wp_page: refcount == 1 → wp_page_reuse (upgrade in-place).
        let ret2 = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(ret2, 0, "COW-reuse fault should succeed");

        // Step 5: Verify PTE is writable again and the same PFN was reused.
        let pte_after_cow = unsafe { paging::ptep_get(ptep) };
        assert!(
            paging::pte_write(pte_after_cow),
            "PTE must be writable after wp_page_reuse"
        );
        assert_eq!(
            paging::pte_pfn(pte_after_cow),
            paging::pte_pfn(pte_ro),
            "wp_page_reuse must not change the PFN"
        );
    }

    // ── smaps dirty accounting ────────────────────────────────────────────────

    #[test]
    fn smaps_for_range_counts_private_dirty() {
        use crate::mm::pagewalk::smaps_for_range;
        use core::sync::atomic::Ordering;

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x00e0_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE,
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        // Install a dirty, writable page via write fault.
        let ret = handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);
        assert_eq!(ret, 0);

        // The installed PTE should be dirty.
        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            paging::pte_offset_kernel(pmdp, test_addr)
        };
        let pte = unsafe { paging::ptep_get(ptep) };
        assert!(paging::pte_dirty(pte), "write fault must produce dirty PTE");

        // _mapcount == 0 → exactly one PTE → private.
        let pfn = paging::pte_pfn(pte) as usize;
        let page_ptr = crate::mm::buddy::pfn_to_page(pfn);
        assert_eq!(
            unsafe { (*page_ptr)._mapcount().load(Ordering::Relaxed) },
            0,
            "_mapcount must be 0 after single fault"
        );

        // smaps_for_range should count this as private_dirty.
        let stats = unsafe {
            smaps_for_range(
                &mm as *const MmStruct,
                test_addr,
                test_addr + crate::mm::frame::PAGE_SIZE as u64,
            )
        };
        assert_eq!(
            stats.private_dirty,
            crate::mm::frame::PAGE_SIZE,
            "one dirty exclusive page must be private_dirty"
        );
        assert_eq!(stats.shared_dirty, 0, "shared_dirty must be zero");
    }

    #[test]
    fn smaps_for_range_counts_shared_dirty() {
        use crate::mm::pagewalk::smaps_for_range;
        use core::sync::atomic::Ordering;

        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe { page.init_lru() };
        }
        unsafe { buddy::set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES) };
        unsafe { buddy::install_test_buddy(0, TEST_PAGES) };
        unsafe { paging::reset_test_pool() };

        let test_addr: u64 = 0x00f0_0000;
        let mut mm = MmStruct::new(paging::init_pgd_for_test() as usize);
        let mut vma = VmAreaStruct::new(
            test_addr,
            test_addr + (crate::mm::frame::PAGE_SIZE as u64),
            VM_WRITE,
        );
        vma.vm_mm = &mut mm as *mut MmStruct;
        vma.vm_page_prot = _PAGE_PRESENT | _PAGE_USER | _PAGE_ACCESSED | _PAGE_NX;

        // Install a dirty page.
        handle_mm_fault(&mut vma, test_addr, FAULT_FLAG_WRITE | FAULT_FLAG_USER);

        let ptep = unsafe {
            let pgdp = paging::pgd_offset_pgd(mm.pgd as *mut pgd_t, test_addr);
            let p4dp = paging::p4d_offset(pgdp, test_addr);
            let pudp = paging::pud_offset(p4dp, test_addr);
            let pmdp = paging::pmd_offset(pudp, test_addr);
            paging::pte_offset_kernel(pmdp, test_addr)
        };
        let pte = unsafe { paging::ptep_get(ptep) };

        // Simulate fork: bump _mapcount to 1 (two PTEs reference the page).
        let pfn = paging::pte_pfn(pte) as usize;
        let page_ptr = crate::mm::buddy::pfn_to_page(pfn);
        unsafe { (*page_ptr)._mapcount().store(1, Ordering::Relaxed) };

        // smaps_for_range should now count it as shared_dirty.
        let stats = unsafe {
            smaps_for_range(
                &mm as *const MmStruct,
                test_addr,
                test_addr + crate::mm::frame::PAGE_SIZE as u64,
            )
        };
        assert_eq!(
            stats.shared_dirty,
            crate::mm::frame::PAGE_SIZE,
            "page with _mapcount=1 must be shared_dirty"
        );
        assert_eq!(stats.private_dirty, 0, "private_dirty must be zero");

        // Restore _mapcount to avoid affecting other tests.
        unsafe { (*page_ptr)._mapcount().store(0, Ordering::Relaxed) };
    }

    // ── Utility ─────────────────────────────────────────────────────────────

    fn make_vmf(vma: &mut VmAreaStruct, address: u64, flags: FaultFlags) -> VmFault {
        VmFault {
            vma: vma as *mut VmAreaStruct,
            gfp_mask: 0,
            pgoff: 0,
            address,
            real_address: address,
            flags,
            pud: ptr::null_mut(),
            pmd: ptr::null_mut(),
            orig_pte: __pte(0),
            pte: ptr::null_mut(),
            page: ptr::null_mut(),
            cow_page: ptr::null_mut(),
        }
    }
}
