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
use core::sync::atomic::{AtomicU32, Ordering};

use super::page::Page;
use super::page_flags::{
    PG_LOCKED, PG_UPTODATE, PG_WAITERS, PG_WRITEBACK, folio_xor_flags_has_waiters,
};
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
// compare-exchange.  PG_WAITERS (bit 7) signals that tasks are sleeping in
// the hashed folio wait table.  The table and stack waiter protocol below
// mirror Linux mm/filemap.c; keeping the wait node on the blocked task's
// kernel stack avoids allocating memory while a page-cache lock is held.
//
// Ref: Linux `include/linux/pagemap.h` — folio_trylock, folio_lock, folio_unlock
//      Linux `mm/folio-compat.c`       — lock_page, unlock_page wrappers
// ---------------------------------------------------------------------------

const PAGE_WAIT_TABLE_BITS: u32 = 8;
const PAGE_WAIT_TABLE_SIZE: usize = 1 << PAGE_WAIT_TABLE_BITS;
const PAGE_LOCK_UNFAIRNESS: i32 = 5;

const WQ_FLAG_EXCLUSIVE: u32 = 0x01;
const WQ_FLAG_WOKEN: u32 = 0x02;
const WQ_FLAG_CUSTOM: u32 = 0x04;
const WQ_FLAG_DONE: u32 = 0x08;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FolioWaitBehavior {
    Exclusive,
    Shared,
}

struct FolioWaiter {
    prev: *mut FolioWaiter,
    next: *mut FolioWaiter,
    task: *mut crate::kernel::task::TaskStruct,
    page: *mut Page,
    bit: u64,
    flags: AtomicU32,
    queued: bool,
}

impl FolioWaiter {
    const fn new(task: *mut crate::kernel::task::TaskStruct, page: *mut Page, bit: u64) -> Self {
        Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
            task,
            page,
            bit,
            flags: AtomicU32::new(0),
            queued: false,
        }
    }
}

struct FolioWaitList {
    head: *mut FolioWaiter,
    tail: *mut FolioWaiter,
}

unsafe impl Send for FolioWaitList {}

impl FolioWaitList {
    const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
        }
    }

    unsafe fn push_tail(&mut self, waiter: *mut FolioWaiter) {
        unsafe {
            (*waiter).prev = self.tail;
            (*waiter).next = core::ptr::null_mut();
            (*waiter).queued = true;
            if self.tail.is_null() {
                self.head = waiter;
            } else {
                (*self.tail).next = waiter;
            }
            self.tail = waiter;
        }
    }

    unsafe fn remove(&mut self, waiter: *mut FolioWaiter) {
        if waiter.is_null() || unsafe { !(*waiter).queued } {
            return;
        }
        unsafe {
            let prev = (*waiter).prev;
            let next = (*waiter).next;
            if prev.is_null() {
                self.head = next;
            } else {
                (*prev).next = next;
            }
            if next.is_null() {
                self.tail = prev;
            } else {
                (*next).prev = prev;
            }
            (*waiter).prev = core::ptr::null_mut();
            (*waiter).next = core::ptr::null_mut();
            (*waiter).queued = false;
        }
    }

    unsafe fn has_page(&self, page: *mut Page) -> bool {
        let mut waiter = self.head;
        while !waiter.is_null() {
            if unsafe { (*waiter).page == page } {
                return true;
            }
            waiter = unsafe { (*waiter).next };
        }
        false
    }
}

struct FolioWaitQueue {
    waiters: spin::Mutex<FolioWaitList>,
}

impl FolioWaitQueue {
    const fn new() -> Self {
        Self {
            waiters: spin::Mutex::new(FolioWaitList::new()),
        }
    }

    fn with_waiters<R>(&self, f: impl FnOnce(&mut FolioWaitList) -> R) -> R {
        let irq_flags = crate::kernel::locking::irqflags::local_irq_save();
        let result = {
            let mut waiters = self.waiters.lock();
            f(&mut waiters)
        };
        crate::kernel::locking::irqflags::local_irq_restore(irq_flags);
        result
    }
}

static FOLIO_WAIT_TABLE: [FolioWaitQueue; PAGE_WAIT_TABLE_SIZE] =
    [const { FolioWaitQueue::new() }; PAGE_WAIT_TABLE_SIZE];

#[inline]
fn folio_waitqueue(page: *mut Page) -> &'static FolioWaitQueue {
    // Linux hash_ptr(ptr, 8): hash_64_generic() with GOLDEN_RATIO_64.
    let hash =
        (page as usize as u64).wrapping_mul(0x61c8_8646_80b5_83eb) >> (64 - PAGE_WAIT_TABLE_BITS);
    &FOLIO_WAIT_TABLE[hash as usize]
}

unsafe fn finish_folio_wait(queue: &FolioWaitQueue, waiter: *mut FolioWaiter) {
    queue.with_waiters(|waiters| unsafe {
        waiters.remove(waiter);
    });
    let task = unsafe { (*waiter).task };
    if !task.is_null() {
        unsafe {
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
        }
    }
}

unsafe fn folio_wake_bit(page: *mut Page, bit: u64) {
    let queue = folio_waitqueue(page);
    queue.with_waiters(|waiters| unsafe {
        let mut waiter = waiters.head;
        while !waiter.is_null() {
            let next = (*waiter).next;
            if (*waiter).page != page || (*waiter).bit != bit {
                waiter = next;
                continue;
            }

            let mut flags = (*waiter).flags.load(Ordering::Relaxed);
            if flags & WQ_FLAG_EXCLUSIVE != 0 {
                if (&*page).flags.load(Ordering::Acquire) & bit != 0 {
                    break;
                }
                if flags & WQ_FLAG_CUSTOM != 0 {
                    if (&*page).flags.fetch_or(bit, Ordering::Acquire) & bit != 0 {
                        break;
                    }
                    flags |= WQ_FLAG_DONE;
                }
            }

            (*waiter)
                .flags
                .store(flags | WQ_FLAG_WOKEN, Ordering::Release);
            let task = (*waiter).task;
            // Linux wake_page_function() wakes before deleting the stack
            // entry.  The queue lock prevents the waiter from completing
            // finish_wait() and leaving this stack frame until deletion.
            if !task.is_null() {
                crate::kernel::sched::wake_task_normal(task);
            }
            waiters.remove(waiter);
            if flags & WQ_FLAG_EXCLUSIVE != 0 {
                break;
            }
            waiter = next;
        }

        if !waiters.has_page(page) {
            (&*page).flags.fetch_and(!PG_WAITERS, Ordering::Relaxed);
        }
    });
}

unsafe fn folio_wait_bit_common(page: *mut Page, bit: u64, behavior: FolioWaitBehavior) {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        match behavior {
            FolioWaitBehavior::Exclusive => {
                while !unsafe { try_lock_page(page) } {
                    core::hint::spin_loop();
                }
            }
            FolioWaitBehavior::Shared => {
                while unsafe { (&*page).flags.load(Ordering::Acquire) & bit != 0 } {
                    core::hint::spin_loop();
                }
            }
        }
        return;
    }

    let queue = folio_waitqueue(page);
    let mut waiter = FolioWaiter::new(task, page, bit);
    let waiter_ptr = core::ptr::addr_of_mut!(waiter);
    let mut unfairness = PAGE_LOCK_UNFAIRNESS;

    'repeat: loop {
        let mut wait_flags = 0;
        if behavior == FolioWaitBehavior::Exclusive {
            wait_flags = WQ_FLAG_EXCLUSIVE;
            unfairness -= 1;
            if unfairness < 0 {
                wait_flags |= WQ_FLAG_CUSTOM;
            }
        }
        unsafe { (*waiter_ptr).flags.store(wait_flags, Ordering::Relaxed) };

        queue.with_waiters(|waiters| unsafe {
            (&*page).flags.fetch_or(PG_WAITERS, Ordering::Relaxed);
            let done = if behavior == FolioWaitBehavior::Exclusive {
                (&*page).flags.fetch_or(bit, Ordering::Acquire) & bit == 0
            } else {
                (&*page).flags.load(Ordering::Acquire) & bit == 0
            };
            if done {
                (*waiter_ptr)
                    .flags
                    .fetch_or(WQ_FLAG_WOKEN | WQ_FLAG_DONE, Ordering::Release);
            } else if !(*waiter_ptr).queued {
                waiters.push_tail(waiter_ptr);
            }
        });

        loop {
            unsafe {
                (*task).__state.store(
                    crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE,
                    Ordering::SeqCst,
                );
            }
            let flags = unsafe { (*waiter_ptr).flags.load(Ordering::Acquire) };
            if flags & WQ_FLAG_WOKEN == 0 {
                unsafe { crate::kernel::sched::schedule_with_irqs_enabled() };
                continue;
            }
            if behavior == FolioWaitBehavior::Shared || flags & WQ_FLAG_DONE != 0 {
                break 'repeat;
            }
            if unsafe { try_lock_page(page) } {
                unsafe {
                    (*waiter_ptr)
                        .flags
                        .fetch_or(WQ_FLAG_DONE, Ordering::Relaxed);
                }
                break 'repeat;
            }
            continue 'repeat;
        }
    }

    unsafe { finish_folio_wait(queue, waiter_ptr) };
}

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

/// Lock the page, sleeping on the hashed folio waitqueue when contended.
///
/// Equivalent to Linux `lock_page()` / `folio_lock()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_lock()`
#[inline]
pub unsafe fn lock_page(page: *mut Page) {
    if !unsafe { try_lock_page(page) } {
        unsafe { folio_wait_bit_common(page, PG_LOCKED, FolioWaitBehavior::Exclusive) };
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
/// Clears `PG_LOCKED` with release ordering, then wakes matching sleepers when
/// `PG_WAITERS` says the hashed waitqueue may contain one.
///
/// Equivalent to Linux `unlock_page()` / `folio_unlock()`.
///
/// Ref: Linux `mm/folio-compat.c` — `unlock_page()`
#[inline]
pub unsafe fn unlock_page(page: *mut Page) {
    debug_assert_ne!(
        unsafe { (&*page).flags.load(Ordering::Relaxed) } & PG_LOCKED,
        0,
        "unlock_page on an unlocked page"
    );
    if folio_xor_flags_has_waiters(page, PG_LOCKED) {
        unsafe { folio_wake_bit(page, PG_LOCKED) };
    }
}

/// Sleep until the page is no longer locked.
///
/// Callers that need to wait for a page to become uptodate should call this
/// after `lock_page` to avoid the "lock then wait" pattern.
///
/// Equivalent to Linux `wait_on_page_locked()` / `folio_wait_locked()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_wait_locked()`
#[inline]
pub unsafe fn wait_on_page_locked(page: *mut Page) {
    if unsafe { (&*page).flags.load(Ordering::Acquire) } & PG_LOCKED != 0 {
        unsafe { folio_wait_bit_common(page, PG_LOCKED, FolioWaitBehavior::Shared) };
    }
}

/// Sleep until the page is no longer under writeback.
///
/// Equivalent to Linux `wait_on_page_writeback()` / `folio_wait_writeback()`.
///
/// Ref: Linux `include/linux/pagemap.h` — `folio_wait_writeback()`
#[inline]
pub unsafe fn wait_on_page_writeback(page: *mut Page) {
    if unsafe { (&*page).flags.load(Ordering::Acquire) } & PG_WRITEBACK != 0 {
        unsafe { folio_wait_bit_common(page, PG_WRITEBACK, FolioWaitBehavior::Shared) };
    }
}

/// Clear `PG_WRITEBACK` and wake the page's matching hashed waitqueue.
/// Returns the flags observed before the clear.
#[inline]
pub unsafe fn clear_page_writeback_and_wake(page: *mut Page) -> u64 {
    let old = unsafe { (&*page).flags.fetch_and(!PG_WRITEBACK, Ordering::Release) };
    if old & PG_WRITEBACK != 0 && old & PG_WAITERS != 0 {
        unsafe { folio_wake_bit(page, PG_WRITEBACK) };
    }
    old
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
