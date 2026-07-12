//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Buddy allocator — the lupos equivalent of Linux's zone-based buddy system.
///
/// Replaces the bitmap frame allocator with a proper buddy allocator that:
/// - Manages physical memory in power-of-two blocks (orders 0–10)
/// - Coalesces adjacent free blocks on deallocation (buddy merging)
/// - Splits larger blocks on allocation when no exact-order block is available
/// - Tracks per-page metadata via a global `mem_map` array of `Page` structs
/// - Organizes pages into zones (ZONE_DMA 0–16 MB, ZONE_NORMAL 16 MB+)
///
/// ## Linux references
///
/// | Component           | Linux file                        | Key function / struct              |
/// |---------------------|-----------------------------------|------------------------------------|
/// | Buddy free/coalesce | mm/page_alloc.c:978               | `__free_one_page()`                |
/// | Buddy alloc/split   | mm/page_alloc.c:1732              | `expand()`                         |
/// | Buddy PFN XOR       | mm/internal.h:758                 | `__find_buddy_pfn()`               |
/// | Page validity check | mm/internal.h:717                 | `page_is_buddy()`                  |
/// | Free-area lists     | include/linux/mmzone.h:138        | `struct free_area`                 |
/// | Zone structure      | include/linux/mmzone.h:879        | `struct zone`                      |
/// | mem_map array       | mm/page_alloc.c                   | `mem_map` / `vmemmap`              |
/// | GFP flags           | include/linux/gfp_types.h         | `gfp_t`, `GFP_KERNEL`, etc.        |
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use crate::arch::x86::mm::paging::phys_to_virt;
use crate::mm::frame::{PAGE_SIZE, PhysFrame};
use crate::mm::page::Page;
use crate::mm::page_flags::{
    __GFP_ZERO, GfpFlags, MAX_NR_ZONES, MigrateType, PAGE_TYPE_NONE, ZoneType, gfp_zone,
};
use crate::mm::region::MemoryMap;
use crate::mm::zone::{MAX_PAGE_ORDER, NR_PAGE_ORDERS, ZONE_DMA_MAX_PFN, Zone};

/// Number of physical bytes covered by the bootstrap identity/direct map.
///
/// The x86-64 boot stub fills 64 PD tables with 2 MiB leaves for both the
/// identity map and Linux-style direct map, so early boot code may only
/// dereference physical memory below this limit through `phys_to_virt()`.
const BOOT_DIRECT_MAP_BYTES: usize = 64 * 1024 * 1024 * 1024;
const BOOT_DIRECT_MAP_BYTES_U64: u64 = BOOT_DIRECT_MAP_BYTES as u64;

// ---------------------------------------------------------------------------
// Global mem_map — a flat array of Page structs for all physical frames.
//
// Placed dynamically after _kernel_end during init.  Accessed via static
// atomics so unit tests can substitute a local array.
// ---------------------------------------------------------------------------

/// Pointer to the first element of the global mem_map array.
static MEM_MAP_PTR: AtomicPtr<Page> = AtomicPtr::new(core::ptr::null_mut());
/// PFN of the first element in the mem_map array.
static MEM_MAP_BASE_PFN: AtomicUsize = AtomicUsize::new(0);
/// Number of Page entries in the mem_map array.
static MEM_MAP_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Convert a page frame number to a pointer into the mem_map array.
///
/// Equivalent to Linux's `pfn_to_page()` for FLATMEM.
///
/// # Panics
/// Debug-asserts that `pfn` is within the mem_map bounds.
#[inline]
pub fn pfn_to_page(pfn: usize) -> *mut Page {
    let base = MEM_MAP_BASE_PFN.load(Ordering::Relaxed);
    let count = MEM_MAP_COUNT.load(Ordering::Relaxed);
    let ptr = MEM_MAP_PTR.load(Ordering::Relaxed);
    debug_assert!(
        pfn >= base && pfn < base + count,
        "pfn_to_page: pfn {} out of range [{}, {})",
        pfn,
        base,
        base + count
    );
    unsafe { ptr.add(pfn - base) }
}

/// Return true when a PFN falls within the active `mem_map`.
///
/// Mirrors Linux's `pfn_valid()` check for the flat memory model Lupos uses.
#[inline]
pub fn pfn_valid(pfn: usize) -> bool {
    let base = MEM_MAP_BASE_PFN.load(Ordering::Relaxed);
    let count = MEM_MAP_COUNT.load(Ordering::Relaxed);
    let Some(end) = base.checked_add(count) else {
        return false;
    };
    count != 0 && pfn >= base && pfn < end
}

/// Convert a Page pointer back to its page frame number.
///
/// Equivalent to Linux's `page_to_pfn()` for FLATMEM.
#[inline]
pub fn page_to_pfn(page: *const Page) -> usize {
    let base = MEM_MAP_BASE_PFN.load(Ordering::Relaxed);
    let ptr = MEM_MAP_PTR.load(Ordering::Relaxed);
    let offset = unsafe { (page as *const Page).offset_from(ptr) } as usize;
    base + offset
}

/// Return true when `page` points into the active global `mem_map`.
///
/// This lets higher layers distinguish buddy-backed `Page` descriptors from
/// heap-allocated test pages without relying on `offset_from()` on unrelated
/// allocations.
#[inline]
pub fn page_in_mem_map(page: *const Page) -> bool {
    let ptr = MEM_MAP_PTR.load(Ordering::Relaxed) as usize;
    let count = MEM_MAP_COUNT.load(Ordering::Relaxed);
    if ptr == 0 || count == 0 {
        return false;
    }

    let page_addr = page as usize;
    let size = core::mem::size_of::<Page>();
    let end = ptr.saturating_add(count.saturating_mul(size));
    page_addr >= ptr && page_addr < end && ((page_addr - ptr) % size == 0)
}

/// Install a custom mem_map (used by unit tests to substitute a stack-allocated array).
///
/// # Safety
/// The caller must ensure `ptr` points to `count` valid, initialized `Page` entries
/// and that the array remains valid for the duration of use.
#[cfg(test)]
pub unsafe fn set_mem_map(ptr: *mut Page, base_pfn: usize, count: usize) {
    MEM_MAP_PTR.store(ptr, Ordering::Relaxed);
    MEM_MAP_BASE_PFN.store(base_pfn, Ordering::Relaxed);
    MEM_MAP_COUNT.store(count, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Buddy PFN arithmetic
// ---------------------------------------------------------------------------

/// Calculate the PFN of a page's buddy at a given order.
///
/// The buddy of PFN `p` at order `n` is `p XOR (1 << n)`.
/// This ensures that a pair of buddies always have PFNs that differ only
/// in bit `n`, and the lower-PFN buddy is aligned to `2^(n+1)`.
///
/// Ref: Linux mm/internal.h:758 — `__find_buddy_pfn()`
#[inline]
pub fn find_buddy_pfn(pfn: usize, order: usize) -> usize {
    pfn ^ (1 << order)
}

// ---------------------------------------------------------------------------
// BuddyAllocator
// ---------------------------------------------------------------------------

/// The main buddy allocator struct, holding zones and bookkeeping.
pub struct BuddyAllocator {
    /// Memory zones: [ZONE_DMA, ZONE_NORMAL].
    pub zones: [Zone; MAX_NR_ZONES],
    /// Total free pages across all zones.
    total_free_pages: usize,
    /// Highest PFN in the system (exclusive).
    max_pfn: usize,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl BuddyAllocator {
    /// Create a new, uninitialized buddy allocator.
    pub const fn new() -> Self {
        BuddyAllocator {
            zones: [
                Zone::new(ZoneType::ZoneDma, "DMA"),
                Zone::new(ZoneType::ZoneNormal, "Normal"),
            ],
            total_free_pages: 0,
            max_pfn: 0,
        }
    }

    unsafe fn prepare_allocated_block(page: *mut Page, order: usize) {
        let nr_pages = 1usize << order;
        for i in 0..nr_pages {
            let current = unsafe { page.add(i) };
            unsafe {
                (*current).flags.store(0, Ordering::Relaxed);
                (*current).mapping = 0;
                (*current).index = 0;
                (*current).private = 0;
                (*current)
                    .page_type
                    .store(PAGE_TYPE_NONE, Ordering::Relaxed);
                (*current)._mapcount().store(-1, Ordering::Relaxed);
                (*current)._refcount.store(0, Ordering::Relaxed);
                (*current).init_lru();
            }
        }
    }

    /// Initialize the buddy allocator from the physical memory map.
    ///
    /// This is called once during boot.  It:
    /// 1. Determines the highest PFN from the memory map
    /// 2. Places the mem_map array after `kernel_end` (page-aligned)
    /// 3. Initializes all `Page` structs (reserved by default)
    /// 4. Sets up zone boundaries
    /// 5. Walks available regions and frees pages into the buddy system
    ///
    /// The first 1 MiB, the kernel image, and the mem_map itself are
    /// kept reserved (never freed into buddy).
    ///
    /// # Safety
    /// - Must be called exactly once, before any allocation.
    /// - `kernel_start`/`kernel_end` must be valid physical addresses.
    /// - The bootstrap identity/direct map must cover every managed PFN.
    pub unsafe fn init(&mut self, memory_map: &MemoryMap, kernel_start: u64, kernel_end: u64) {
        // 1. Find the highest available PFN that is covered by the bootstrap
        // identity/direct map.  Later early-boot users access allocated frames
        // through `phys_to_virt()`, so do not manage PFNs above that mapped
        // window until the direct map can be extended.
        let max_pfn = highest_boot_mapped_available_pfn(memory_map);
        self.max_pfn = max_pfn;

        let mem_map_pages = max_pfn; // one Page struct per PFN from 0 to max_pfn
        let mem_map_bytes = mem_map_pages
            .checked_mul(core::mem::size_of::<Page>())
            .expect("mem_map size overflow");
        let mem_map_start =
            choose_mem_map_start(memory_map, kernel_start, kernel_end, mem_map_bytes)
                .expect("mem_map must fit in available boot-mapped RAM");
        let mem_map_end = align_up(
            mem_map_start
                .checked_add(mem_map_bytes)
                .expect("mem_map end overflow"),
            PAGE_SIZE,
        );

        // Store `mem_map` in the kernel direct map so page metadata is always
        // accessed through a canonical kernel virtual address.
        let mem_map_ptr = phys_to_virt(mem_map_start as u64) as *mut Page;

        // Initialize the global mem_map statics.
        MEM_MAP_PTR.store(mem_map_ptr, Ordering::Relaxed);
        MEM_MAP_BASE_PFN.store(0, Ordering::Relaxed);
        MEM_MAP_COUNT.store(mem_map_pages, Ordering::Relaxed);
        crate::arch::x86::kernel::head64::set_linux_vmemmap_base(mem_map_ptr, 0);

        // 3. Initialize all Page structs to default state.
        for i in 0..mem_map_pages {
            unsafe {
                let page = mem_map_ptr.add(i);
                core::ptr::write(page, Page::new());
                (*page).init_lru();
            }
        }

        // 4. Setup zone boundaries.
        let dma_end_pfn = core::cmp::min(ZONE_DMA_MAX_PFN, max_pfn);
        self.zones[ZoneType::ZoneDma as usize].zone_start_pfn = 0;
        self.zones[ZoneType::ZoneDma as usize].spanned_pages = dma_end_pfn;

        if max_pfn > dma_end_pfn {
            self.zones[ZoneType::ZoneNormal as usize].zone_start_pfn = dma_end_pfn;
            self.zones[ZoneType::ZoneNormal as usize].spanned_pages = max_pfn - dma_end_pfn;
        }

        for zone in self.zones.iter_mut() {
            unsafe {
                zone.init_free_areas();
            }
        }

        // 5. Walk available regions and free pages into the buddy system.
        let reserved_ranges: [(u64, u64); 3] = [
            (0, 0x10_0000),                             // First 1 MiB
            (kernel_start, kernel_end),                 // Kernel image
            (mem_map_start as u64, mem_map_end as u64), // mem_map array
        ];

        for region in memory_map.available_regions() {
            let region_start_pfn = align_up(region.base as usize, PAGE_SIZE) / PAGE_SIZE;
            let region_end_pfn = core::cmp::min(
                region.base.saturating_add(region.size) as usize / PAGE_SIZE,
                max_pfn,
            );

            let mut pfn = region_start_pfn;
            while pfn < region_end_pfn && pfn < max_pfn {
                let addr = (pfn * PAGE_SIZE) as u64;
                let addr_end = addr + PAGE_SIZE as u64;

                let reserved = reserved_ranges
                    .iter()
                    .any(|&(rs, re)| addr < re && addr_end > rs);

                if reserved {
                    unsafe {
                        (*pfn_to_page(pfn)).set_reserved();
                    }
                    pfn += 1;
                    continue;
                }

                let zone_end_pfn = match self.pfn_to_zone_idx(pfn) {
                    zone if zone == ZoneType::ZoneDma as usize => {
                        core::cmp::min(region_end_pfn, ZONE_DMA_MAX_PFN)
                    }
                    _ => region_end_pfn,
                };
                let max_order_for_pfn = self.max_free_order(pfn, zone_end_pfn, &reserved_ranges);

                let zone_idx = self.pfn_to_zone_idx(pfn);
                self.zones[zone_idx].managed_pages += 1 << max_order_for_pfn;
                self.zones[zone_idx].present_pages += 1 << max_order_for_pfn;

                unsafe {
                    let page = pfn_to_page(pfn);
                    (*page).set_buddy_order(max_order_for_pfn);
                    self.zones[zone_idx].add_to_free_list(
                        page,
                        max_order_for_pfn,
                        MigrateType::Unmovable,
                        false,
                    );
                }
                self.total_free_pages += 1 << max_order_for_pfn;

                pfn += 1 << max_order_for_pfn;
            }
        }
    }

    /// Calculate the largest order that can be freed starting at `pfn`.
    ///
    /// The block must be properly aligned, fit within the region, and not
    /// overlap any reserved range.
    fn max_free_order(
        &self,
        pfn: usize,
        region_end_pfn: usize,
        reserved_ranges: &[(u64, u64)],
    ) -> usize {
        let mut order = 0;
        while order < MAX_PAGE_ORDER {
            let next_order = order + 1;
            let block_size = 1usize << next_order;
            // Must be aligned to 2^(next_order).
            if pfn & (block_size - 1) != 0 {
                break;
            }
            // Must fit within the region.
            if pfn + block_size > region_end_pfn {
                break;
            }
            if pfn + block_size > self.max_pfn {
                break;
            }
            // Must not overlap any reserved range.
            let block_start = (pfn * PAGE_SIZE) as u64;
            let block_end = ((pfn + block_size) * PAGE_SIZE) as u64;
            let overlaps = reserved_ranges
                .iter()
                .any(|&(rs, re)| block_start < re && block_end > rs);
            if overlaps {
                break;
            }
            order = next_order;
        }
        order
    }

    /// Determine which zone a PFN belongs to.
    #[inline]
    pub fn pfn_to_zone_idx(&self, pfn: usize) -> usize {
        if pfn < ZONE_DMA_MAX_PFN {
            ZoneType::ZoneDma as usize
        } else {
            ZoneType::ZoneNormal as usize
        }
    }

    // -----------------------------------------------------------------------
    // Allocation — Linux mm/page_alloc.c: rmqueue → expand
    // -----------------------------------------------------------------------

    /// Allocate 2^order contiguous pages.
    ///
    /// Searches zones from the preferred zone (derived from GFP flags) upward.
    /// Within each zone, searches from the requested order up to MAX_PAGE_ORDER
    /// for a free block, then splits down via `expand()`.
    ///
    /// Returns a pointer to the first `Page` of the allocated block, or `None`
    /// if no memory is available.
    ///
    /// Ref: Linux mm/page_alloc.c — `rmqueue()`, `get_page_from_freelist()`
    pub fn alloc_pages(&mut self, order: usize, gfp: GfpFlags) -> Option<*mut Page> {
        if order > MAX_PAGE_ORDER {
            return None;
        }

        let preferred_zone = gfp_zone(gfp);

        // Try each zone from the GFP-selected class.  GFP_KERNEL must not dip
        // into ZONE_DMA; that low-memory pool is reserved for explicit DMA
        // callers, matching Linux's zone selection semantics.
        let zone_order: &[usize] = match preferred_zone {
            ZoneType::ZoneDma => &[0, 1],
            ZoneType::ZoneNormal
                if self.zones[ZoneType::ZoneNormal as usize].spanned_pages != 0 =>
            {
                &[1]
            }
            ZoneType::ZoneNormal => &[0],
        };

        for &zone_idx in zone_order {
            if zone_idx >= MAX_NR_ZONES {
                continue;
            }
            // Search from the requested order up to MAX_PAGE_ORDER.
            for current_order in order..NR_PAGE_ORDERS {
                let page = unsafe {
                    self.zones[zone_idx]
                        .get_page_from_free_area(current_order, MigrateType::Unmovable)
                };

                if let Some(page) = page {
                    // Remove from free list.
                    unsafe {
                        self.zones[zone_idx].del_from_free_list(
                            page,
                            current_order,
                            MigrateType::Unmovable,
                        );
                    }

                    // Split excess orders down to the requested order.
                    if current_order > order {
                        unsafe {
                            self.expand(zone_idx, page, order, current_order);
                        }
                    }

                    self.total_free_pages -= 1 << order;

                    // Handle __GFP_ZERO: zero the allocated memory.
                    if gfp & __GFP_ZERO != 0 {
                        let addr = page_to_pfn(page) * PAGE_SIZE;
                        let bytes = (1 << order) * PAGE_SIZE;
                        unsafe {
                            core::ptr::write_bytes(phys_to_virt(addr as u64), 0, bytes);
                        }
                    }

                    unsafe {
                        Self::prepare_allocated_block(page, order);
                    }

                    return Some(page);
                }
            }
        }

        None
    }

    /// Split a block from `high` order down to `low` order.
    ///
    /// The block at `page` is order `high`.  We want order `low`.
    /// We repeatedly halve the block: the upper half goes back to the
    /// free list at one order lower, until we reach `low`.
    ///
    /// Ref: Linux mm/page_alloc.c:1732-1758 — `expand()`
    ///
    /// # Safety
    /// - `page` must be the head of a valid block at order `high`.
    /// - All pages in the block must have initialized `lru` fields.
    unsafe fn expand(&mut self, zone_idx: usize, page: *mut Page, low: usize, high: usize) {
        let mut size = 1usize << high;
        let mut current = high;

        while current > low {
            current -= 1;
            size >>= 1;

            // The upper-half buddy starts at page[size].
            unsafe {
                let buddy = page.add(size);
                (*buddy).set_buddy_order(current);
                self.zones[zone_idx].add_to_free_list(
                    buddy,
                    current,
                    MigrateType::Unmovable,
                    false,
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Deallocation — Linux mm/page_alloc.c: free_one_page → __free_one_page
    // -----------------------------------------------------------------------

    /// Free a 2^order block of pages back to the buddy system.
    ///
    /// Attempts to coalesce with the buddy at each order, merging upward
    /// until the buddy is not free or MAX_PAGE_ORDER is reached.
    ///
    /// Ref: Linux mm/page_alloc.c:978-1064 — `__free_one_page()`
    pub fn free_pages(&mut self, page: *mut Page, order: usize) {
        let pfn = page_to_pfn(page);
        let zone_idx = self.pfn_to_zone_idx(pfn);

        self.free_one_page(page, pfn, zone_idx, order);
        self.total_free_pages += 1 << order;
    }

    /// Release one boot-reserved page into the buddy allocator.
    ///
    /// Linux's `free_reserved_area()` clears PG_reserved, accounts the page as
    /// managed memory, then frees it. The ordinary `free_pages()` path is kept
    /// for pages that were already managed and allocated from the buddy.
    pub fn free_reserved_page(&mut self, page: *mut Page) {
        if page.is_null() {
            return;
        }
        unsafe {
            if (*page).is_buddy() {
                return;
            }
            let pfn = page_to_pfn(page);
            let zone_idx = self.pfn_to_zone_idx(pfn);
            if (*page).is_reserved() {
                (*page).clear_flag(crate::mm::page_flags::PG_RESERVED);
                self.zones[zone_idx].managed_pages += 1;
                self.zones[zone_idx].present_pages += 1;
            }
            (*page)._refcount.store(0, Ordering::Relaxed);
            self.free_one_page(page, pfn, zone_idx, 0);
            self.total_free_pages += 1;
        }
    }

    /// Internal: free one block and coalesce with buddies.
    ///
    /// Ref: Linux mm/page_alloc.c:978-1064
    fn free_one_page(&mut self, page: *mut Page, pfn: usize, zone_idx: usize, order: usize) {
        let mut order = order;
        let mut pfn = pfn;
        let mut page = page;

        while order < MAX_PAGE_ORDER {
            let buddy_pfn = find_buddy_pfn(pfn, order);

            // Buddy PFN must be within the mem_map.
            let count = MEM_MAP_COUNT.load(Ordering::Relaxed);
            let base = MEM_MAP_BASE_PFN.load(Ordering::Relaxed);
            if buddy_pfn < base || buddy_pfn >= base + count {
                break;
            }

            let buddy = pfn_to_page(buddy_pfn);

            // Buddy must be in the same zone.
            if self.pfn_to_zone_idx(buddy_pfn) != zone_idx {
                break;
            }

            // Buddy must be free (PageBuddy) and at the same order.
            // Ref: Linux mm/internal.h:717-736 — page_is_buddy()
            unsafe {
                if !(*buddy).is_buddy() || (*buddy).buddy_order() != order {
                    break;
                }

                // Remove buddy from its free list — it will merge with us.
                self.zones[zone_idx].del_from_free_list(buddy, order, MigrateType::Unmovable);
            }

            // Merge: the combined block starts at the lower PFN.
            let combined_pfn = pfn & buddy_pfn;
            page = pfn_to_page(combined_pfn);
            pfn = combined_pfn;
            order += 1;
        }

        // Mark the final merged page as buddy at the resulting order and
        // add it to the free list.
        unsafe {
            (*page).set_buddy_order(order);
            self.zones[zone_idx].add_to_free_list(page, order, MigrateType::Unmovable, false);
        }
    }

    // -----------------------------------------------------------------------
    // Compatibility API — same interface as the old BitmapFrameAllocator
    // so the heap allocator (GlobalAlloc) continues to work unchanged.
    // -----------------------------------------------------------------------

    /// Allocate a single physical frame (order-0 page).
    ///
    /// Returns the PFN wrapped in `PhysFrame`, or `None` on OOM.
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        use crate::mm::page_flags::GFP_KERNEL;
        self.alloc_pages(0, GFP_KERNEL)
            .map(|page| PhysFrame(page_to_pfn(page) as u64))
    }

    /// Allocate `count` contiguous physical frames.
    ///
    /// Rounds up to the nearest power-of-two order and allocates via the
    /// buddy system.  Returns the PFN of the first frame.
    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrame> {
        use crate::mm::page_flags::GFP_KERNEL;
        if count == 0 {
            return None;
        }
        let order = if count == 1 {
            0
        } else {
            // ceil_log2(count)
            (usize::BITS - (count - 1).leading_zeros()) as usize
        };
        self.alloc_pages(order, GFP_KERNEL)
            .map(|page| PhysFrame(page_to_pfn(page) as u64))
    }

    /// Deallocate a single physical frame (order-0).
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let page = pfn_to_page(frame.0 as usize);
        self.free_pages(page, 0);
    }

    // -----------------------------------------------------------------------
    // Statistics
    // -----------------------------------------------------------------------

    /// Total free pages across all zones.
    pub fn free_count(&self) -> usize {
        self.total_free_pages
    }

    /// Free pages in a specific zone.
    pub fn zone_free_count(&self, zone_type: ZoneType) -> usize {
        self.zones[zone_type as usize].free_pages()
    }

    /// Total managed pages across all zones.
    pub fn total_managed(&self) -> usize {
        self.zones.iter().map(|z| z.managed_pages).sum()
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Align `value` up to the next multiple of `align`.
/// `align` must be a power of two.
#[inline]
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[inline]
const fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

#[inline]
const fn range_overlaps(
    start: usize,
    end: usize,
    reserved_start: usize,
    reserved_end: usize,
) -> bool {
    start < reserved_end && reserved_start < end
}

fn highest_boot_mapped_available_pfn(memory_map: &MemoryMap) -> usize {
    let mut highest_addr = 0u64;

    for region in memory_map.available_regions() {
        if region.base >= BOOT_DIRECT_MAP_BYTES_U64 {
            continue;
        }

        let mapped_end = core::cmp::min(
            region.base.saturating_add(region.size),
            BOOT_DIRECT_MAP_BYTES_U64,
        );
        highest_addr = core::cmp::max(highest_addr, mapped_end);
    }

    (highest_addr as usize + PAGE_SIZE - 1) / PAGE_SIZE
}

fn choose_mem_map_start(
    memory_map: &MemoryMap,
    kernel_start: u64,
    kernel_end: u64,
    mem_map_bytes: usize,
) -> Option<usize> {
    let needed = align_up(mem_map_bytes, PAGE_SIZE);
    let low_reserved_end = 0x10_0000usize;
    let kernel_start = kernel_start as usize;
    let kernel_end = kernel_end as usize;

    for region in memory_map.available_regions() {
        if region.base >= BOOT_DIRECT_MAP_BYTES_U64 {
            continue;
        }

        let mut candidate = align_up(region.base as usize, PAGE_SIZE);
        let region_end = align_down(
            core::cmp::min(
                region.base.saturating_add(region.size),
                BOOT_DIRECT_MAP_BYTES_U64,
            ) as usize,
            PAGE_SIZE,
        );

        loop {
            let Some(candidate_end) = candidate.checked_add(needed) else {
                break;
            };
            if candidate_end > region_end {
                break;
            }

            if range_overlaps(candidate, candidate_end, 0, low_reserved_end) {
                candidate = align_up(low_reserved_end, PAGE_SIZE);
                continue;
            }
            if range_overlaps(candidate, candidate_end, kernel_start, kernel_end) {
                candidate = align_up(kernel_end, PAGE_SIZE);
                continue;
            }

            return Some(candidate);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Global buddy allocator — accessed from slab, vmalloc, and tests.
//
// The allocator is stored in a spin::Mutex so any subsystem can call
// `with_global_buddy` to allocate or free pages without holding a reference
// to a local BuddyAllocator instance.
//
// IMPORTANT: `global_buddy_init` must be called ONCE during boot (after the
// physical memory map is available) before any allocations are attempted.
//
// Ref: Linux mm/page_alloc.c — init_per_zone_wmark_min, free_area_init
// ---------------------------------------------------------------------------

/// The global buddy allocator singleton.
///
/// Stored in a spin::Mutex to allow safe concurrent access.  The allocator
/// is initialised with `BuddyAllocator::new()` (all-zero, non-functional)
/// and made live by calling `global_buddy_init()`.
static GLOBAL_BUDDY: spin::Mutex<BuddyAllocator> = spin::Mutex::new(BuddyAllocator::new());

/// Set to `true` after `global_buddy_init` has completed successfully.
static BUDDY_READY: AtomicBool = AtomicBool::new(false);

/// Initialize the global buddy allocator in-place (no copy/move after init).
///
/// Must be called exactly once, before any call to `with_global_buddy`.
/// Equivalent to Linux's `zone_sizes_init` + `free_area_init`.
///
/// # Safety
/// Same requirements as `BuddyAllocator::init`.
pub unsafe fn global_buddy_init(memory_map: &MemoryMap, kernel_start: u64, kernel_end: u64) {
    // Lock the mutex and initialise the allocator in-place.
    // init() sets up self-referential ListHead pointers — doing it through
    // the lock guard keeps the allocator at its final address (no move).
    let mut guard = GLOBAL_BUDDY.lock();
    unsafe {
        guard.init(memory_map, kernel_start, kernel_end);
    }
    sync_totalram_with_buddy(&guard);
    // Release lock before setting BUDDY_READY so readers always see a fully
    // initialised allocator (Release/Acquire pair).
    drop(guard);
    BUDDY_READY.store(true, Ordering::Release);
}

/// Publish the pages this allocator manages as `totalram_pages`.
///
/// Linux's `memblock_free_all()` (vendor/linux/mm/memblock.c) ends with
/// `totalram_pages_add(pages)` for every page released into the buddy;
/// `sysconf(_SC_PHYS_PAGES)` (glibc → `sysinfo(2)`) and `/proc/meminfo`
/// `MemTotal` read that counter. Without it, sysinfo reports 0 bytes of
/// RAM and systemd's `physical_memory()` assertion aborts PID 1.
fn sync_totalram_with_buddy(buddy: &BuddyAllocator) {
    let managed = buddy.total_managed() as i64;
    let current = crate::mm::mm_public::totalram_pages() as i64;
    crate::mm::mm_public::totalram_pages_add(managed - current);
}

/// Run a closure with a mutable reference to the global buddy allocator.
///
/// Panics if called before `global_buddy_init`.
///
/// # Example
/// ```ignore
/// let page = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL));
/// ```
/// Returns `true` if the global buddy allocator has been initialised.
pub fn is_buddy_ready() -> bool {
    BUDDY_READY.load(Ordering::Acquire)
}

pub fn with_global_buddy<F, R>(f: F) -> R
where
    F: FnOnce(&mut BuddyAllocator) -> R,
{
    assert!(
        BUDDY_READY.load(Ordering::Acquire),
        "with_global_buddy: buddy not initialized"
    );
    // IRQ-safe like Linux's zone locks: the LAPIC timer ISR can allocate
    // (scheduler_tick → CFS task_tick → kmalloc → new slab → alloc_pages), so a
    // tick landing while a task holds the buddy lock would make the ISR's nested
    // allocation spin on the lock forever.  Disable interrupts for the critical
    // section; the lock guard is released (end of the statement) before IRQs are
    // restored.
    #[cfg(not(test))]
    {
        let flags = crate::kernel::locking::irqflags::local_irq_save();
        let r = f(&mut GLOBAL_BUDDY.lock());
        crate::kernel::locking::irqflags::local_irq_restore(flags);
        r
    }
    #[cfg(test)]
    {
        f(&mut GLOBAL_BUDDY.lock())
    }
}

/// Install a test buddy allocator as the global buddy, initializing it in-place.
///
/// This helper avoids the self-referential pointer problem that would arise from
/// constructing a `BuddyAllocator` locally and then moving it into `GLOBAL_BUDDY`.
/// Instead it initializes the global allocator in-place and frees all pages in
/// `[base_pfn, base_pfn + n_pages)` into it.
///
/// # Safety
/// - `set_mem_map` must have been called first for the same range.
/// - Must be called from a serialized test context (e.g. under `GLOBAL_HW_TEST_LOCK`).
/// - Only available in test builds.
#[cfg(test)]
pub unsafe fn install_test_buddy(base_pfn: usize, n_pages: usize) {
    let mut guard = GLOBAL_BUDDY.lock();
    guard.max_pfn = base_pfn + n_pages;
    guard.total_free_pages = 0;
    guard.zones[0].zone_start_pfn = base_pfn;
    guard.zones[0].spanned_pages = n_pages;
    guard.zones[0].managed_pages = 0;
    guard.zones[1].zone_start_pfn = base_pfn + n_pages;
    guard.zones[1].spanned_pages = 0;
    guard.zones[1].managed_pages = 0;
    for zone in guard.zones.iter_mut() {
        // Reset per-order free counts before re-initializing list sentinels.
        for area in zone.free_area.iter_mut() {
            area.nr_free = 0;
        }
        unsafe { zone.init_free_areas() };
    }
    for pfn in base_pfn..base_pfn + n_pages {
        let page = pfn_to_page(pfn);
        guard.free_pages(page, 0);
    }
    // Keep the boot-path invariant in tests: totalram tracks the buddy's
    // managed pages (see `sync_totalram_with_buddy`).
    sync_totalram_with_buddy(&guard);
    drop(guard);
    BUDDY_READY.store(true, Ordering::Release);
}

#[cfg(test)]
pub fn reset_buddy_state_for_test() {
    *GLOBAL_BUDDY.lock() = BuddyAllocator::new();
    MEM_MAP_PTR.store(core::ptr::null_mut(), Ordering::Relaxed);
    MEM_MAP_BASE_PFN.store(0, Ordering::Relaxed);
    MEM_MAP_COUNT.store(0, Ordering::Relaxed);
    BUDDY_READY.store(false, Ordering::Release);
}

// ---------------------------------------------------------------------------
// OOM-aware allocation path — Milestone 18
// ---------------------------------------------------------------------------

/// Allocate `2^order` pages, triggering reclaim and the OOM killer on failure.
///
/// This is the "slowpath" equivalent of Linux's `__alloc_pages_slowpath()`:
/// 1. Fast path: try the buddy allocator directly.
/// 2. Reclaim path: call `reclaim_pages()` then retry (lock is released between
///    calls — no deadlock with the buddy lock).
/// 3. OOM path: invoke `out_of_memory()` and retry once more.
///
/// Callers that already hold the buddy lock must use `alloc_pages()` directly.
///
/// Ref: Linux `mm/page_alloc.c` — `__alloc_pages_slowpath()`
pub fn alloc_pages_or_oom(order: usize, gfp: GfpFlags) -> Option<*mut Page> {
    // Fast path.
    if let Some(p) = with_global_buddy(|b| b.alloc_pages(order, gfp)) {
        return Some(p);
    }
    // Reclaim path: release lock, reclaim, retry.
    crate::mm::reclaim::reclaim_pages(1usize << order);
    if let Some(p) = with_global_buddy(|b| b.alloc_pages(order, gfp)) {
        return Some(p);
    }
    // OOM path: invoke killer, retry.
    let mut oc = crate::mm::oom::OomControl::new(gfp, order as i32);
    crate::mm::oom::out_of_memory(&mut oc);
    with_global_buddy(|b| b.alloc_pages(order, gfp))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
pub mod tests {
    extern crate alloc;
    use alloc::boxed::Box;
    extern crate std;
    // use std::sync::Mutex; // No longer needed

    use super::*;
    use crate::mm::page::Page;
    use crate::mm::page_flags::{GFP_DMA, GFP_KERNEL};
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    // -----------------------------------------------------------------------
    // Test scaffolding
    //
    // All buddy tests share the global MEM_MAP_PTR / MEM_MAP_BASE_PFN /
    // MEM_MAP_COUNT atomics, so tests must not run in parallel.  This mutex
    // serializes access.
    // -----------------------------------------------------------------------

    /// Number of pages in the test mem_map.
    /// 1024 pages = 4 MiB — enough for one max-order block.
    const TEST_PAGES: usize = 1024;

    /// Helper: create a test environment with `TEST_PAGES` pages.
    /// Returns (Box<BuddyAllocator>, mem_map array).
    ///
    /// Both the allocator and pages are heap-allocated (Box) so that
    /// self-referential ListHead pointers remain valid (no move after init).
    ///
    /// The allocator starts empty — call `test_free_range` to populate it.
    unsafe fn make_test_env() -> (Box<BuddyAllocator>, Box<[Page; TEST_PAGES]>) {
        // Use Box to avoid stack overflow with 1024 × 64 = 64 KiB.
        let mut pages = Box::new([const { Page::new() }; TEST_PAGES]);
        for page in pages.iter_mut() {
            unsafe {
                page.init_lru();
            }
        }

        // Install as global mem_map.
        unsafe {
            set_mem_map(pages.as_mut_ptr(), 0, TEST_PAGES);
        }

        // Box the allocator FIRST, then init free areas in-place on the heap.
        // This prevents self-referential ListHead pointers from being
        // invalidated by a stack→heap move.
        let mut alloc = Box::new(BuddyAllocator::new());
        alloc.max_pfn = TEST_PAGES;

        // Setup zones: all pages in zone 0 (DMA) since PFNs 0-1023 < ZONE_DMA_MAX_PFN.
        alloc.zones[0].zone_start_pfn = 0;
        alloc.zones[0].spanned_pages = TEST_PAGES;
        alloc.zones[1].zone_start_pfn = TEST_PAGES;
        alloc.zones[1].spanned_pages = 0;

        for zone in alloc.zones.iter_mut() {
            unsafe {
                zone.init_free_areas();
            }
        }

        (alloc, pages)
    }

    /// Free a range of pages into the buddy system one at a time.
    /// Pages coalesce automatically via buddy merging.
    unsafe fn test_free_range(alloc: &mut BuddyAllocator, start_pfn: usize, count: usize) {
        for pfn in start_pfn..start_pfn + count {
            let page = pfn_to_page(pfn);
            alloc.free_pages(page, 0);
        }
    }

    // -----------------------------------------------------------------------
    // PFN / buddy arithmetic tests
    // -----------------------------------------------------------------------

    #[test]
    fn buddy_init_publishes_totalram_pages_like_memblock_free_all() {
        // Linux: memblock_free_all() → totalram_pages_add(freed pages);
        // glibc sysconf(_SC_PHYS_PAGES) divides that through sysinfo(2).
        // systemd PID 1 asserts the result is > 0 (physical_memory()).
        let _guard = crate::mm::test_lock::GLOBAL_HW_TEST_LOCK.lock();
        let mut buddy = BuddyAllocator::new();
        buddy.zones[0].managed_pages = 300;
        buddy.zones[1].managed_pages = 7_900;
        sync_totalram_with_buddy(&buddy);
        assert_eq!(crate::mm::mm_public::totalram_pages(), 8_200);
        // Re-sync is idempotent (re-init in tests must not double-count).
        sync_totalram_with_buddy(&buddy);
        assert_eq!(crate::mm::mm_public::totalram_pages(), 8_200);
    }

    #[test]
    fn buddy_pfn_xor_calculation() {
        // Order 0: buddies are adjacent pages.
        assert_eq!(find_buddy_pfn(0, 0), 1);
        assert_eq!(find_buddy_pfn(1, 0), 0);

        // Order 1: buddy is 2 pages away.
        assert_eq!(find_buddy_pfn(0, 1), 2);
        assert_eq!(find_buddy_pfn(2, 1), 0);

        // Order 2: buddy is 4 pages away.
        assert_eq!(find_buddy_pfn(4, 2), 0);
        assert_eq!(find_buddy_pfn(0, 2), 4);

        // Order 10: buddy is 1024 pages away.
        assert_eq!(find_buddy_pfn(0, 10), 1024);
        assert_eq!(find_buddy_pfn(1024, 10), 0);
    }

    #[test]
    fn mem_map_placement_skips_reserved_initrd_gap() {
        let mut map = MemoryMap::new();
        map.regions_mut()[0] = crate::mm::region::PhysRegion {
            base: 0x100000,
            size: 0x3f00000,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.set_count(1);
        map.mark_reserved(0x200000, 0x800000);

        let start = choose_mem_map_start(&map, 0x100000, 0x180000, 0x300000).unwrap();

        assert!(
            start >= 0xa00000,
            "mem_map must be placed after the reserved initrd, got {start:#x}"
        );
    }

    #[test]
    fn highest_available_pfn_is_capped_to_boot_direct_map() {
        let mut map = MemoryMap::new();
        map.regions_mut()[0] = crate::mm::region::PhysRegion {
            base: 0x100000,
            size: 0x300000,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.regions_mut()[1] = crate::mm::region::PhysRegion {
            base: BOOT_DIRECT_MAP_BYTES_U64 + 0x100000,
            size: 0x400000,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.set_count(2);

        assert_eq!(
            highest_boot_mapped_available_pfn(&map),
            0x400000 / PAGE_SIZE
        );
    }

    #[test]
    fn highest_available_pfn_clips_region_spanning_boot_direct_map() {
        let mut map = MemoryMap::new();
        map.regions_mut()[0] = crate::mm::region::PhysRegion {
            base: BOOT_DIRECT_MAP_BYTES_U64 - PAGE_SIZE as u64,
            size: (PAGE_SIZE * 4) as u64,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.set_count(1);

        assert_eq!(
            highest_boot_mapped_available_pfn(&map),
            BOOT_DIRECT_MAP_BYTES / PAGE_SIZE
        );
    }

    #[test]
    fn mem_map_placement_ignores_unmapped_high_regions() {
        let mut map = MemoryMap::new();
        map.regions_mut()[0] = crate::mm::region::PhysRegion {
            base: BOOT_DIRECT_MAP_BYTES_U64 + 0x200000,
            size: 0x1000000,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.regions_mut()[1] = crate::mm::region::PhysRegion {
            base: 0x100000,
            size: 0x800000,
            region_type: crate::mm::region::RegionType::Available,
        };
        map.set_count(2);

        let start = choose_mem_map_start(&map, 0x100000, 0x180000, PAGE_SIZE).unwrap();

        assert!(start < BOOT_DIRECT_MAP_BYTES);
        assert_eq!(start, 0x180000);
    }

    #[test]
    fn max_free_order_stops_before_kernel_and_zone_boundaries() {
        let mut alloc = BuddyAllocator::new();
        alloc.max_pfn = 0x40000;
        let reserved = [
            (0, 0x10_0000),
            (0x20_0000, 0xa1_5014),
            (0xa1_6000, 0x14_16000),
        ];

        assert_eq!(
            alloc.max_free_order(0x100, ZONE_DMA_MAX_PFN, &reserved),
            8,
            "1MiB..2MiB should be the largest free block before the kernel image"
        );
        assert_eq!(
            alloc.max_free_order(0xfff, ZONE_DMA_MAX_PFN, &reserved),
            0,
            "a block starting below 16MiB must not grow across the DMA/Normal boundary"
        );
    }

    #[test]
    fn free_reserved_page_accounts_managed_memory() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            let page = pfn_to_page(4);
            (*page).set_reserved();

            alloc.free_reserved_page(page);

            assert!(!(*page).is_reserved());
            assert!((*page).is_buddy());
            assert_eq!(alloc.free_count(), 1);
            assert_eq!(alloc.total_managed(), 1);
        }
    }

    #[test]
    fn pfn_to_page_roundtrip() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (_alloc, _pages) = make_test_env();
            for pfn in [0, 1, 512, TEST_PAGES - 1] {
                let page = pfn_to_page(pfn);
                assert_eq!(page_to_pfn(page), pfn);
            }
        }
    }

    #[test]
    fn page_to_pfn_correct() {
        let _g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (_alloc, pages) = make_test_env();
            for i in 0..10 {
                let page_ptr = &pages[i] as *const Page;
                assert_eq!(page_to_pfn(page_ptr), i);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Coalescing tests
    // -----------------------------------------------------------------------

    #[test]
    fn free_coalesces_order0_pair() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            // Free PFN 0 and PFN 1 — they should merge into one order-1 block.
            let p0 = pfn_to_page(0);
            alloc.free_pages(p0, 0);
            assert_eq!(alloc.free_count(), 1);
            assert_eq!(alloc.zones[0].free_area[0].nr_free, 1);

            let p1 = pfn_to_page(1);
            alloc.free_pages(p1, 0);
            // After merging: one order-1 block at PFN 0.
            assert_eq!(alloc.free_count(), 2); // 2 pages total
            assert_eq!(alloc.zones[0].free_area[0].nr_free, 0);
            assert_eq!(alloc.zones[0].free_area[1].nr_free, 1);
        }
    }

    #[test]
    fn free_coalesces_to_max_order() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            // Free all 1024 pages → should coalesce into one order-10 block.
            test_free_range(&mut alloc, 0, 1024);
            assert_eq!(alloc.free_count(), 1024);
            assert_eq!(alloc.zones[0].free_area[10].nr_free, 1);
            // All lower orders should be empty.
            for order in 0..10 {
                assert_eq!(
                    alloc.zones[0].free_area[order].nr_free, 0,
                    "order {} should be empty after full coalesce",
                    order
                );
            }
        }
    }

    #[test]
    fn free_no_coalesce_when_buddy_allocated() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            // Free only PFN 0 — PFN 1 (its buddy) is not free.
            let p0 = pfn_to_page(0);
            alloc.free_pages(p0, 0);
            assert_eq!(alloc.free_count(), 1);
            assert_eq!(alloc.zones[0].free_area[0].nr_free, 1);
            // No coalescing should have happened.
            assert_eq!(alloc.zones[0].free_area[1].nr_free, 0);
        }
    }

    #[test]
    fn free_stops_at_max_order() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            // Free all 1024 pages — should stop at order 10.
            test_free_range(&mut alloc, 0, 1024);
            assert_eq!(alloc.zones[0].free_area[10].nr_free, 1);
            // If we had > 1024 pages, we could test that it doesn't go to order 11,
            // but with TEST_PAGES=1024, the natural limit applies.
        }
    }

    #[test]
    fn free_increments_zone_free_count() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            assert_eq!(alloc.zones[0].free_pages(), 0);
            test_free_range(&mut alloc, 0, 4);
            // 4 pages = 1 order-2 block (PFN 0-3 coalesce fully).
            assert_eq!(alloc.zones[0].free_pages(), 4);
        }
    }

    // -----------------------------------------------------------------------
    // Allocation tests
    // -----------------------------------------------------------------------

    #[test]
    fn alloc_order0_single_page() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 2);
            // Should have 1 order-1 block.
            let page = alloc.alloc_pages(0, GFP_KERNEL).unwrap();
            assert!(!(*page).is_buddy());
            assert_eq!(alloc.free_count(), 1);
        }
    }

    #[test]
    fn alloc_splits_higher_order() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            // Free 4 pages → order-2 block.
            test_free_range(&mut alloc, 0, 4);
            assert_eq!(alloc.zones[0].free_area[2].nr_free, 1);

            // Alloc order 0 → splits order-2 into order-1 + order-0 + order-0.
            let _p = alloc.alloc_pages(0, GFP_KERNEL).unwrap();
            assert_eq!(alloc.free_count(), 3);
            // Should leave: 1 order-1 block + 1 order-0 block = 3 pages.
            assert_eq!(alloc.zones[0].free_area[0].nr_free, 1);
            assert_eq!(alloc.zones[0].free_area[1].nr_free, 1);
            assert_eq!(alloc.zones[0].free_area[2].nr_free, 0);
        }
    }

    #[test]
    fn alloc_order10_full_block() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 1024);
            let page = alloc.alloc_pages(10, GFP_KERNEL).unwrap();
            assert_eq!(page_to_pfn(page), 0);
            assert_eq!(alloc.free_count(), 0);
        }
    }

    #[test]
    fn alloc_order_above_max_returns_none() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 1024);
            assert!(
                alloc.alloc_pages(MAX_PAGE_ORDER + 1, GFP_KERNEL).is_none(),
                "oversized orders must fail, not panic"
            );
        }
    }

    #[test]
    fn alloc_returns_none_when_empty() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            assert!(alloc.alloc_pages(0, GFP_KERNEL).is_none());
        }
    }

    #[test]
    fn alloc_free_roundtrip_preserves_count() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 1024);
            let initial = alloc.free_count();
            assert_eq!(initial, 1024);

            // Allocate some pages of various orders.
            let p0 = alloc.alloc_pages(0, GFP_KERNEL).unwrap();
            let p3 = alloc.alloc_pages(3, GFP_KERNEL).unwrap();
            let p5 = alloc.alloc_pages(5, GFP_KERNEL).unwrap();
            assert_eq!(
                alloc.free_count(),
                1024 - 1 - 8 - 32,
                "free count after allocs"
            );

            // Free them back.
            alloc.free_pages(p0, 0);
            alloc.free_pages(p3, 3);
            alloc.free_pages(p5, 5);
            assert_eq!(alloc.free_count(), initial, "free count after round-trip");
        }
    }

    // -----------------------------------------------------------------------
    // Zone-aware allocation tests
    // -----------------------------------------------------------------------

    #[test]
    fn alloc_respects_zone_dma() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            // Setup: PFN 0-7 in ZONE_DMA, PFN 8-1023 in ZONE_NORMAL.
            let (mut alloc, _pages) = make_test_env();
            // Reconfigure zones for this test.
            alloc.zones[0].zone_start_pfn = 0;
            alloc.zones[0].spanned_pages = 8;
            alloc.zones[1].zone_start_pfn = 8;
            alloc.zones[1].spanned_pages = TEST_PAGES - 8;

            // Free PFN 0-7 into DMA, PFN 8-15 into Normal.
            for pfn in 0..8 {
                let page = pfn_to_page(pfn);
                alloc.free_one_page(page, pfn, 0, 0);
                alloc.total_free_pages += 1;
            }
            for pfn in 8..16 {
                let page = pfn_to_page(pfn);
                alloc.free_one_page(page, pfn, 1, 0);
                alloc.total_free_pages += 1;
            }

            // GFP_DMA should allocate from ZONE_DMA (PFN < 8).
            let page = alloc.alloc_pages(0, GFP_DMA).unwrap();
            let pfn = page_to_pfn(page);
            assert!(
                pfn < 8,
                "GFP_DMA should allocate from ZONE_DMA, got PFN {}",
                pfn
            );
        }
    }

    #[test]
    fn gfp_kernel_does_not_fallback_to_dma_when_normal_zone_exists() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        const ZONE_SPLIT_TEST_PAGES: usize = ZONE_DMA_MAX_PFN + 16;
        unsafe {
            let mut pages = Box::new([const { Page::new() }; ZONE_SPLIT_TEST_PAGES]);
            for page in pages.iter_mut() {
                page.init_lru();
            }
            set_mem_map(pages.as_mut_ptr(), 0, ZONE_SPLIT_TEST_PAGES);

            let mut alloc = Box::new(BuddyAllocator::new());
            alloc.max_pfn = ZONE_SPLIT_TEST_PAGES;
            alloc.zones[0].zone_start_pfn = 0;
            alloc.zones[0].spanned_pages = ZONE_DMA_MAX_PFN;
            alloc.zones[1].zone_start_pfn = ZONE_DMA_MAX_PFN;
            alloc.zones[1].spanned_pages = 16;
            for zone in alloc.zones.iter_mut() {
                zone.init_free_areas();
            }

            alloc.free_pages(pfn_to_page(1), 0);
            alloc.free_pages(pfn_to_page(ZONE_DMA_MAX_PFN), 0);

            let page = alloc.alloc_pages(0, GFP_KERNEL).expect("normal page");
            assert_eq!(page_to_pfn(page), ZONE_DMA_MAX_PFN);
            assert!(
                alloc.alloc_pages(0, GFP_KERNEL).is_none(),
                "GFP_KERNEL must not consume ZONE_DMA while a Normal zone exists"
            );
            assert!(
                page_to_pfn(alloc.alloc_pages(0, GFP_DMA).expect("dma page")) < ZONE_DMA_MAX_PFN
            );
        }
    }

    // -----------------------------------------------------------------------
    // Compatibility API tests
    // -----------------------------------------------------------------------

    #[test]
    fn allocate_frame_returns_phys_frame() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 4);
            let frame = alloc.allocate_frame().unwrap();
            assert!(frame.0 < TEST_PAGES as u64);
        }
    }

    #[test]
    fn allocate_contiguous_returns_aligned() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 32);
            // Request 16 contiguous frames → needs order 4 (2^4 = 16).
            let frame = alloc.allocate_contiguous(16).unwrap();
            // PFN must be aligned to 16.
            assert_eq!(frame.0 % 16, 0, "contiguous alloc must be order-aligned");
            assert_eq!(
                alloc.free_count(),
                32 - 16,
                "free count after contiguous alloc"
            );
        }
    }

    #[test]
    fn deallocate_frame_returns_to_buddy() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            test_free_range(&mut alloc, 0, 4);
            let initial = alloc.free_count();
            let frame = alloc.allocate_frame().unwrap();
            assert_eq!(alloc.free_count(), initial - 1);
            alloc.deallocate_frame(frame);
            assert_eq!(alloc.free_count(), initial);
        }
    }

    #[test]
    fn allocate_contiguous_zero_returns_none() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (mut alloc, _pages) = make_test_env();
            assert!(alloc.allocate_contiguous(0).is_none());
        }
    }

    // -----------------------------------------------------------------------
    // mem_map tests
    // -----------------------------------------------------------------------

    #[test]
    fn mem_map_count_matches() {
        let _g = crate::mm::buddy::tests::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        unsafe {
            let (_alloc, _pages) = make_test_env();
            assert_eq!(MEM_MAP_COUNT.load(Ordering::Relaxed), TEST_PAGES);
        }
    }
}
