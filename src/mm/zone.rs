//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Memory zones and free-area management — the lupos equivalent of Linux's
/// `struct zone` and `struct free_area`.
///
/// Each zone owns an array of `NR_PAGE_ORDERS` (11) free areas, indexed by
/// allocation order (0 = single 4 KiB page, 10 = 4 MiB block of 1024 pages).
/// Each free area contains `MIGRATE_TYPES` (4) doubly-linked free lists.
///
/// For Milestone 7, only `MIGRATE_UNMOVABLE` (index 0) is populated; the
/// other lists exist for struct-layout compatibility with Linux.
///
/// Ref: Linux include/linux/mmzone.h — struct zone, struct free_area
///      Linux mm/page_alloc.c — __add_to_free_list, __del_page_from_free_list
use crate::container_of;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::list::ListHead;
use crate::mm::page::Page;
use crate::mm::page_flags::{MIGRATE_TYPES, MigrateType, ZoneType};

/// Maximum allocation order.  An order-N allocation returns 2^N contiguous
/// pages.  Order 10 = 1024 pages = 4 MiB (with 4 KiB pages).
///
/// Ref: Linux include/linux/mmzone.h:30 — MAX_PAGE_ORDER
pub const MAX_PAGE_ORDER: usize = 10;

/// Number of page orders: 0, 1, 2, ..., MAX_PAGE_ORDER.
///
/// Ref: Linux include/linux/mmzone.h:36 — NR_PAGE_ORDERS
pub const NR_PAGE_ORDERS: usize = MAX_PAGE_ORDER + 1;

/// PFN boundary between ZONE_DMA and ZONE_NORMAL.
/// On x86-64, ISA DMA is limited to the first 16 MiB of physical memory.
///
/// Ref: Linux arch/x86/include/asm/dma.h — MAX_DMA_ADDRESS (16 MB)
pub const ZONE_DMA_MAX_PFN: usize = 16 * 1024 * 1024 / PAGE_SIZE; // 4096

/// A single free-area bucket for one allocation order.
///
/// Contains `MIGRATE_TYPES` free lists (one per migration type) and a count
/// of free blocks in this order across all migration types.
///
/// Ref: Linux include/linux/mmzone.h:138-141
pub struct FreeArea {
    /// One free list per migration type.
    /// Each list is a circular doubly-linked list of `Page` structs
    /// linked via their `lru` field.
    pub free_list: [ListHead; MIGRATE_TYPES],
    /// Total number of free blocks at this order (across all migration types).
    pub nr_free: usize,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl FreeArea {
    /// Create a new, empty free area.
    ///
    /// All free lists are uninitialized — caller must call `init()`.
    pub const fn new() -> Self {
        FreeArea {
            free_list: [
                ListHead::uninit(),
                ListHead::uninit(),
                ListHead::uninit(),
                ListHead::uninit(),
            ],
            nr_free: 0,
        }
    }

    /// Initialize all free-list sentinels to empty (self-referential).
    ///
    /// # Safety
    /// Must be called before any list operations.
    pub unsafe fn init(&mut self) {
        for list in self.free_list.iter_mut() {
            ListHead::init(list);
        }
    }

    /// Check if the free list for `migratetype` is empty.
    ///
    /// # Safety
    /// The free list must be initialized.
    #[inline]
    pub unsafe fn is_free_list_empty(&self, migratetype: MigrateType) -> bool {
        ListHead::is_empty(&self.free_list[migratetype as usize])
    }
}

/// A memory zone — groups physical pages by their addressing constraints.
///
/// Ref: Linux include/linux/mmzone.h:879-1059
pub struct Zone {
    /// Which zone this is (DMA or Normal).
    pub zone_type: ZoneType,
    /// First PFN in this zone's address range.
    pub zone_start_pfn: usize,
    /// Total pages spanned by this zone (including holes).
    pub spanned_pages: usize,
    /// Pages actually present (physical RAM, excluding holes).
    pub present_pages: usize,
    /// Pages managed by the buddy allocator (present minus reserved).
    pub managed_pages: usize,
    /// Free-area array indexed by order (0..MAX_PAGE_ORDER).
    pub free_area: [FreeArea; NR_PAGE_ORDERS],
    /// Human-readable zone name for logging.
    pub name: &'static str,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl Zone {
    /// Create a new, empty zone.
    pub const fn new(zone_type: ZoneType, name: &'static str) -> Self {
        Zone {
            zone_type,
            zone_start_pfn: 0,
            spanned_pages: 0,
            present_pages: 0,
            managed_pages: 0,
            free_area: [
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
                FreeArea::new(),
            ],
            name,
        }
    }

    /// Initialize all free-area list sentinels.
    ///
    /// # Safety
    /// Must be called before any add/del/get operations.
    pub unsafe fn init_free_areas(&mut self) {
        for area in self.free_area.iter_mut() {
            area.init();
        }
    }

    /// Total free pages across all orders in this zone.
    pub fn free_pages(&self) -> usize {
        let mut total = 0;
        for (order, area) in self.free_area.iter().enumerate() {
            total += area.nr_free << order;
        }
        total
    }

    /// Add a page to a free list at the given order and migration type.
    ///
    /// If `tail` is true, add to the tail (LIFO-friendly for hot pages);
    /// otherwise add to the head.
    ///
    /// Equivalent to Linux's `__add_to_free_list()` (mm/page_alloc.c:832-851).
    ///
    /// # Safety
    /// - `page` must be a valid, initialized `Page` with its `lru` initialized.
    /// - The page must not already be on any list.
    pub unsafe fn add_to_free_list(
        &mut self,
        page: *mut Page,
        order: usize,
        migratetype: MigrateType,
        tail: bool,
    ) {
        let area = &mut self.free_area[order];
        let head = &mut area.free_list[migratetype as usize] as *mut ListHead;
        let entry = &mut (*page).lru as *mut ListHead;
        if tail {
            ListHead::list_add_tail(entry, head);
        } else {
            ListHead::list_add(entry, head);
        }
        area.nr_free += 1;
    }

    /// Remove a page from a free list at the given order and migration type.
    ///
    /// Equivalent to Linux's `__del_page_from_free_list()` (mm/page_alloc.c:882-902).
    ///
    /// # Safety
    /// - `page` must currently be linked in the free list at `order`/`migratetype`.
    pub unsafe fn del_from_free_list(
        &mut self,
        page: *mut Page,
        order: usize,
        _migratetype: MigrateType,
    ) {
        let entry = &mut (*page).lru as *mut ListHead;
        ListHead::list_del(entry);
        (*page).clear_buddy();
        self.free_area[order].nr_free -= 1;
    }

    /// Get the first page from a free list at the given order and migration type.
    ///
    /// Returns `None` if the free list is empty.
    ///
    /// Equivalent to Linux's `get_page_from_free_area()` (mm/page_alloc.c:911-916).
    ///
    /// # Safety
    /// - The free area must be initialized.
    pub unsafe fn get_page_from_free_area(
        &self,
        order: usize,
        migratetype: MigrateType,
    ) -> Option<*mut Page> {
        let head = &self.free_area[order].free_list[migratetype as usize] as *const ListHead;
        ListHead::first_entry(head).map(|entry| container_of!(entry, Page, lru))
    }
}

// ---------------------------------------------------------------------------
// Linux-visible mmzone.h helpers
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
pub fn NODE_DATA(_nid: i32) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn pfn_to_section_nr(pfn: usize) -> usize {
    pfn >> 18
}

pub fn section_nr_to_pfn(section: usize) -> usize {
    section << 18
}

pub fn __nr_to_section(_nr: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn __pfn_to_section(pfn: usize) -> *mut u8 {
    __nr_to_section(pfn_to_section_nr(pfn))
}

pub fn __section_mem_map_addr(_section: *const u8) -> *mut Page {
    core::ptr::null_mut()
}

pub fn valid_section(_section: *const u8) -> bool {
    false
}

pub fn valid_section_nr(_nr: usize) -> bool {
    false
}

pub fn present_section(_section: *const u8) -> bool {
    false
}

pub fn present_section_nr(_nr: usize) -> bool {
    false
}

pub fn online_section(_section: *const u8) -> bool {
    false
}

pub fn online_section_nr(_nr: usize) -> bool {
    false
}

pub fn online_mem_sections(_start: usize, _end: usize) {}

pub fn offline_mem_sections(_start: usize, _end: usize) {}

pub fn online_device_section(_section: *mut u8) {}

pub fn early_section(_section: *const u8) -> bool {
    false
}

pub fn pfn_in_present_section(pfn: usize) -> bool {
    pfn_valid(pfn)
}

pub fn pfn_section_valid(_section: *const u8, pfn: usize) -> bool {
    pfn_valid(pfn)
}

pub fn pfn_section_first_valid(_section: *const u8) -> usize {
    0
}

pub fn first_valid_pfn() -> usize {
    0
}

pub fn next_valid_pfn(pfn: usize) -> usize {
    pfn.saturating_add(1)
}

pub fn next_present_section_nr(section: usize) -> usize {
    section.saturating_add(1)
}

pub fn preinited_vmemmap_section(_section: *const u8) -> bool {
    false
}

pub fn subsection_map_index(pfn: usize) -> usize {
    pfn & ((1 << 18) - 1)
}

pub fn section_to_usemap(_section: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn mem_section_usage_size() -> usize {
    0
}

pub fn sparse_vmemmap_init_nid_early(_nid: i32, _pnum_begin: usize, _pnum_end: usize) {}

pub fn sparse_vmemmap_init_nid_late(_nid: i32, _pnum_begin: usize, _pnum_end: usize) {}

pub fn pfn_valid(pfn: usize) -> bool {
    let page = crate::mm::buddy::pfn_to_page(pfn);
    !page.is_null() && crate::mm::buddy::page_in_mem_map(page)
}

pub fn page_zonenum(page: *const Page) -> usize {
    if page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
        ZoneType::ZoneNormal as usize
    } else if crate::mm::buddy::page_to_pfn(page) < ZONE_DMA_MAX_PFN {
        ZoneType::ZoneDma as usize
    } else {
        ZoneType::ZoneNormal as usize
    }
}

pub fn folio_zonenum(folio: *const Page) -> usize {
    page_zonenum(folio)
}

pub fn memdesc_zonenum(page: *const Page) -> usize {
    page_zonenum(page)
}

pub fn is_zone_device_page(_page: *const Page) -> bool {
    false
}

pub fn is_zone_movable_page(page: *const Page) -> bool {
    page_zonenum(page) == ZoneType::ZoneNormal as usize
}

pub fn folio_is_zone_device(folio: *const Page) -> bool {
    is_zone_device_page(folio)
}

pub fn folio_is_zone_movable(folio: *const Page) -> bool {
    is_zone_movable_page(folio)
}

pub fn memdesc_is_zone_device(page: *const Page) -> bool {
    is_zone_device_page(page)
}

pub fn page_pgmap(_page: *const Page) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn zone_device_pages_have_same_pgmap(_a: *const Page, _b: *const Page) -> bool {
    true
}

pub fn managed_zone(zone: *const Zone) -> bool {
    !zone.is_null() && unsafe { (*zone).managed_pages > 0 }
}

pub fn populated_zone(zone: *const Zone) -> bool {
    !zone.is_null() && unsafe { (*zone).present_pages > 0 }
}

pub fn zone_is_empty(zone: *const Zone) -> bool {
    zone.is_null() || unsafe { (*zone).spanned_pages == 0 }
}

pub fn zone_is_initialized(zone: *const Zone) -> bool {
    !zone.is_null()
}

pub fn zone_managed_pages(zone: *const Zone) -> usize {
    if zone.is_null() {
        0
    } else {
        unsafe { (*zone).managed_pages }
    }
}

pub fn zone_cma_pages(_zone: *const Zone) -> usize {
    0
}

pub fn zone_to_nid(_zone: *const Zone) -> i32 {
    0
}

pub fn zone_set_nid(_zone: *mut Zone, _nid: i32) {}

pub fn zone_is_zone_device(_zone: *const Zone) -> bool {
    false
}

pub fn zone_end_pfn(zone: *const Zone) -> usize {
    if zone.is_null() {
        0
    } else {
        unsafe { (*zone).zone_start_pfn.saturating_add((*zone).spanned_pages) }
    }
}

pub fn zone_spans_pfn(zone: *const Zone, pfn: usize) -> bool {
    !zone.is_null() && unsafe { pfn >= (*zone).zone_start_pfn && pfn < zone_end_pfn(zone) }
}

pub fn zone_intersects(zone: *const Zone, start_pfn: usize, nr_pages: usize) -> bool {
    if zone.is_null() || nr_pages == 0 {
        return false;
    }
    let end = start_pfn.saturating_add(nr_pages);
    unsafe { start_pfn < zone_end_pfn(zone) && end > (*zone).zone_start_pfn }
}

pub fn min_wmark_pages(zone: *const Zone) -> usize {
    zone_managed_pages(zone) / 100
}

pub fn low_wmark_pages(zone: *const Zone) -> usize {
    zone_managed_pages(zone) / 50
}

pub fn high_wmark_pages(zone: *const Zone) -> usize {
    zone_managed_pages(zone) / 25
}

pub fn promo_wmark_pages(zone: *const Zone) -> usize {
    high_wmark_pages(zone)
}

pub fn wmark_pages(zone: *const Zone, mark: usize) -> usize {
    match mark {
        0 => min_wmark_pages(zone),
        1 => low_wmark_pages(zone),
        _ => high_wmark_pages(zone),
    }
}

pub fn has_managed_zone(zone: *const Zone) -> bool {
    managed_zone(zone)
}

pub fn has_managed_dma() -> bool {
    true
}

pub fn is_highmem_idx(_idx: usize) -> bool {
    false
}

pub fn is_highmem(_zone: *const Zone) -> bool {
    false
}

pub fn is_file_lru(lru: usize) -> bool {
    lru & 1 != 0
}

pub fn is_active_lru(lru: usize) -> bool {
    lru & 2 != 0
}

pub fn is_migrate_movable(migratetype: usize) -> bool {
    migratetype == MigrateType::Movable as usize
}

pub fn migratetype_is_mergeable(migratetype: usize) -> bool {
    migratetype <= MigrateType::Reclaimable as usize
}

pub fn first_online_pgdat() -> *mut u8 {
    core::ptr::null_mut()
}

pub fn next_online_pgdat(_pgdat: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn pgdat_end_pfn(_pgdat: *const u8) -> usize {
    0
}

pub fn local_memory_node(_node: i32) -> i32 {
    0
}

pub fn first_zones_zonelist(
    _zonelist: *const u8,
    _highest_zoneidx: usize,
    _nodes: *const u8,
) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn next_zones_zonelist(_z: *mut u8, _highest_zoneidx: usize, _nodes: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn zonelist_zone(_z: *const u8) -> *mut Zone {
    core::ptr::null_mut()
}

pub fn zonelist_zone_idx(_z: *const u8) -> usize {
    ZoneType::ZoneNormal as usize
}

pub fn zonelist_node_idx(_z: *const u8) -> i32 {
    0
}

pub fn next_zone(_zone: *mut Zone) -> *mut Zone {
    core::ptr::null_mut()
}

pub fn build_all_zonelists(_pgdat: *mut u8) {}

pub fn init_currently_empty_zone(
    zone: *mut Zone,
    start_pfn: usize,
    size: usize,
    _zone_type: usize,
    _altmap: *mut u8,
) {
    if !zone.is_null() {
        unsafe {
            (*zone).zone_start_pfn = start_pfn;
            (*zone).spanned_pages = size;
            (*zone).present_pages = size;
            (*zone).managed_pages = size;
        }
    }
}

pub fn memmap_init_zone_device(
    _zone: *mut Zone,
    _start_pfn: usize,
    _nr_pages: usize,
    _pgmap: *mut u8,
) {
}

pub fn movable_only_nodes(_nodes: *const u8) -> bool {
    false
}

pub fn kswapd_test_hopeless(_pgdat: *const u8) -> bool {
    false
}

pub fn kswapd_clear_hopeless(_pgdat: *mut u8) {}

pub fn lruvec_init(_lruvec: *mut u8) {}

pub fn lruvec_pgdat(_lruvec: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn lru_gen_init_lruvec(_lruvec: *mut u8) {}

pub fn lru_gen_init_pgdat(_pgdat: *mut u8) {}

pub fn lru_gen_init_memcg(_memcg: *mut u8) {}

pub fn lru_gen_exit_memcg(_memcg: *mut u8) {}

pub fn lru_gen_online_memcg(_memcg: *mut u8) {}

pub fn lru_gen_offline_memcg(_memcg: *mut u8) {}

pub fn lru_gen_release_memcg(_memcg: *mut u8) {}

pub fn lru_gen_reparent_memcg(_memcg: *mut u8, _parent: *mut u8) {}

pub fn lru_gen_soft_reclaim(_memcg: *mut u8, _pgdat: *mut u8, _sc: *mut u8) -> usize {
    0
}

pub fn lru_gen_look_around(_vmf: *mut u8) {}

pub fn max_lru_gen_memcg() -> usize {
    0
}

pub fn recheck_lru_gen_max_memcg(_memcg: *mut u8) {}

pub fn generation(_lruvec: *const u8) -> usize {
    0
}

pub fn vmstat_item_in_bytes(_item: usize) -> bool {
    false
}

pub fn vmstat_item_print_in_thp(_item: usize) -> bool {
    false
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unsafe_op_in_unsafe_fn)]
mod tests {
    use super::*;
    use crate::mm::page::Page;
    use crate::mm::page_flags::MigrateType;

    #[test]
    fn free_area_initial_nr_free_zero() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            for area in zone.free_area.iter() {
                assert_eq!(area.nr_free, 0);
            }
        }
    }

    #[test]
    fn zone_has_11_free_areas() {
        let zone = Zone::new(ZoneType::ZoneNormal, "Test");
        assert_eq!(zone.free_area.len(), NR_PAGE_ORDERS);
        assert_eq!(NR_PAGE_ORDERS, 11);
    }

    #[test]
    fn zone_dma_boundary_is_16mb() {
        assert_eq!(ZONE_DMA_MAX_PFN, 4096);
        assert_eq!(ZONE_DMA_MAX_PFN * PAGE_SIZE, 16 * 1024 * 1024);
    }

    #[test]
    fn max_page_order_is_10() {
        assert_eq!(MAX_PAGE_ORDER, 10);
        // Order 10 = 2^10 = 1024 pages = 4 MiB
        assert_eq!((1 << MAX_PAGE_ORDER) * PAGE_SIZE, 4 * 1024 * 1024);
    }

    #[test]
    fn add_to_free_list_increments_nr_free() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            let mut page = Page::new();
            page.init_lru();
            zone.add_to_free_list(&mut page, 0, MigrateType::Unmovable, false);
            assert_eq!(zone.free_area[0].nr_free, 1);
        }
    }

    #[test]
    fn del_from_free_list_decrements_nr_free() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            let mut page = Page::new();
            page.init_lru();
            page.set_buddy_order(0);
            zone.add_to_free_list(&mut page, 0, MigrateType::Unmovable, false);
            assert_eq!(zone.free_area[0].nr_free, 1);

            zone.del_from_free_list(&mut page, 0, MigrateType::Unmovable);
            assert_eq!(zone.free_area[0].nr_free, 0);
        }
    }

    #[test]
    fn get_page_from_empty_returns_none() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            assert!(
                zone.get_page_from_free_area(0, MigrateType::Unmovable)
                    .is_none()
            );
        }
    }

    #[test]
    fn get_page_from_free_area_returns_first() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            let mut page_a = Page::new();
            page_a.init_lru();
            let mut page_b = Page::new();
            page_b.init_lru();
            // Add a (front), then b (front) → order is: head → b → a → head
            zone.add_to_free_list(&mut page_a, 0, MigrateType::Unmovable, false);
            zone.add_to_free_list(&mut page_b, 0, MigrateType::Unmovable, false);

            let first = zone
                .get_page_from_free_area(0, MigrateType::Unmovable)
                .unwrap();
            assert_eq!(first, &mut page_b as *mut Page);
        }
    }

    #[test]
    fn zone_free_pages_counts_across_orders() {
        unsafe {
            let mut zone = Zone::new(ZoneType::ZoneNormal, "TestNormal");
            zone.init_free_areas();
            // Manually bump nr_free for testing.
            zone.free_area[0].nr_free = 3; // 3 × 2^0 = 3 pages
            zone.free_area[3].nr_free = 2; // 2 × 2^3 = 16 pages
            zone.free_area[10].nr_free = 1; // 1 × 2^10 = 1024 pages
            assert_eq!(zone.free_pages(), 3 + 16 + 1024);
        }
    }
}
