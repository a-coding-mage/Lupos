//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Per-page metadata — the lupos equivalent of Linux's `struct page`.
///
/// Every physical page frame tracked by the buddy allocator has a
/// corresponding `Page` struct in the global `mem_map` array.  The struct
/// is 64 bytes (one cache line) and `#[repr(C)]` for ABI compatibility.
///
/// ## Field layout (matches Linux mm_types.h:79-222)
///
/// | Offset | Size | Field       | Purpose                                      |
/// |--------|------|-------------|----------------------------------------------|
/// |  0     |  8   | flags       | Atomic page flags (PG_locked, PG_dirty, etc.) |
/// |  8     | 16   | lru         | Intrusive list node (buddy_list / LRU)        |
/// | 24     |  8   | mapping     | address_space pointer (M7: unused)             |
/// | 32     |  8   | index       | Offset within mapping (M7: unused)             |
/// | 40     |  8   | private     | Buddy order when PageBuddy; fs-private data    |
/// | 48     |  4   | page_type   | PGTY_buddy, PGTY_slab, etc.                   |
/// | 52     |  4   | _mapcount   | Page table reference count                    |
/// | 56     |  4   | _refcount   | Usage reference count                         |
/// | 60     |  4   | _pad        | Alignment padding to 64 bytes                 |
///
/// Ref: Linux include/linux/mm_types.h — struct page
///      Linux include/linux/page-flags.h — PageBuddy, set_buddy_order
///      Linux mm/internal.h — buddy_order(), set_buddy_order()
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};

use crate::mm::list::ListHead;
use crate::mm::page_flags::{
    PAGE_TYPE_NONE, PG_RESERVED, PGTY_BUDDY, decode_page_type, encode_page_type,
};

/// Per-page frame descriptor, equivalent to Linux's `struct page`.
///
/// Size: exactly 64 bytes (compile-time asserted below).
#[repr(C)]
pub struct Page {
    /// Atomic page flags (PG_locked, PG_dirty, PG_reserved, etc.).
    /// Ref: Linux page-flags.h — memdesc_flags_t
    pub flags: AtomicU64,

    /// Intrusive list node used for:
    /// - Buddy free list (`buddy_list`) when the page is free
    /// - LRU list when the page is in the page cache
    /// Ref: Linux mm_types.h:96-101
    pub lru: ListHead,

    /// Pointer to the owning `address_space` (page cache mapping).
    /// Unused in M7 — set to 0.
    /// Ref: Linux mm_types.h:103
    pub mapping: usize,

    /// Offset within the address_space mapping.
    /// Unused in M7 — set to 0.
    /// Ref: Linux mm_types.h:105
    pub index: usize,

    /// Multi-purpose private field:
    /// - When `PageBuddy()`: stores the allocation order (0..MAX_PAGE_ORDER)
    /// - When `PagePrivate()`: filesystem-private data
    /// Ref: Linux mm_types.h:115, mm/internal.h:685 (buddy_order)
    pub private: usize,

    /// Page type tag (top byte) for non-mapcount pages.
    /// `PGTY_BUDDY` (0xf0) when free in the buddy system.
    /// `PAGE_TYPE_NONE` (0xFFFF_FFFF) when untyped.
    /// Ref: Linux page-flags.h:925 — enum pagetype
    pub page_type: AtomicU32,

    /// Page table reference count (how many PTEs map this page).
    /// -1 means unmapped.
    /// Ref: Linux mm_types.h:180
    pub _mapcount: AtomicI32,

    /// Usage reference count.  0 = free, >0 = in use.
    /// `get_page()` increments, `put_page()` decrements.
    /// Ref: Linux mm_types.h:184
    pub _refcount: AtomicI32,

    /// Padding to reach exactly 64 bytes (cache-line aligned).
    pub _pad: u32,
}

// Compile-time assertion: struct Page must be exactly 64 bytes.
const _: () = assert!(core::mem::size_of::<Page>() == 64);

impl Page {
    /// Create a new Page with all fields zeroed / default.
    ///
    /// The page starts as:
    /// - flags = 0 (no flags set)
    /// - page_type = PAGE_TYPE_NONE (untyped)
    /// - _refcount = 0 (free)
    /// - _mapcount = -1 (unmapped)
    /// - lru = uninitialized (caller must init before linking)
    pub const fn new() -> Self {
        Page {
            flags: AtomicU64::new(0),
            lru: ListHead::uninit(),
            mapping: 0,
            index: 0,
            private: 0,
            page_type: AtomicU32::new(PAGE_TYPE_NONE),
            _mapcount: AtomicI32::new(-1),
            _refcount: AtomicI32::new(0),
            _pad: 0,
        }
    }

    /// Initialize the page's lru list node to point to itself.
    ///
    /// # Safety
    /// Must be called before the page's lru is linked into any list.
    #[inline]
    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn init_lru(&mut self) {
        ListHead::init(&mut self.lru);
    }

    // -----------------------------------------------------------------------
    // Buddy helpers — match Linux mm/internal.h and page-flags.h
    // -----------------------------------------------------------------------

    /// Check if this page is free in the buddy system.
    ///
    /// Equivalent to Linux's `PageBuddy(page)`.
    /// Returns true when `page_type` top byte == `PGTY_BUDDY` (0xf0).
    ///
    /// Ref: Linux page-flags.h — PAGE_TYPE_OPS(Buddy, buddy, buddy)
    #[inline]
    pub fn is_buddy(&self) -> bool {
        decode_page_type(self.page_type.load(Ordering::Relaxed)) == PGTY_BUDDY
    }

    /// Get the buddy order stored in `private`.
    ///
    /// Only valid when `is_buddy()` returns true.
    ///
    /// Equivalent to Linux's `buddy_order(page)` (mm/internal.h:685).
    #[inline]
    pub fn buddy_order(&self) -> usize {
        debug_assert!(self.is_buddy(), "buddy_order called on non-buddy page");
        self.private
    }

    /// Mark this page as a buddy-system free page at the given order.
    ///
    /// Sets `private = order` and `page_type = PGTY_BUDDY`.
    ///
    /// Equivalent to Linux's `set_buddy_order()` (mm/page_alloc.c:752).
    ///
    /// Ref: mm/page_alloc.c:752-756
    #[inline]
    pub fn set_buddy_order(&mut self, order: usize) {
        self.private = order;
        self.page_type
            .store(encode_page_type(PGTY_BUDDY), Ordering::Relaxed);
    }

    /// Clear buddy-system metadata from this page.
    ///
    /// Resets `page_type` to `PAGE_TYPE_NONE` and `private` to 0.
    ///
    /// Equivalent to Linux's `__ClearPageBuddy(page)` + `set_page_private(page, 0)`.
    ///
    /// Ref: mm/page_alloc.c:886-901 (__del_page_from_free_list)
    #[inline]
    pub fn clear_buddy(&mut self) {
        self.page_type.store(PAGE_TYPE_NONE, Ordering::Relaxed);
        self.private = 0;
    }

    // -----------------------------------------------------------------------
    // Flag helpers
    // -----------------------------------------------------------------------

    /// Set a flag bit in this page's flags.
    #[inline]
    pub fn set_flag(&self, flag: u64) {
        self.flags.fetch_or(flag, Ordering::Relaxed);
    }

    /// Clear a flag bit from this page's flags.
    #[inline]
    pub fn clear_flag(&self, flag: u64) {
        self.flags.fetch_and(!flag, Ordering::Relaxed);
    }

    /// Test whether a flag bit is set.
    #[inline]
    pub fn test_flag(&self, flag: u64) -> bool {
        self.flags.load(Ordering::Relaxed) & flag != 0
    }

    /// Check if this page is reserved (PG_reserved set).
    #[inline]
    pub fn is_reserved(&self) -> bool {
        self.test_flag(PG_RESERVED)
    }

    /// Mark this page as reserved.
    #[inline]
    pub fn set_reserved(&self) {
        self.set_flag(PG_RESERVED);
    }

    // -----------------------------------------------------------------------
    // Reference counting — matches Linux mm/internal.h, include/linux/mm.h
    // -----------------------------------------------------------------------

    /// Increment the usage reference count.
    ///
    /// Equivalent to Linux's `get_page()`.
    #[inline]
    pub fn get_page(&self) {
        self._refcount.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the usage reference count and return the new value.
    ///
    /// Equivalent to Linux's `put_page()` (simplified — no free-on-zero).
    #[inline]
    pub fn put_page(&self) -> i32 {
        self._refcount.fetch_sub(1, Ordering::Relaxed) - 1
    }

    /// Get the current reference count.
    #[inline]
    pub fn refcount(&self) -> i32 {
        self._refcount.load(Ordering::Relaxed)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    /// struct Page must be exactly 64 bytes (one cache line).
    #[test]
    fn page_size_is_64_bytes() {
        assert_eq!(mem::size_of::<Page>(), 64);
    }

    /// The `flags` field must be at offset 0 for Linux ABI compatibility.
    #[test]
    fn page_flags_at_offset_zero() {
        assert_eq!(mem::offset_of!(Page, flags), 0);
    }

    /// A newly created page has zero refcount.
    #[test]
    fn page_default_refcount_zero() {
        let page = Page::new();
        assert_eq!(page.refcount(), 0);
    }

    /// A newly created page has mapcount -1 (unmapped).
    #[test]
    fn page_default_mapcount_minus_one() {
        let page = Page::new();
        assert_eq!(page._mapcount.load(Ordering::Relaxed), -1);
    }

    /// set_buddy_order stores the order and marks as PageBuddy.
    #[test]
    fn set_buddy_order_stores_order_and_type() {
        let mut page = Page::new();
        page.set_buddy_order(5);
        assert!(page.is_buddy());
        assert_eq!(page.buddy_order(), 5);
        assert_eq!(page.private, 5);
        assert_eq!(
            page.page_type.load(Ordering::Relaxed),
            encode_page_type(PGTY_BUDDY)
        );
    }

    /// clear_buddy resets page_type and private.
    #[test]
    fn clear_buddy_resets() {
        let mut page = Page::new();
        page.set_buddy_order(7);
        assert!(page.is_buddy());
        page.clear_buddy();
        assert!(!page.is_buddy());
        assert_eq!(page.private, 0);
        assert_eq!(page.page_type.load(Ordering::Relaxed), PAGE_TYPE_NONE);
    }

    /// get_page/put_page increment and decrement refcount.
    #[test]
    fn refcount_get_put() {
        let page = Page::new();
        assert_eq!(page.refcount(), 0);
        page.get_page();
        assert_eq!(page.refcount(), 1);
        page.get_page();
        assert_eq!(page.refcount(), 2);
        let after = page.put_page();
        assert_eq!(after, 1);
        assert_eq!(page.refcount(), 1);
    }

    /// Page flag set/clear/test operations work correctly.
    #[test]
    fn page_flag_operations() {
        let page = Page::new();
        assert!(!page.is_reserved());
        page.set_reserved();
        assert!(page.is_reserved());
        page.clear_flag(PG_RESERVED);
        assert!(!page.is_reserved());
    }

    /// Verify key field offsets for ABI compatibility.
    #[test]
    fn page_field_offsets() {
        assert_eq!(mem::offset_of!(Page, flags), 0);
        assert_eq!(mem::offset_of!(Page, lru), 8);
        assert_eq!(mem::offset_of!(Page, mapping), 24);
        assert_eq!(mem::offset_of!(Page, index), 32);
        assert_eq!(mem::offset_of!(Page, private), 40);
        assert_eq!(mem::offset_of!(Page, page_type), 48);
        assert_eq!(mem::offset_of!(Page, _mapcount), 52);
        assert_eq!(mem::offset_of!(Page, _refcount), 56);
        assert_eq!(mem::offset_of!(Page, _pad), 60);
    }
}
