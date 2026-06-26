//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Page cache anchor — `struct address_space` and `address_space_operations`.
///
/// Every file (inode) that participates in the page cache owns one
/// `AddressSpace`.  The XArray `i_pages` maps file-offset (in pages) to
/// cached `Page` structs.  Filesystems register their I/O callbacks through
/// the `AddressSpaceOperations` vtable; generic code calls through that vtable
/// without knowing which filesystem is underneath.
///
/// This module also houses the **page locking** primitives (`lock_page`,
/// `unlock_page`, `try_lock_page`, `wait_on_page_locked`) because they operate
/// directly on `Page::flags` bits (`PG_LOCKED`, `PG_WAITERS`) that are defined
/// alongside the other page-cache semantics here.
///
/// Ref: Linux `include/linux/fs.h:470-490` — `struct address_space`
///      Linux `include/linux/fs.h:403-444` — `struct address_space_operations`
///      Linux `include/linux/pagemap.h`    — folio/page locking helpers
extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use super::page::Page;
use super::page_flags::{PG_LOCKED, PG_UPTODATE, PG_WAITERS, PG_WRITEBACK};
use super::xarray::XArray;

// Forward declarations for vtable callback argument types.
// Real types land in M36 (writeback_control), M38 (file/inode), M43 (block).
// We use *mut u8 placeholders so the vtable has the correct number of
// function-pointer slots from day one — pointer width is identical.

/// Opaque placeholder for `struct readahead_control *`.
/// The concrete type lives in `readahead.rs`; the vtable pointer slot uses
/// this alias so both modules can refer to it without a circular import.
pub use super::readahead::ReadaheadControl;
/// Opaque placeholder for `struct writeback_control *` until M16.
pub use super::writeback::WritebackControl;

// ---------------------------------------------------------------------------
// AS_* mapping flags  (stored in AddressSpace::flags)
//
// Ref: Linux include/linux/fs.h:458-468
// ---------------------------------------------------------------------------

/// I/O error has been reported on this mapping.
pub const AS_EIO: u64 = 1 << 0;
/// No space left on device for this mapping.
pub const AS_ENOSPC: u64 = 1 << 1;
/// Mapping cannot be evicted.
pub const AS_UNEVICTABLE: u64 = 1 << 2;
/// Mapping is being torn down.
pub const AS_EXITING: u64 = 1 << 3;
/// Suppress writeback tag tracking.
pub const AS_NO_WRITEBACK_TAGS: u64 = 1 << 4;
/// Require stable page writes.
pub const AS_STABLE_WRITES: u64 = 1 << 5;
/// Anonymous MAP_SHARED backing store owned by the mm layer.
pub const AS_SHARED_ANON: u64 = 1 << 6;

// ---------------------------------------------------------------------------
// address_space_operations vtable
// ---------------------------------------------------------------------------

/// Filesystem-provided callbacks for the page cache.
///
/// The function-pointer fields mirror Linux `struct address_space_operations`
/// in field order, so a future `.ko` loader can overlay this struct directly.
///
/// All callbacks are `Option<unsafe extern "C" fn(...)>` — `None` means the
/// generic fallback applies.
///
/// Ref: Linux `include/linux/fs.h:403-444`
#[repr(C)]
pub struct AddressSpaceOperations {
    /// Fill a page from the backing store.
    /// Ref: `->read_folio()` — Linux fs.h:405
    pub read_folio: Option<unsafe extern "C" fn(*mut AddressSpace, *mut Page) -> i32>,

    /// Initiate readahead for a batch of pages.
    /// Ref: `->readahead()` — Linux fs.h:406
    pub readahead: Option<unsafe extern "C" fn(*mut ReadaheadControl)>,

    /// Flush dirty pages to the backing store.
    /// Ref: `->writepages()` — Linux fs.h:407
    pub writepages: Option<unsafe extern "C" fn(*mut AddressSpace, *mut WritebackControl) -> i32>,

    /// Mark a page dirty in the mapping.
    /// Ref: `->dirty_folio()` — Linux fs.h:408
    pub dirty_folio: Option<unsafe extern "C" fn(*mut AddressSpace, *mut Page) -> bool>,

    /// Prepare a page for buffered write.
    /// Ref: `->write_begin()` — Linux fs.h:409-411
    pub write_begin: Option<
        unsafe extern "C" fn(*mut AddressSpace, i64, u32, *mut *mut Page, *mut *mut u8) -> i32,
    >,

    /// Commit a buffered write.
    /// Ref: `->write_end()` — Linux fs.h:412-414
    pub write_end:
        Option<unsafe extern "C" fn(*mut AddressSpace, i64, u32, u32, *mut Page, *mut u8) -> i32>,

    /// Invalidate part of a cached page.
    /// Ref: `->invalidate_folio()` — Linux fs.h:420
    pub invalidate_folio: Option<unsafe extern "C" fn(*mut Page, usize, usize)>,

    /// Release fs-private data from a page.
    /// Ref: `->release_folio()` — Linux fs.h:421
    pub release_folio: Option<unsafe extern "C" fn(*mut Page, u32) -> bool>,

    /// Free a page (final cleanup).
    /// Ref: `->free_folio()` — Linux fs.h:422
    pub free_folio: Option<unsafe extern "C" fn(*mut Page)>,

    /// Test if a page range is partially uptodate.
    /// Ref: `->is_partially_uptodate()` — Linux fs.h:427
    pub is_partially_uptodate: Option<unsafe extern "C" fn(*mut Page, usize, usize) -> bool>,

    /// Report I/O error on a page.
    /// Ref: `->error_remove_folio()` — Linux fs.h:431
    pub error_remove_folio: Option<unsafe extern "C" fn(*mut AddressSpace, *mut Page) -> i32>,
}

// ---------------------------------------------------------------------------
// AddressSpace
// ---------------------------------------------------------------------------

/// Per-inode page cache anchor.
///
/// Field order and sizes match Linux `struct address_space` (fs.h:470-490)
/// for the fields we implement.  Placeholder fields (`host`) will be filled
/// with real types when `struct inode` lands in M38.
///
/// Ref: Linux `include/linux/fs.h:470-490`
#[repr(C)]
pub struct AddressSpace {
    /// Owning inode (placeholder `*mut u8` until M38 defines `Inode`).
    /// Ref: Linux `struct inode *host` — fs.h:471
    pub host: *mut u8,

    /// XArray-indexed page cache (`i_pages`).
    /// Maps file-offset (in pages) to `*mut Page`.
    /// Ref: Linux `struct xarray i_pages` — fs.h:472
    pub i_pages: XArray,

    /// Allocation flags for page cache allocations.
    /// Ref: Linux `gfp_t gfp_mask` — fs.h:474
    pub gfp_mask: u32,

    /// Count of `VM_SHARED | VM_MAYWRITE` mappings.
    /// Ref: Linux `atomic_t i_mmap_writable` — fs.h:475
    pub i_mmap_writable: core::sync::atomic::AtomicI32,

    /// VMAs that map this address space.
    /// Ref: Linux `struct address_space::i_mmap`
    pub i_mmap: spin::Mutex<Vec<usize>>,

    /// Number of pages currently in the cache.
    /// Ref: Linux `unsigned long nrpages` — fs.h:480
    pub nrpages: core::sync::atomic::AtomicUsize,

    /// Index from which the next writeback sweep starts.
    /// Ref: Linux `pgoff_t writeback_index` — fs.h:481
    pub writeback_index: core::sync::atomic::AtomicU64,

    /// Filesystem-provided vtable.
    /// Ref: Linux `const struct address_space_operations *a_ops` — fs.h:482
    pub a_ops: *const AddressSpaceOperations,

    /// Error and state flags (`AS_*` constants).
    /// Ref: Linux `unsigned long flags` — fs.h:483
    pub flags: core::sync::atomic::AtomicU64,

    /// Writeback error sequence number.
    /// Ref: Linux `errseq_t wb_err` — fs.h:484
    pub wb_err: core::sync::atomic::AtomicU32,
}

// Safety: AddressSpace is shared across CPUs; all mutable state is protected
// by the XArray spinlock, atomic fields, or external locking (mmap_lock, etc.)
unsafe impl Send for AddressSpace {}
unsafe impl Sync for AddressSpace {}

impl AddressSpace {
    /// Allocate a new, empty `AddressSpace`.
    ///
    /// Ref: Linux `address_space_init_once()` (via slab ctor)
    pub fn new() -> Self {
        AddressSpace {
            host: core::ptr::null_mut(),
            i_pages: XArray::new(),
            gfp_mask: super::page_flags::GFP_KERNEL,
            i_mmap_writable: core::sync::atomic::AtomicI32::new(0),
            i_mmap: spin::Mutex::new(Vec::new()),
            nrpages: core::sync::atomic::AtomicUsize::new(0),
            writeback_index: core::sync::atomic::AtomicU64::new(0),
            a_ops: core::ptr::null(),
            flags: core::sync::atomic::AtomicU64::new(0),
            wb_err: core::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Set the `AddressSpaceOperations` vtable.
    pub fn set_ops(&mut self, ops: *const AddressSpaceOperations) {
        self.a_ops = ops;
    }
}

pub fn register_mapping_vma(
    mapping: *mut AddressSpace,
    vma: *mut crate::mm::mm_types::VmAreaStruct,
) {
    if mapping.is_null() || vma.is_null() {
        return;
    }

    let mut list = unsafe { &*mapping }.i_mmap.lock();
    let value = vma as usize;
    if !list.iter().any(|&entry| entry == value) {
        list.push(value);
    }
}

pub fn unregister_mapping_vma(
    mapping: *mut AddressSpace,
    vma: *mut crate::mm::mm_types::VmAreaStruct,
) {
    if mapping.is_null() || vma.is_null() {
        return;
    }

    unsafe { &*mapping }
        .i_mmap
        .lock()
        .retain(|&entry| entry != vma as usize);
}

pub fn mapping_vmas(mapping: *const AddressSpace) -> Vec<*mut crate::mm::mm_types::VmAreaStruct> {
    if mapping.is_null() {
        return Vec::new();
    }

    unsafe { &*mapping }
        .i_mmap
        .lock()
        .iter()
        .map(|&entry| entry as *mut crate::mm::mm_types::VmAreaStruct)
        .collect()
}

// ---------------------------------------------------------------------------
// Page locking primitives
//
// These operate on the PG_LOCKED bit (bit 0) of Page::flags via atomic
// compare-exchange.  PG_WAITERS (bit 7) signals that other CPUs are spinning
// waiting for the lock — cleared on unlock.
//
// On bare-metal without a scheduler we spin; future milestones will replace
// the spin loop with a proper wait queue once process sleep is available.
//
// Ref: Linux `include/linux/pagemap.h` — folio_trylock, folio_lock, folio_unlock
//      Linux `mm/folio-compat.c`       — lock_page, unlock_page wrappers
// ---------------------------------------------------------------------------

/// Try to lock the page.
///
/// Atomically sets `PG_LOCKED` if it is clear.  Returns `true` if the lock
/// was acquired, `false` if the page was already locked.
///
/// Equivalent to Linux `folio_trylock()` / `trylock_page()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_trylock()`
#[inline]
pub unsafe fn try_lock_page(page: *mut Page) -> bool {
    let flags = &unsafe { &*page }.flags;
    // CAS: if current & PG_LOCKED == 0, set PG_LOCKED.
    let mut old = flags.load(Ordering::Relaxed);
    loop {
        if old & PG_LOCKED != 0 {
            return false;
        }
        match flags.compare_exchange_weak(
            old,
            old | PG_LOCKED,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => return true,
            Err(current) => old = current,
        }
    }
}

/// Lock the page, spinning until the lock is acquired.
///
/// Bare-metal implementation: busy-waits (no sleep).  Future milestones will
/// park the task in a wait queue instead.
///
/// Equivalent to Linux `lock_page()` / `folio_lock()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_lock()`
#[inline]
pub unsafe fn lock_page(page: *mut Page) {
    while !unsafe { try_lock_page(page) } {
        core::hint::spin_loop();
    }
}

/// Lock the page, returning 0 on success.
///
/// In Linux this can return `-EINTR` when a fatal signal is pending; in
/// Lupos M15 (no signal support) it always succeeds like `lock_page`.
///
/// Equivalent to Linux `folio_lock_killable()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_lock_killable()`
#[inline]
pub unsafe fn lock_page_killable(page: *mut Page) -> i32 {
    unsafe { lock_page(page) };
    0
}

/// Unlock the page.
///
/// Clears `PG_LOCKED` and `PG_WAITERS`.  A memory-release fence ensures that
/// all writes to page data are visible before a waiting locker proceeds.
///
/// Equivalent to Linux `unlock_page()` / `folio_unlock()`.
///
/// Ref: Linux `mm/folio-compat.c` — `unlock_page()`
#[inline]
pub unsafe fn unlock_page(page: *mut Page) {
    unsafe { &*page }
        .flags
        .fetch_and(!(PG_LOCKED | PG_WAITERS), Ordering::Release);
}

/// Spin until the page is no longer locked.
///
/// Callers that need to wait for a page to become uptodate should call this
/// after `lock_page` to avoid the "lock then wait" pattern.
///
/// Equivalent to Linux `wait_on_page_locked()` / `folio_wait_locked()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_wait_locked()`
#[inline]
pub unsafe fn wait_on_page_locked(page: *mut Page) {
    while unsafe { &*page }.flags.load(Ordering::Acquire) & PG_LOCKED != 0 {
        core::hint::spin_loop();
    }
}

/// Spin until the page is no longer under writeback.
///
/// Equivalent to Linux `wait_on_page_writeback()` / `folio_wait_writeback()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_wait_writeback()`
#[inline]
pub unsafe fn wait_on_page_writeback(page: *mut Page) {
    while unsafe { &*page }.flags.load(Ordering::Acquire) & PG_WRITEBACK != 0 {
        core::hint::spin_loop();
    }
}

/// Test whether a page's data is uptodate.
#[inline]
pub unsafe fn page_uptodate(page: *const Page) -> bool {
    unsafe { &*page }.flags.load(Ordering::Acquire) & PG_UPTODATE != 0
}

/// Mark a page's data as uptodate (read from backing store succeeded).
#[inline]
pub unsafe fn set_page_uptodate(page: *mut Page) {
    unsafe { &*page }
        .flags
        .fetch_or(PG_UPTODATE, Ordering::Release);
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::page::Page;
    use crate::mm::page_flags::GFP_KERNEL;

    // ── address_space_new_has_zero_nrpages ────────────────────────────────────

    #[test]
    fn address_space_new_has_zero_nrpages() {
        let mapping = AddressSpace::new();
        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 0);
        assert!(mapping.i_pages.is_empty());
        assert!(mapping.a_ops.is_null());
        assert_eq!(mapping.gfp_mask, GFP_KERNEL);
    }

    // ── page_try_lock_exclusive ───────────────────────────────────────────────

    #[test]
    fn page_try_lock_exclusive() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        // First acquire succeeds
        assert!(unsafe { try_lock_page(ptr) });
        // Second acquire fails — already locked
        assert!(!unsafe { try_lock_page(ptr) });

        unsafe { unlock_page(ptr) };
        // After unlock, can acquire again
        assert!(unsafe { try_lock_page(ptr) });

        unsafe { unlock_page(ptr) };
    }

    // ── page_lock_then_unlock_clears_flag ─────────────────────────────────────

    #[test]
    fn page_lock_then_unlock_clears_flag() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        unsafe { lock_page(ptr) };
        assert!(page.flags.load(Ordering::Relaxed) & PG_LOCKED != 0);
        page.flags.fetch_or(PG_WAITERS, Ordering::Relaxed);

        unsafe { unlock_page(ptr) };
        assert_eq!(page.flags.load(Ordering::Relaxed) & PG_LOCKED, 0);
        assert_eq!(page.flags.load(Ordering::Relaxed) & PG_WAITERS, 0);
    }

    // ── wait_on_page_locked_returns_when_unlocked ─────────────────────────────
    //
    // We can't spin two threads in a unit test easily, so we verify that
    // wait_on_page_locked on an already-unlocked page returns immediately.

    #[test]
    fn wait_on_page_locked_returns_when_unlocked() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        // Page starts unlocked — wait_on_page_locked should return immediately.
        unsafe { wait_on_page_locked(ptr) };
        // No hang means pass.

        // Lock then unlock, then wait again — should also be immediate.
        unsafe { lock_page(ptr) };
        unsafe { unlock_page(ptr) };
        unsafe { wait_on_page_locked(ptr) };
    }

    #[test]
    fn wait_on_page_writeback_returns_when_writeback_clear() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        unsafe { wait_on_page_writeback(ptr) };
        page.flags.fetch_or(PG_WRITEBACK, Ordering::Relaxed);
        page.flags.fetch_and(!PG_WRITEBACK, Ordering::Relaxed);
        unsafe { wait_on_page_writeback(ptr) };
    }

    // ── vtable_null_read_folio_is_safe_to_call_through_option ─────────────────

    #[test]
    fn vtable_null_read_folio_is_safe_to_call_through_option() {
        let ops = AddressSpaceOperations {
            read_folio: None,
            readahead: None,
            writepages: None,
            dirty_folio: None,
            write_begin: None,
            write_end: None,
            invalidate_folio: None,
            release_folio: None,
            free_folio: None,
            is_partially_uptodate: None,
            error_remove_folio: None,
        };
        // Calling through None option must not invoke anything.
        assert!(ops.read_folio.is_none());
        assert!(ops.readahead.is_none());
    }

    // ── set_page_uptodate_and_page_uptodate ───────────────────────────────────

    #[test]
    fn set_page_uptodate_and_check() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        assert!(!unsafe { page_uptodate(ptr) });
        unsafe { set_page_uptodate(ptr) };
        assert!(unsafe { page_uptodate(ptr) });
    }
}
