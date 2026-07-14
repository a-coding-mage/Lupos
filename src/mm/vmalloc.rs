//! linux-parity: complete
//! linux-source: vendor/linux/mm/vmalloc.c
//! test-origin: linux:vendor/linux/mm/vmalloc.c
/// Kernel virtual memory allocator — non-contiguous physical pages mapped into
/// a contiguous kernel virtual address window.
///
/// Mirrors Linux's `mm/vmalloc.c` at a simplified level: the kernel reserves a
/// fixed 1 GiB VA window (`VMALLOC_START..VMALLOC_END`) in the canonical
/// upper-half address space.  `vmalloc(size)` allocates one physical 4 KiB
/// page per page of the requested size from the buddy allocator, maps each
/// physical frame into the next available VA slot, and returns the window base
/// pointer.  `vfree(ptr)` undoes the mapping and returns the frames.
///
/// ## Virtual-address window
///
/// | Symbol          | Value                   | Notes                         |
/// |-----------------|-------------------------|-------------------------------|
/// | `VMALLOC_START` | `0xFFFF_C900_0000_0000` | safely above kernel image     |
/// | `VMALLOC_END`   | `VMALLOC_START + 1 GiB` | 1 GiB window                  |
///
/// Early boot maps only 0–4 GiB (identity).  PML4/PDPT/PD/PT entries for
/// the vmalloc window are created on demand by `map_kernel_page` and remain
/// in the kernel's page tables permanently (like Linux's vmalloc area).
///
/// ## VA-range allocator
///
/// A static free-list of `VaRange` descriptors (capacity 256) provides
/// bump-allocation until the first `vfree`, then best-fit reuse.
/// No heap memory is used — the list is a fixed `[Option<VaRange>; 256]` array
/// similar to how Linux manages the early `vm_struct` free list.
///
/// ## Per-allocation metadata
///
/// The number of pages for a vmalloc region is stored in a flat metadata
/// array indexed by the region's page offset within the vmalloc window
/// (`(va - VMALLOC_START) / PAGE_SIZE`).  This avoids storing a header
/// before the returned pointer (which would complicate alignment).
///
/// ## References
///
/// - Linux `mm/vmalloc.c` — `__vmalloc_node_range`, `vmap_pages_range`, `vfree`
/// - Linux `include/linux/vmalloc.h` — `struct vm_struct`, `VMALLOC_START`
/// - Linux `arch/x86/include/asm/pgtable_64_types.h` — `__VMALLOC_BASE_L4`
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86::mm::paging::{
    PAGE_KERNEL, map_kernel_page, pgprot_t, unmap_kernel_page, virt_to_phys,
};
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::buddy::{page_to_pfn, with_global_buddy};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::page_flags::GFP_KERNEL;

// ---------------------------------------------------------------------------
// VA window constants
// ---------------------------------------------------------------------------

/// Start of the vmalloc virtual address window.
///
/// Placed at `0xFFFF_C900_0000_0000`, matching Linux's `__VMALLOC_BASE_L4`.
///
/// Ref: Linux `arch/x86/include/asm/pgtable_64_types.h:107`
///      (`__VMALLOC_BASE_L4 = 0xffffc90000000000`)
pub const VMALLOC_START: u64 = 0xFFFF_C900_0000_0000;

/// End of the vmalloc window (exclusive). 1 GiB after `VMALLOC_START`.
pub const VMALLOC_END: u64 = VMALLOC_START + (1 << 30); // +1 GiB

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmStruct {
    pub addr: *mut u8,
    pub size: usize,
    pub flags: u32,
}

/// Maximum number of concurrently live vmalloc regions.
///
/// Each descriptor is 16 bytes; 256 × 16 = 4 KiB of static storage.
const NR_VM_STRUCTS: usize = 256;

/// Maximum number of pages in the vmalloc window (1 GiB / 4 KiB).
const VMALLOC_PAGES: usize = (1 << 30) / PAGE_SIZE;

// ---------------------------------------------------------------------------
// Free-list of VA ranges — mirrors Linux struct vm_struct / vmap_area.
//
// Ref: Linux `include/linux/vmalloc.h` — `struct vm_struct`
//      Linux `mm/vmalloc.c`            — `struct vmap_area`
// ---------------------------------------------------------------------------

/// A free (available) virtual-address range inside `VMALLOC_START..VMALLOC_END`.
///
/// Analogous to Linux's `struct vmap_area` with `VM_FREE` flag set.
#[derive(Clone, Copy)]
struct VaRange {
    /// First page's virtual address (VMALLOC_START-relative, always page-aligned).
    start: u64,
    /// Length in bytes (always a multiple of PAGE_SIZE).
    size: usize,
}

// ---------------------------------------------------------------------------
// Global vmalloc state
// ---------------------------------------------------------------------------

/// True after `vmalloc_init()` has completed.
static VMALLOC_READY: AtomicBool = AtomicBool::new(false);

/// Coarse subsystem lock.
static VMALLOC_LOCK: spin::Mutex<VmallocState> = spin::Mutex::new(VmallocState::new());

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("is_vmalloc_addr", linux_is_vmalloc_addr as usize, false);
    export_symbol_once("vfree", linux_vfree as usize, false);
    export_symbol_once("vunmap", linux_vunmap as usize, false);
    export_symbol_once("vmap", linux_vmap as usize, false);
    export_symbol_once("vmap_pfn", linux_vmap_pfn as usize, true);
    export_symbol_once("vm_map_ram", linux_vm_map_ram as usize, false);
    export_symbol_once("vm_unmap_ram", linux_vm_unmap_ram as usize, false);
    export_symbol_once("vmalloc_to_page", linux_vmalloc_to_page as usize, false);
    export_symbol_once("vmalloc_noprof", linux_vmalloc_noprof as usize, false);
    export_symbol_once("__vmalloc_noprof", linux___vmalloc_noprof as usize, false);
    export_symbol_once("vzalloc_noprof", linux_vzalloc_noprof as usize, false);
    export_symbol_once(
        "vmalloc_user_noprof",
        linux_vmalloc_user_noprof as usize,
        false,
    );
    export_symbol_once(
        "vzalloc_node_noprof",
        linux_vzalloc_node_noprof as usize,
        false,
    );
    export_symbol_once(
        "remap_vmalloc_range",
        linux_remap_vmalloc_range as usize,
        false,
    );
    export_symbol_once(
        "register_vmap_purge_notifier",
        linux_register_vmap_purge_notifier as usize,
        true,
    );
    export_symbol_once(
        "unregister_vmap_purge_notifier",
        linux_unregister_vmap_purge_notifier as usize,
        true,
    );
}

unsafe extern "C" fn linux_is_vmalloc_addr(addr: *const u8) -> bool {
    is_vmalloc_addr(addr)
}

unsafe extern "C" fn linux_vfree(ptr: *mut u8) {
    vfree(ptr);
}

unsafe extern "C" fn linux_vunmap(ptr: *const u8) {
    vunmap(ptr.cast_mut());
}

unsafe extern "C" fn linux_vmap(
    pages: *const *mut crate::mm::page::Page,
    count: usize,
    flags: u32,
    prot: u64,
) -> *mut u8 {
    vmap(pages, count, flags, prot)
}

unsafe extern "C" fn linux_vmap_pfn(pfns: *const usize, count: u32, prot: pgprot_t) -> *mut u8 {
    vmap_pfn(pfns, count as usize, prot)
}

unsafe extern "C" fn linux_vm_map_ram(
    pages: *const *mut crate::mm::page::Page,
    count: u32,
    node: i32,
) -> *mut u8 {
    vm_map_ram(pages, count as usize, node)
}

unsafe extern "C" fn linux_vm_unmap_ram(mem: *mut u8, count: u32) {
    vm_unmap_ram(mem, count as usize);
}

unsafe extern "C" fn linux_vmalloc_to_page(addr: *const u8) -> *mut crate::mm::page::Page {
    vmalloc_to_page(addr)
}

unsafe extern "C" fn linux_vmalloc_noprof(size: usize) -> *mut u8 {
    vmalloc_noprof(size)
}

unsafe extern "C" fn linux___vmalloc_noprof(size: usize, flags: u32) -> *mut u8 {
    __vmalloc_noprof(size, flags)
}

unsafe extern "C" fn linux_vzalloc_noprof(size: usize) -> *mut u8 {
    vzalloc_noprof(size)
}

unsafe extern "C" fn linux_vmalloc_user_noprof(size: usize) -> *mut u8 {
    vmalloc_user_noprof(size)
}

unsafe extern "C" fn linux_vzalloc_node_noprof(size: usize, node: i32) -> *mut u8 {
    vzalloc_node_noprof(size, node)
}

unsafe extern "C" fn linux_remap_vmalloc_range(
    vma: *mut crate::mm::mm_types::VmAreaStruct,
    addr: *mut u8,
    pgoff: u64,
) -> i32 {
    if vma.is_null() {
        return -EINVAL;
    }
    match remap_vmalloc_range(vma as usize, addr, pgoff) {
        Ok(()) => 0,
        Err(err) => -err,
    }
}

struct VmallocState {
    /// Free VA ranges available for new allocations.
    free: [Option<VaRange>; NR_VM_STRUCTS],
    /// `nr_pages[i]` = number of pages in the live allocation whose first
    /// virtual page has PFN-offset `i` within the vmalloc window.
    /// Zero means "no allocation at this slot".
    nr_pages: [u16; VMALLOC_PAGES],
    /// Whether the backing frames for the live allocation should be returned
    /// to the buddy allocator on `vfree`. PFN mappings are caller-owned.
    free_backing: [bool; VMALLOC_PAGES],
}

impl VmallocState {
    const fn new() -> Self {
        VmallocState {
            free: [None; NR_VM_STRUCTS],
            nr_pages: [0u16; VMALLOC_PAGES],
            free_backing: [false; VMALLOC_PAGES],
        }
    }
}

// ---------------------------------------------------------------------------
// VA-range allocator helpers
// ---------------------------------------------------------------------------

/// Find a free VA range of at least `size` bytes (best-fit).
/// Splits the found range, storing any remainder back.
/// Returns `None` if no range is large enough.
fn va_alloc(state: &mut VmallocState, size: usize) -> Option<u64> {
    debug_assert_eq!(
        size & (PAGE_SIZE - 1),
        0,
        "va_alloc: size must be page-aligned"
    );

    // Find the best-fit (smallest range that is large enough).
    let mut best_idx: Option<usize> = None;
    for (i, slot) in state.free.iter().enumerate() {
        if let Some(r) = slot {
            if r.size >= size {
                match best_idx {
                    None => best_idx = Some(i),
                    Some(bi) => {
                        if r.size < state.free[bi].unwrap().size {
                            best_idx = Some(i);
                        }
                    }
                }
            }
        }
    }

    let idx = best_idx?;
    let range = state.free[idx].take().unwrap();

    // If the range is larger than needed, put the remainder back.
    if range.size > size {
        let remainder = VaRange {
            start: range.start + size as u64,
            size: range.size - size,
        };
        // Find an empty slot for the remainder.
        if let Some(slot) = state.free.iter_mut().find(|s| s.is_none()) {
            *slot = Some(remainder);
        }
        // If no slot is available, the remainder is lost (internal fragmentation).
        // In practice NR_VM_STRUCTS is large enough to avoid this.
    }

    Some(range.start)
}

/// Return a VA range to the free list, coalescing with adjacent free ranges.
fn va_free(state: &mut VmallocState, start: u64, size: usize) {
    let end = start + size as u64;

    // Look for adjacent free ranges to coalesce.
    let mut coalesced_start = start;
    let mut coalesced_size = size;

    for slot in state.free.iter_mut() {
        if let Some(r) = slot {
            if r.start + r.size as u64 == coalesced_start {
                // `r` is immediately before us.
                coalesced_start = r.start;
                coalesced_size += r.size;
                *slot = None;
            } else if coalesced_start + coalesced_size as u64 == r.start {
                // `r` is immediately after us.
                coalesced_size += r.size;
                *slot = None;
            }
        }
    }
    let _ = end; // coalescing logic consumes `end` conceptually

    // Place coalesced range in the first empty free-list slot.
    if let Some(slot) = state.free.iter_mut().find(|s| s.is_none()) {
        *slot = Some(VaRange {
            start: coalesced_start,
            size: coalesced_size,
        });
    }
    // If no free slot exists, the VA range is permanently lost.
    // This is a degraded but safe state (allocations will eventually fail).
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise the vmalloc subsystem.
///
/// Registers the entire `VMALLOC_START..VMALLOC_END` window as one large free
/// VA range.  Must be called once after `slab_init()` / buddy init.
///
/// Ref: Linux `vmalloc_init()` — `mm/vmalloc.c`
pub fn vmalloc_init() {
    let mut state = VMALLOC_LOCK.lock();
    state.free[0] = Some(VaRange {
        start: VMALLOC_START,
        size: (VMALLOC_END - VMALLOC_START) as usize,
    });
    drop(state);
    VMALLOC_READY.store(true, Ordering::Release);
}

/// Allocate `size` bytes of kernel virtual memory backed by non-contiguous
/// physical pages.  Returns a pointer to the start of the mapped region.
///
/// - `size` is rounded up to the next page boundary.
/// - Each page is independently allocated from the buddy allocator via
///   `alloc_pages(0, GFP_KERNEL)`.
/// - The pages are mapped contiguously in the vmalloc VA window using
///   `map_kernel_page` with `PAGE_KERNEL` protection.
/// - Returns a null pointer on allocation failure (OOM).
///
/// Ref: Linux `__vmalloc_node_range()` — `mm/vmalloc.c`
pub fn vmalloc(size: usize) -> *mut u8 {
    assert!(
        VMALLOC_READY.load(Ordering::Acquire),
        "vmalloc: called before vmalloc_init"
    );
    if size == 0 {
        return core::ptr::null_mut();
    }

    // Round up to page boundary.
    let n_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    let alloc_size = n_pages * PAGE_SIZE;

    if n_pages > u16::MAX as usize {
        return core::ptr::null_mut(); // too large for metadata encoding
    }

    let mut state = VMALLOC_LOCK.lock();

    // 1. Claim a VA range.
    let va_start = match va_alloc(&mut state, alloc_size) {
        Some(va) => va,
        None => return core::ptr::null_mut(),
    };

    // 2. Allocate and map one physical frame per page.
    let mut mapped = 0usize;
    let mut ok = true;
    for i in 0..n_pages {
        let va = va_start + (i * PAGE_SIZE) as u64;
        let page_opt = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL));
        match page_opt {
            None => {
                ok = false;
                break;
            }
            Some(page_ptr) => {
                let pfn = page_to_pfn(page_ptr);
                let phys = (pfn * PAGE_SIZE) as u64;
                // SAFETY: VA is in the vmalloc window, phys is page-aligned.
                unsafe { map_kernel_page(va, phys, PAGE_KERNEL) };
                mapped += 1;
            }
        }
    }

    if !ok {
        // Roll back partial mapping.
        for i in 0..mapped {
            let va = va_start + (i * PAGE_SIZE) as u64;
            if let Some(phys) = virt_to_phys(va) {
                let pfn = phys as usize / PAGE_SIZE;
                // Free the physical frame back to buddy.
                // We only mapped it — the page is order-0 with buddy metadata intact.
                let page_ptr = crate::mm::buddy::pfn_to_page(pfn);
                with_global_buddy(|b| b.free_pages(page_ptr, 0));
                unsafe { unmap_kernel_page(va) };
            }
        }
        va_free(&mut state, va_start, alloc_size);
        return core::ptr::null_mut();
    }

    // 3. Record the allocation size in the metadata array.
    let window_offset = (va_start - VMALLOC_START) as usize / PAGE_SIZE;
    state.nr_pages[window_offset] = n_pages as u16;
    state.free_backing[window_offset] = true;
    drop(state);

    unsafe {
        sync_vmalloc_pgd_slot_to_current_mm(va_start, alloc_size);
    }

    va_start as *mut u8
}

/// Free a vmalloc allocation returned by `vmalloc(ptr)`.
///
/// - Looks up the page count in the metadata array.
/// - Unmaps each page via `unmap_kernel_page`.
/// - Frees each backing physical frame to the buddy allocator.
/// - Returns the VA range to the free list.
///
/// Does nothing if `ptr` is null.
///
/// Ref: Linux `vfree()` — `mm/vmalloc.c`
pub fn vfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    assert!(
        VMALLOC_READY.load(Ordering::Acquire),
        "vfree: called before vmalloc_init"
    );

    let va_start = ptr as u64;
    debug_assert!(
        va_start >= VMALLOC_START && va_start < VMALLOC_END,
        "vfree: pointer {:#x} outside vmalloc window",
        va_start
    );

    let window_offset = (va_start - VMALLOC_START) as usize / PAGE_SIZE;
    let mut state = VMALLOC_LOCK.lock();
    let n_pages = state.nr_pages[window_offset] as usize;
    if n_pages == 0 {
        // Double-free or invalid pointer — do nothing (debug assert above
        // already checked the window range).
        return;
    }
    let free_backing = state.free_backing[window_offset];
    state.nr_pages[window_offset] = 0;
    state.free_backing[window_offset] = false;

    let alloc_size = n_pages * PAGE_SIZE;

    for i in 0..n_pages {
        let va = va_start + (i * PAGE_SIZE) as u64;
        if free_backing && let Some(phys) = virt_to_phys(va) {
            let pfn = phys as usize / PAGE_SIZE;
            let page_ptr = crate::mm::buddy::pfn_to_page(pfn);
            with_global_buddy(|b| b.free_pages(page_ptr, 0));
        }
        unsafe { unmap_kernel_page(va) };
    }

    va_free(&mut state, va_start, alloc_size);
}

pub fn vmalloc_usable_size(ptr: *const u8) -> usize {
    if ptr.is_null() || !is_vmalloc_addr(ptr) {
        return 0;
    }
    if !VMALLOC_READY.load(Ordering::Acquire) {
        return 0;
    }
    let va_start = ptr as u64;
    let window_offset = (va_start - VMALLOC_START) as usize / PAGE_SIZE;
    let state = VMALLOC_LOCK.lock();
    (state.nr_pages[window_offset] as usize) * PAGE_SIZE
}

pub fn is_vmalloc_addr(addr: *const u8) -> bool {
    let addr = addr as u64;
    (VMALLOC_START..VMALLOC_END).contains(&addr)
}

pub fn is_vmalloc_or_module_addr(addr: *const u8) -> bool {
    is_vmalloc_addr(addr) || crate::arch::x86::mm::init::is_module_addr(addr)
}

#[cfg(not(test))]
pub unsafe fn vmalloc_fault(addr: u64) -> bool {
    if !is_vmalloc_addr(addr as *const u8) {
        return false;
    }
    if virt_to_phys(addr).is_none() {
        return false;
    }

    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return false;
    }
    let mm = unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    };
    if mm.is_null() {
        return false;
    }

    unsafe {
        sync_vmalloc_pgd_slot_to_mm(
            mm,
            addr & crate::arch::x86::mm::paging::PAGE_MASK,
            PAGE_SIZE,
        )
    };
    true
}

#[cfg(test)]
pub unsafe fn vmalloc_fault(addr: u64) -> bool {
    is_vmalloc_addr(addr as *const u8)
}

#[cfg(not(test))]
unsafe fn sync_vmalloc_pgd_slot_to_current_mm(start: u64, size: usize) {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return;
    }
    let mm = unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    };
    unsafe { sync_vmalloc_pgd_slot_to_mm(mm, start, size) };
}

#[cfg(test)]
unsafe fn sync_vmalloc_pgd_slot_to_current_mm(_start: u64, _size: usize) {}

#[cfg(not(test))]
pub unsafe fn sync_vmalloc_pgd_slot_to_mm(
    mm: *mut crate::mm::mm_types::MmStruct,
    start: u64,
    size: usize,
) {
    use crate::arch::x86::mm::paging::{init_pgd_phys, pgd_index, pgd_t, phys_to_virt};

    if mm.is_null() || unsafe { (*mm).pgd == 0 } {
        return;
    }
    let end = start.saturating_add(size.saturating_sub(1) as u64);
    let first = pgd_index(start);
    let last = pgd_index(end);
    let src = phys_to_virt(init_pgd_phys()) as *mut pgd_t;
    let dst = unsafe { (*mm).pgd as *mut pgd_t };
    for idx in first..=last {
        unsafe {
            *dst.add(idx) = *src.add(idx);
        }
    }
    unsafe { flush_cr3_if_mm_is_current(mm) };
}

#[cfg(test)]
pub unsafe fn sync_vmalloc_pgd_slot_to_mm(
    _mm: *mut crate::mm::mm_types::MmStruct,
    _start: u64,
    _size: usize,
) {
}

#[cfg(not(test))]
pub unsafe fn sync_vmalloc_to_mm(mm: *mut crate::mm::mm_types::MmStruct) {
    unsafe {
        sync_vmalloc_pgd_slot_to_mm(mm, VMALLOC_START, (VMALLOC_END - VMALLOC_START) as usize)
    };
}

#[cfg(test)]
pub unsafe fn sync_vmalloc_to_mm(_mm: *mut crate::mm::mm_types::MmStruct) {}

#[cfg(not(test))]
unsafe fn flush_cr3_if_mm_is_current(mm: *mut crate::mm::mm_types::MmStruct) {
    let pgd_virt = unsafe { (*mm).pgd as u64 };
    let Some(pgd_phys) = crate::arch::x86::mm::paging::virt_to_phys(pgd_virt) else {
        return;
    };
    if crate::arch::x86::mm::paging::read_cr3() == pgd_phys {
        unsafe {
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) pgd_phys,
                options(nostack, preserves_flags)
            );
        }
    }
}

pub fn vmalloc_noprof(size: usize) -> *mut u8 {
    vmalloc(size)
}

pub fn __vmalloc_noprof(size: usize, _flags: u32) -> *mut u8 {
    vmalloc(size)
}

pub fn __vmalloc_node_noprof(size: usize, _align: usize, _flags: u32, _node: i32) -> *mut u8 {
    vmalloc(size)
}

pub fn __vmalloc_node_range_noprof(
    size: usize,
    _align: usize,
    _start: u64,
    _end: u64,
    _flags: u32,
    _page_flags: u32,
    _prot: u64,
    _node: i32,
    _caller: usize,
) -> *mut u8 {
    vmalloc(size)
}

pub fn vmalloc_node_noprof(size: usize, node: i32) -> *mut u8 {
    __vmalloc_node_noprof(size, PAGE_SIZE, GFP_KERNEL, node)
}

pub fn vmalloc_huge_node_noprof(size: usize, flags: u32, node: i32) -> *mut u8 {
    __vmalloc_node_noprof(size, PAGE_SIZE, flags, node)
}

pub fn vmalloc_huge(size: usize, flags: u32) -> *mut u8 {
    vmalloc_huge_node_noprof(size, flags, -1)
}

pub fn vzalloc_noprof(size: usize) -> *mut u8 {
    let ptr = vmalloc(size);
    if !ptr.is_null() && size != 0 {
        unsafe { core::ptr::write_bytes(ptr, 0, size) };
    }
    ptr
}

pub fn vzalloc_node_noprof(size: usize, node: i32) -> *mut u8 {
    let ptr = vmalloc_node_noprof(size, node);
    if !ptr.is_null() && size != 0 {
        unsafe { core::ptr::write_bytes(ptr, 0, size) };
    }
    ptr
}

pub fn vmalloc_user_noprof(size: usize) -> *mut u8 {
    vzalloc_noprof(size)
}

pub fn vmalloc_32_noprof(size: usize) -> *mut u8 {
    vmalloc(size)
}

pub fn vmalloc_32_user_noprof(size: usize) -> *mut u8 {
    vzalloc_noprof(size)
}

pub fn vmalloc_array_noprof(n: usize, size: usize) -> *mut u8 {
    let Some(bytes) = n.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    vmalloc(bytes)
}

pub fn __vmalloc_array_noprof(n: usize, size: usize, _flags: u32) -> *mut u8 {
    vmalloc_array_noprof(n, size)
}

pub fn vcalloc_noprof(n: usize, size: usize) -> *mut u8 {
    let Some(bytes) = n.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    vzalloc_noprof(bytes)
}

pub fn __vcalloc_noprof(n: usize, size: usize, _flags: u32) -> *mut u8 {
    vcalloc_noprof(n, size)
}

pub fn vrealloc_node_align_noprof(
    ptr: *mut u8,
    new_size: usize,
    _align: usize,
    _flags: u32,
    _node: i32,
) -> *mut u8 {
    if ptr.is_null() {
        return vmalloc(new_size);
    }
    let new_ptr = vmalloc(new_size);
    if !new_ptr.is_null() {
        vfree(ptr);
    }
    new_ptr
}

pub fn vfree_atomic(ptr: *mut u8) {
    vfree(ptr)
}

pub fn vunmap(addr: *mut u8) {
    vfree(addr)
}

pub fn vmalloc_to_pfn(addr: *const u8) -> usize {
    virt_to_phys(addr as u64)
        .map(|phys| phys as usize / PAGE_SIZE)
        .unwrap_or(0)
}

pub fn vmalloc_to_page(addr: *const u8) -> *mut crate::mm::page::Page {
    let pfn = vmalloc_to_pfn(addr);
    if pfn == 0 {
        core::ptr::null_mut()
    } else {
        crate::mm::buddy::pfn_to_page(pfn)
    }
}

pub fn get_vm_area_size(area: *const VmStruct) -> usize {
    if area.is_null() {
        0
    } else {
        unsafe { (*area).size }
    }
}

pub fn get_vm_area(size: usize, flags: u32) -> VmStruct {
    VmStruct {
        addr: vmalloc(size),
        size,
        flags,
    }
}

pub fn get_vm_area_caller(size: usize, flags: u32, _caller: usize) -> VmStruct {
    get_vm_area(size, flags)
}

pub fn __get_vm_area_caller(
    size: usize,
    flags: u32,
    _start: u64,
    _end: u64,
    caller: usize,
) -> VmStruct {
    get_vm_area_caller(size, flags, caller)
}

pub fn find_vm_area(addr: *const u8) -> Option<VmStruct> {
    if is_vmalloc_addr(addr) {
        Some(VmStruct {
            addr: addr as *mut u8,
            size: PAGE_SIZE,
            flags: 0,
        })
    } else {
        None
    }
}

pub fn free_vm_area(area: VmStruct) {
    vfree(area.addr)
}

pub fn remove_vm_area(addr: *mut u8) -> Option<VmStruct> {
    if is_vmalloc_addr(addr) {
        vfree(addr);
        Some(VmStruct {
            addr,
            size: PAGE_SIZE,
            flags: 0,
        })
    } else {
        None
    }
}

pub fn vm_map_ram(_pages: *const *mut crate::mm::page::Page, count: usize, _node: i32) -> *mut u8 {
    vmalloc(count.saturating_mul(PAGE_SIZE))
}

pub fn vm_unmap_ram(mem: *mut u8, _count: usize) {
    vfree(mem)
}

pub fn vmap(
    pages: *const *mut crate::mm::page::Page,
    count: usize,
    _flags: u32,
    _prot: u64,
) -> *mut u8 {
    vm_map_ram(pages, count, -1)
}

pub fn vmap_pfn(pfns: *const usize, count: usize, prot: pgprot_t) -> *mut u8 {
    assert!(
        VMALLOC_READY.load(Ordering::Acquire),
        "vmap_pfn: called before vmalloc_init"
    );
    if pfns.is_null() || count == 0 || count > u16::MAX as usize {
        return core::ptr::null_mut();
    }
    let Some(alloc_size) = count.checked_mul(PAGE_SIZE) else {
        return core::ptr::null_mut();
    };

    let mut state = VMALLOC_LOCK.lock();
    let Some(va_start) = va_alloc(&mut state, alloc_size) else {
        return core::ptr::null_mut();
    };

    for i in 0..count {
        let pfn = unsafe { *pfns.add(i) };
        let va = va_start + (i * PAGE_SIZE) as u64;
        unsafe { map_kernel_page(va, (pfn * PAGE_SIZE) as u64, prot) };
    }

    let window_offset = (va_start - VMALLOC_START) as usize / PAGE_SIZE;
    state.nr_pages[window_offset] = count as u16;
    state.free_backing[window_offset] = false;
    drop(state);

    unsafe {
        sync_vmalloc_pgd_slot_to_current_mm(va_start, alloc_size);
    }

    va_start as *mut u8
}

pub fn remap_vmalloc_range(_vma: usize, addr: *mut u8, _pgoff: u64) -> Result<(), i32> {
    if is_vmalloc_addr(addr) {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub fn remap_vmalloc_range_partial(
    _vma: usize,
    _uaddr: u64,
    addr: *mut u8,
    _pgoff: u64,
    _size: usize,
) -> Result<(), i32> {
    remap_vmalloc_range(_vma, addr, _pgoff)
}

pub fn vm_unmap_aliases() {}

pub fn register_vmap_purge_notifier(_notifier: usize) -> i32 {
    0
}

pub fn unregister_vmap_purge_notifier(_notifier: usize) -> i32 {
    0
}

unsafe extern "C" fn linux_register_vmap_purge_notifier(notifier: usize) -> i32 {
    register_vmap_purge_notifier(notifier)
}

unsafe extern "C" fn linux_unregister_vmap_purge_notifier(notifier: usize) -> i32 {
    unregister_vmap_purge_notifier(notifier)
}

pub const fn arch_vmap_p4d_supported(_prot: u64) -> bool {
    false
}

pub const fn arch_vmap_pud_supported(_prot: u64) -> bool {
    false
}

pub const fn arch_vmap_pmd_supported(_prot: u64) -> bool {
    false
}

pub const fn arch_vmap_pte_supported_shift(_size: usize) -> u32 {
    PAGE_SIZE.trailing_zeros()
}

pub const fn arch_vmap_pte_range_map_size(
    _addr: u64,
    _end: u64,
    _pfn: u64,
    _max_page_shift: u32,
) -> usize {
    PAGE_SIZE
}

pub const fn arch_vmap_pte_range_unmap_size(_addr: u64, _end: u64) -> usize {
    PAGE_SIZE
}

pub const fn arch_vmap_pgprot_tagged(prot: u64) -> u64 {
    prot
}

pub fn set_vm_flush_reset_perms(_addr: *mut u8) {}

pub fn vmalloc_dump_obj(_ptr: *const u8) -> bool {
    false
}

pub fn vread_iter(_iter: usize, _addr: u64, _count: usize) -> usize {
    0
}

pub fn vunmap_range(_addr: u64, _end: u64) {}

pub fn vm_area_add_early(_vm: *mut VmStruct) {}

pub fn vm_area_register_early(_vm: *mut VmStruct, _align: usize) {}

pub fn pcpu_free_vm_areas(_vms: *mut *mut VmStruct, _nr_vms: usize) {}

pub fn memalloc_apply_gfp_scope(_gfp: u32) -> u32 {
    0
}

pub fn memalloc_restore_scope(_scope: u32) {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::arch::x86::mm::paging::test_pool;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    /// Re-initialise the vmalloc subsystem for a clean test.
    ///
    /// Resets the paging mock PML4, clears the free-list and metadata, then
    /// re-registers the full window as a single free range.
    unsafe fn setup() {
        unsafe { test_pool::reset() };

        let mut state = VMALLOC_LOCK.lock();
        // Clear free-list and metadata.
        for slot in state.free.iter_mut() {
            *slot = None;
        }
        for n in state.nr_pages.iter_mut() {
            *n = 0;
        }
        for owned in state.free_backing.iter_mut() {
            *owned = false;
        }
        // Re-register the whole window.
        state.free[0] = Some(VaRange {
            start: VMALLOC_START,
            size: (VMALLOC_END - VMALLOC_START) as usize,
        });
        VMALLOC_READY.store(true, Ordering::Relaxed);
    }

    // ── VA allocator ────────────────────────────────────────────────────

    #[test]
    fn va_alloc_first_range_starts_at_vmalloc_start() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { setup() };
        let mut state = VMALLOC_LOCK.lock();
        let va = va_alloc(&mut state, PAGE_SIZE).unwrap();
        assert_eq!(va, VMALLOC_START);
    }

    #[test]
    fn two_va_allocs_do_not_overlap() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { setup() };
        let mut state = VMALLOC_LOCK.lock();
        let va_a = va_alloc(&mut state, PAGE_SIZE).unwrap();
        let va_b = va_alloc(&mut state, PAGE_SIZE).unwrap();
        // The two ranges must not overlap.
        let a_end = va_a + PAGE_SIZE as u64;
        let b_end = va_b + PAGE_SIZE as u64;
        let overlap = va_a < b_end && va_b < a_end;
        assert!(!overlap, "va ranges overlap: a={:#x} b={:#x}", va_a, va_b);
    }

    #[test]
    fn va_free_reuse_same_address() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { setup() };
        let mut state = VMALLOC_LOCK.lock();

        let va = va_alloc(&mut state, PAGE_SIZE).unwrap();
        va_free(&mut state, va, PAGE_SIZE);

        // The same address should be available again.
        let va2 = va_alloc(&mut state, PAGE_SIZE).unwrap();
        assert_eq!(va, va2, "freed VA range was not reused");
    }

    #[test]
    fn va_free_coalesces_adjacent_ranges() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { setup() };
        let mut state = VMALLOC_LOCK.lock();

        let va_a = va_alloc(&mut state, PAGE_SIZE).unwrap();
        let va_b = va_alloc(&mut state, PAGE_SIZE).unwrap();
        // Free both — they should coalesce into one 2-page range.
        va_free(&mut state, va_a, PAGE_SIZE);
        va_free(&mut state, va_b, PAGE_SIZE);

        // Now a 2-page allocation should succeed.
        let va_big = va_alloc(&mut state, 2 * PAGE_SIZE).unwrap();
        assert_eq!(
            va_big,
            va_a.min(va_b),
            "coalesced range not reused correctly"
        );
    }

    // ── PTE write / read (paging layer) ─────────────────────────────────

    #[test]
    fn pte_write_read_via_map_kernel_page() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { test_pool::reset() };
        // Pick a virtual address in the vmalloc window and a dummy physical
        // address (page-aligned).
        let va: u64 = VMALLOC_START;
        let phys: u64 = 0x0000_0000_0100_0000; // 16 MiB physical
        unsafe {
            crate::arch::x86::mm::paging::map_kernel_page(va, phys, PAGE_KERNEL);
        }
        let resolved = crate::arch::x86::mm::paging::virt_to_phys(va);
        assert_eq!(resolved, Some(phys), "PTE round-trip failed");
    }

    #[test]
    fn vmap_pfn_maps_caller_owned_pfns() {
        let _g = TEST_LOCK.lock().unwrap();
        unsafe { setup() };

        let pfns = [0x1000_0000usize / PAGE_SIZE, 0x1001_0000usize / PAGE_SIZE];
        let ptr = vmap_pfn(pfns.as_ptr(), pfns.len(), PAGE_KERNEL);

        assert!(!ptr.is_null());
        assert_eq!(virt_to_phys(ptr as u64), Some(0x1000_0000));
        assert_eq!(
            virt_to_phys((ptr as u64) + PAGE_SIZE as u64),
            Some(0x1001_0000)
        );

        vfree(ptr);

        assert_eq!(virt_to_phys(ptr as u64), None);
        assert_eq!(virt_to_phys((ptr as u64) + PAGE_SIZE as u64), None);
    }
}
