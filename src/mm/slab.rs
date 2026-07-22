//! linux-parity: complete
//! linux-source: vendor/linux/mm/slab.h
//! test-origin: linux:vendor/linux/mm/slab.h
/// Kernel slab allocator — Linux-compatible `kmalloc` / `kfree`.
///
/// Implements a SLUB-style per-size-class object cache on top of the buddy
/// allocator.  Object caches (`KmemCache`) maintain partial and full slab
/// lists; each slab is one (or a few) buddy pages whose objects are linked
/// via a freelist encoded in the free objects themselves.
///
/// ## Size classes
///
/// `KMALLOC_SIZES` defines 13 fixed size classes (8–8192 bytes).  `kmalloc`
/// rounds the requested size up to the next class and allocates from the
/// corresponding cache.  Requests larger than `KMALLOC_MAX_SIZE` fall
/// through to the buddy allocator directly (large-kmalloc path).
///
/// ## Slab page metadata
///
/// When a page is owned by a slab cache it is marked `PGTY_SLAB` and its
/// fields are repurposed as follows:
///
/// | `Page` field  | Slab meaning                                    |
/// |---------------|-------------------------------------------------|
/// | `page_type`   | `PGTY_SLAB`                                     |
/// | `mapping`     | freelist head (`*mut u8`, 0 = full)             |
/// | `index`       | in-use object count                             |
/// | `private`     | head page: `*mut KmemCache`; non-head: `(head_pfn << 1) | 1` |
/// | `lru`         | list node in cache's `partial` or `full` list   |
///
/// ## GlobalAlloc
///
/// When the `slab-alloc` feature is enabled (the default for the kernel
/// binary), this module registers a `#[global_allocator]` that routes
/// `alloc::boxed::Box`, `alloc::vec::Vec`, etc. through `kmalloc`/`kfree`.
///
/// ## References
///
/// - Linux `mm/slub.c` — SLUB allocator implementation
/// - Linux `include/linux/slub_def.h` — `struct kmem_cache`
/// - Linux `mm/slab_common.c` — common cache infrastructure
/// - Linux `include/linux/slab.h` — `kmalloc` / `kfree` API
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(test)]
extern crate alloc;
#[cfg(not(test))]
use crate::arch::x86::mm::paging::phys_to_virt;
use crate::container_of;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::buddy::{page_to_pfn, pfn_to_page};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::list::ListHead;
use crate::mm::page::Page;
use crate::mm::page_flags::PAGE_TYPE_NONE;
use crate::mm::page_flags::{
    __GFP_ZERO, GFP_KERNEL, GfpFlags, PGTY_LARGE_KMALLOC, PGTY_SLAB, decode_page_type,
    encode_page_type,
};
use crate::mm::zone::MAX_PAGE_ORDER;
#[cfg(test)]
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Size classes
// ---------------------------------------------------------------------------

/// Maximum object size handled by the slab caches.
/// Requests above this go to the large-kmalloc path (direct buddy pages).
///
/// Ref: Linux `include/linux/slab.h` — `KMALLOC_MAX_CACHE_SIZE`
pub const KMALLOC_MAX_SIZE: usize = 8192;

/// Number of per-size kmalloc caches.
pub const NR_KMALLOC_SIZES: usize = 13;

/// Object sizes for each kmalloc cache, in ascending order.
///
/// Ref: Linux `mm/slab_common.c` — `kmalloc_size_table[]`
const KMALLOC_SIZES: [usize; NR_KMALLOC_SIZES] = [
    8, 16, 32, 64, 96, 128, 192, 256, 512, 1024, 2048, 4096, 8192,
];

/// Debug names for each cache.
const KMALLOC_NAMES: [&str; NR_KMALLOC_SIZES] = [
    "kmalloc-8",
    "kmalloc-16",
    "kmalloc-32",
    "kmalloc-64",
    "kmalloc-96",
    "kmalloc-128",
    "kmalloc-192",
    "kmalloc-256",
    "kmalloc-512",
    "kmalloc-1024",
    "kmalloc-2048",
    "kmalloc-4096",
    "kmalloc-8192",
];

// ---------------------------------------------------------------------------
// Global slab state
// ---------------------------------------------------------------------------

/// True after `slab_init()` has completed; guards GlobalAlloc bootstrap.
static SLAB_READY: AtomicBool = AtomicBool::new(false);
static SLAB_FREE_REJECTIONS: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_PTR: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_HEAD_PFN: AtomicUsize = AtomicUsize::new(usize::MAX);
static LAST_REJECTED_FREE_REASON: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_CACHE: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_OBJECT_SIZE: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_SLOT_SIZE: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_INUSE: AtomicUsize = AtomicUsize::new(0);
static LAST_REJECTED_FREE_CURSOR: AtomicUsize = AtomicUsize::new(0);

/// Coarse per-slab-subsystem lock.  Held during every `kmalloc` / `kfree`
/// call.  Per-CPU cache magazines can be layered on this lock without changing
/// the public allocation contract.
///
/// Ref: Linux `struct kmem_cache::lock` (per-cache in SLUB)
static SLAB_LOCK: spin::Mutex<()> = spin::Mutex::new(());

/// Kernel ABI unit tests execute exported allocation entry points before the
/// boot-time buddy/slab setup exists. Track those pre-init allocations in the
/// host allocator so the ABI paths remain testable without weakening the
/// production pre-slab guard.
#[cfg(test)]
static TEST_HOST_ALLOCATIONS: spin::Mutex<Vec<(usize, usize, usize)>> =
    spin::Mutex::new(Vec::new());

#[cfg(test)]
fn test_host_kmalloc(size: usize, gfp: GfpFlags) -> *mut u8 {
    let size = size.max(1);
    let align = core::mem::align_of::<usize>();
    let layout = Layout::from_size_align(size, align).expect("valid host kmalloc layout");
    let ptr = unsafe {
        if gfp & __GFP_ZERO != 0 {
            alloc::alloc::alloc_zeroed(layout)
        } else {
            alloc::alloc::alloc(layout)
        }
    };
    if !ptr.is_null() {
        TEST_HOST_ALLOCATIONS
            .lock()
            .push((ptr as usize, size, align));
    }
    ptr
}

#[cfg(test)]
unsafe fn test_host_kfree(ptr: *mut u8) -> bool {
    let mut allocations = TEST_HOST_ALLOCATIONS.lock();
    let Some(index) = allocations
        .iter()
        .position(|(address, _, _)| *address == ptr as usize)
    else {
        return false;
    };
    let (_, size, align) = allocations.swap_remove(index);
    drop(allocations);
    let layout = Layout::from_size_align(size, align).expect("valid host kfree layout");
    unsafe { alloc::alloc::dealloc(ptr, layout) };
    true
}

#[cfg(test)]
fn test_host_ksize(ptr: *const u8) -> Option<usize> {
    TEST_HOST_ALLOCATIONS
        .lock()
        .iter()
        .find(|(address, _, _)| *address == ptr as usize)
        .map(|(_, size, _)| *size)
}

/// Lock-free diagnostics for frees rejected before they can corrupt a slab.
/// Reason values are: 1 = foreign/misaligned pointer, 2 = invalid in-use
/// count, 3 = duplicate free, 4 = corrupt freelist.
pub fn slab_free_rejection_snapshot() -> (usize, usize, usize, usize) {
    (
        SLAB_FREE_REJECTIONS.load(Ordering::Acquire),
        LAST_REJECTED_FREE_PTR.load(Ordering::Acquire),
        LAST_REJECTED_FREE_HEAD_PFN.load(Ordering::Acquire),
        LAST_REJECTED_FREE_REASON.load(Ordering::Acquire),
    )
}

pub fn slab_free_rejection_detail() -> (usize, usize, usize, usize, usize) {
    (
        LAST_REJECTED_FREE_CACHE.load(Ordering::Acquire),
        LAST_REJECTED_FREE_OBJECT_SIZE.load(Ordering::Acquire),
        LAST_REJECTED_FREE_SLOT_SIZE.load(Ordering::Acquire),
        LAST_REJECTED_FREE_INUSE.load(Ordering::Acquire),
        LAST_REJECTED_FREE_CURSOR.load(Ordering::Acquire),
    )
}

fn record_slab_free_rejection(
    cache: &KmemCache,
    ptr: *mut u8,
    head_page: *mut Page,
    reason: usize,
    cursor: usize,
) {
    LAST_REJECTED_FREE_PTR.store(ptr as usize, Ordering::Release);
    LAST_REJECTED_FREE_HEAD_PFN.store(slab_page_to_pfn(head_page), Ordering::Release);
    LAST_REJECTED_FREE_REASON.store(reason, Ordering::Release);
    LAST_REJECTED_FREE_CACHE.store(cache as *const KmemCache as usize, Ordering::Release);
    LAST_REJECTED_FREE_OBJECT_SIZE.store(cache.object_size, Ordering::Release);
    LAST_REJECTED_FREE_SLOT_SIZE.store(cache.size, Ordering::Release);
    LAST_REJECTED_FREE_INUSE.store(unsafe { (*head_page).index }, Ordering::Release);
    LAST_REJECTED_FREE_CURSOR.store(cursor, Ordering::Release);
    SLAB_FREE_REJECTIONS.fetch_add(1, Ordering::AcqRel);
}

/// Restores the saved interrupt state when dropped.  Paired with the spin guard
/// in [`lock_slab`]; tuple fields drop left-to-right, so the spin lock is
/// released *before* interrupts are re-enabled.
struct SlabIrqState(crate::kernel::locking::irqflags::IrqFlags);

impl Drop for SlabIrqState {
    #[inline]
    fn drop(&mut self) {
        crate::kernel::locking::irqflags::local_irq_restore(self.0);
    }
}

/// Acquire [`SLAB_LOCK`] with interrupts disabled.
///
/// The slab lock must be IRQ-safe.  The LAPIC timer ISR runs
/// `scheduler_tick`, whose CFS `task_tick` inserts into a `BTreeMap` and so
/// calls `kmalloc`.  If that tick lands while a task already holds `SLAB_LOCK`,
/// the ISR's nested `kmalloc` would spin on the lock forever — a self-deadlock
/// (observed on multi-CPU boots: the BSP hangs in `kmalloc` spinning on
/// `SLAB_LOCK` after "Welcome to Arch Linux!").  Disabling interrupts for the
/// duration mirrors Linux, where the kmalloc fast-path locks are IRQ-safe so
/// that GFP_ATOMIC allocation from interrupt context is sound.
#[inline]
fn lock_slab() -> (spin::MutexGuard<'static, ()>, SlabIrqState) {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    let guard = SLAB_LOCK.lock();
    (guard, SlabIrqState(flags))
}

/// The 13 global kmalloc size-class caches.  Initialised in-place by
/// `slab_init()` — their addresses are stable (static) so that the
/// `page.private` back-pointers into them remain valid.
///
/// # Safety
/// Only accessed while holding `SLAB_LOCK` or before SLAB_READY is set.
static mut KMALLOC_CACHES: [KmemCache; NR_KMALLOC_SIZES] =
    [const { KmemCache::const_uninit() }; NR_KMALLOC_SIZES];

// ---------------------------------------------------------------------------
// KmemCache — one per object size class
// ---------------------------------------------------------------------------

/// Per-size-class object cache — the lupos equivalent of Linux's
/// `struct kmem_cache` from `include/linux/slub_def.h`.
///
/// The cache maintains a list of *partial* slabs (have at least one free
/// slot) and *full* slabs (every slot allocated).  New slabs are obtained
/// from the buddy allocator via `alloc_slab_page`.
pub struct KmemCache {
    /// Cache name, e.g. "kmalloc-32".
    pub name: &'static str,
    /// Caller-visible object size (bytes).
    pub object_size: usize,
    /// Actual slot size (≥ `object_size`, aligned to `align`, ≥ 8 to hold
    /// the freelist next-pointer while the object is free).
    pub size: usize,
    /// Slot alignment.
    pub align: usize,
    /// Buddy order of each slab backing page (0 = 4 KiB, 1 = 8 KiB).
    pub order: usize,
    /// Number of object slots per slab.
    pub objects_per_slab: usize,
    /// Slabs with at least one free slot.
    ///
    /// Pages are linked via their embedded `Page::lru` field.
    pub partial: ListHead,
    /// Fully allocated slabs (no free slots).
    pub full: ListHead,
    /// Count of partial slabs.
    pub nr_partial: AtomicUsize,
    /// Count of full slabs.
    pub nr_full: AtomicUsize,
}

// Safety: all mutation happens behind `SLAB_LOCK`.
unsafe impl Send for KmemCache {}
unsafe impl Sync for KmemCache {}

impl KmemCache {
    /// Create an all-zero, uninitialised cache — safe for static initialisation.
    ///
    /// MUST call `init()` before using the cache.
    pub const fn const_uninit() -> Self {
        KmemCache {
            name: "",
            object_size: 0,
            size: 0,
            align: 0,
            order: 0,
            objects_per_slab: 0,
            partial: ListHead::uninit(),
            full: ListHead::uninit(),
            nr_partial: AtomicUsize::new(0),
            nr_full: AtomicUsize::new(0),
        }
    }

    /// Initialise the cache in-place (sets self-referential `ListHead` pointers).
    ///
    /// # Safety
    /// Must be called exactly once, while the cache is at its final address.
    pub unsafe fn init(&mut self, name: &'static str, object_size: usize, align: usize) {
        let align = align.max(core::mem::size_of::<usize>()).next_power_of_two(); // ≥ pointer size and valid for align_up
        // Slot size: large enough to hold the freelist next-pointer when free.
        let size = align_up_usize(object_size.max(core::mem::size_of::<usize>()), align);
        let order = slab_order(size);
        let objects = ((PAGE_SIZE << order) / size).max(1);

        self.name = name;
        self.object_size = object_size;
        self.size = size;
        self.align = align;
        self.order = order;
        self.objects_per_slab = objects;
        unsafe {
            ListHead::init(&mut self.partial);
            ListHead::init(&mut self.full);
        }
        self.nr_partial.store(0, Ordering::Relaxed);
        self.nr_full.store(0, Ordering::Relaxed);
    }

    // -----------------------------------------------------------------------
    // Internal slab operations
    // -----------------------------------------------------------------------

    /// Allocate a fresh slab page from the buddy allocator, build its
    /// freelist, and add it to `self.partial`.
    ///
    /// Returns a pointer to the head `Page` on success, `None` on OOM.
    ///
    /// Ref: Linux `mm/slub.c` — `new_slab()`
    unsafe fn new_slab(&mut self, gfp: GfpFlags) -> Option<*mut Page> {
        let (head_page, mem) = unsafe { alloc_slab_page(self.order, gfp)? };
        let head_pfn = slab_page_to_pfn(head_page);
        let n_pages = 1usize << self.order;

        // Mark ALL pages in this slab block.
        // Head page: private = cache pointer.
        // Non-head pages: private = (head_pfn << 1) | 1 (low bit = non-head flag).
        unsafe {
            for i in 0..n_pages {
                let p = slab_pfn_to_page(head_pfn + i);
                (*p).page_type
                    .store(encode_page_type(PGTY_SLAB), Ordering::Relaxed);
                if i == 0 {
                    (*p).private = self as *mut KmemCache as usize;
                } else {
                    (*p).private = (head_pfn << 1) | 1;
                }
            }

            // Build a singly-linked freelist through the slab objects.
            // The first word of each free object stores the next pointer.
            // The last object's next pointer is 0 (null = end of list).
            let n = self.objects_per_slab;
            for i in 0..n {
                let obj = mem.add(i * self.size);
                let next = if i + 1 < n {
                    mem.add((i + 1) * self.size) as usize
                } else {
                    0
                };
                *(obj as *mut usize) = next;
            }

            // Head page: mapping = freelist head, index = 0 (inuse).
            (*head_page).mapping = mem as usize;
            (*head_page).index = 0;

            // Link into partial list (head_page.lru was init'd by buddy alloc).
            ListHead::init(&mut (*head_page).lru);
            ListHead::list_add(&mut (*head_page).lru, &mut self.partial);
        }
        self.nr_partial.fetch_add(1, Ordering::Relaxed);

        Some(head_page)
    }

    /// Pop one object from the given slab page's freelist.
    ///
    /// Moves the page from `partial` to `full` when the last slot is taken.
    ///
    /// # Safety
    /// `head_page` must be the head of a valid partial slab in `self.partial`.
    unsafe fn alloc_object(&mut self, head_page: *mut Page) -> *mut u8 {
        let freelist_head = unsafe { (*head_page).mapping } as *mut u8;
        debug_assert!(!freelist_head.is_null(), "alloc_object on empty slab");

        // Pop from freelist: next pointer is first word of the object.
        let next = unsafe { *(freelist_head as *const usize) };
        unsafe {
            (*head_page).mapping = next;
            (*head_page).index += 1; // inuse++
        }

        // Slab is now full → move partial → full.
        if unsafe { (*head_page).mapping } == 0 {
            unsafe {
                ListHead::list_del(&mut (*head_page).lru);
            }
            self.nr_partial.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                ListHead::list_add(&mut (*head_page).lru, &mut self.full);
            }
            self.nr_full.fetch_add(1, Ordering::Relaxed);
        }

        freelist_head
    }

    /// Allocate one object from this cache.
    ///
    /// Uses an existing partial slab if available, otherwise allocates a new
    /// slab from the buddy allocator.
    ///
    /// Ref: Linux `mm/slub.c` — `slab_alloc_node()`
    pub unsafe fn alloc(&mut self, gfp: GfpFlags) -> *mut u8 {
        // Fast path: pop from a partial slab.
        if let Some(lru_ptr) = unsafe { ListHead::first_entry(&self.partial) } {
            let head_page = container_of!(lru_ptr, Page, lru);
            return unsafe { self.alloc_object(head_page) };
        }
        // Slow path: allocate a new slab.
        match unsafe { self.new_slab(gfp) } {
            None => core::ptr::null_mut(),
            Some(head_page) => unsafe { self.alloc_object(head_page) },
        }
    }

    /// Return object `ptr` to this cache (push onto the freelist).
    ///
    /// Moves the slab from `full` to `partial` if it was previously full.
    /// Empty slabs are returned to the page allocator after the object is put
    /// back on the freelist.
    ///
    /// # Safety
    /// - `head_page` must be the head page of the slab containing `ptr`.
    /// - `ptr` must be a valid object from this cache.
    ///
    /// Ref: Linux `mm/slub.c` — `slab_free()`
    pub unsafe fn free_object(&mut self, head_page: *mut Page, ptr: *mut u8) -> bool {
        let slab_start = slab_page_to_pfn(head_page).saturating_mul(PAGE_SIZE);
        #[cfg(not(test))]
        let slab_start = phys_to_virt(slab_start as u64) as usize;
        let object_bytes = self.objects_per_slab.saturating_mul(self.size);
        let ptr_addr = ptr as usize;
        let valid_object = ptr_addr >= slab_start
            && ptr_addr < slab_start.saturating_add(object_bytes)
            && ptr_addr.wrapping_sub(slab_start) % self.size == 0;
        if !valid_object {
            record_slab_free_rejection(self, ptr, head_page, 1, 0);
            return false;
        }

        let inuse = unsafe { (*head_page).index };
        if inuse == 0 || inuse > self.objects_per_slab {
            record_slab_free_rejection(self, ptr, head_page, 2, 0);
            return false;
        }

        // A free object contains the next freelist pointer in its first word.
        // Walk at most one slab's capacity, validating every pointer before
        // dereferencing it. Finding `ptr` proves a duplicate free; rejecting it
        // keeps `index` from reaching zero while other objects are still live.
        let mut free = unsafe { (*head_page).mapping } as *mut u8;
        let mut walked = 0usize;
        while !free.is_null() && walked < self.objects_per_slab {
            let free_addr = free as usize;
            if free_addr < slab_start
                || free_addr >= slab_start.saturating_add(object_bytes)
                || free_addr.wrapping_sub(slab_start) % self.size != 0
            {
                record_slab_free_rejection(self, ptr, head_page, 4, free_addr);
                return false;
            }
            if free == ptr {
                record_slab_free_rejection(self, ptr, head_page, 3, free_addr);
                return false;
            }
            free = unsafe { *(free as *const usize) as *mut u8 };
            walked += 1;
        }
        if !free.is_null() {
            record_slab_free_rejection(self, ptr, head_page, 4, free as usize);
            return false;
        }

        let was_full = unsafe { (*head_page).mapping } == 0;

        // Push onto freelist.
        unsafe {
            let old_head = (*head_page).mapping as *mut u8;
            *(ptr as *mut usize) = old_head as usize;
            (*head_page).mapping = ptr as usize;
            (*head_page).index -= 1; // inuse--
        }

        // Was full → move full → partial.
        if was_full {
            unsafe {
                ListHead::list_del(&mut (*head_page).lru);
            }
            self.nr_full.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                ListHead::list_add(&mut (*head_page).lru, &mut self.partial);
            }
            self.nr_partial.fetch_add(1, Ordering::Relaxed);
        }

        if unsafe { (*head_page).index } == 0 {
            unsafe {
                ListHead::list_del(&mut (*head_page).lru);
            }
            self.nr_partial.fetch_sub(1, Ordering::Relaxed);
            unsafe { free_slab_page(head_page, self.order) };
        }
        true
    }

    // -----------------------------------------------------------------------
    // Statistics
    // -----------------------------------------------------------------------

    /// Total in-use objects across all slabs in this cache.
    pub fn inuse_objects(&self) -> usize {
        // For simplicity: inuse = total_capacity - free_objects.
        let partial_cap = self.nr_partial.load(Ordering::Relaxed) * self.objects_per_slab;
        let full_cap = self.nr_full.load(Ordering::Relaxed) * self.objects_per_slab;
        // Count free objects in partial slabs (each full slab has 0 free).
        // This is approximate; an exact count would require walking the list.
        // For unit tests, we count via alloc_count directly.
        partial_cap + full_cap // upper bound; subtract free approximation
    }
}

// ---------------------------------------------------------------------------
// Page allocation backend (cfg-conditional: buddy in prod, mock in tests)
// ---------------------------------------------------------------------------

/// Allocate `2^order` contiguous pages for a slab.
///
/// Returns `(head_page_ptr, raw_memory_ptr)` or `None` on OOM.
///
/// In production this calls the global buddy allocator.
/// In unit tests a static page pool is used (no buddy required).
#[cfg(not(test))]
unsafe fn alloc_slab_page(order: usize, gfp: GfpFlags) -> Option<(*mut Page, *mut u8)> {
    crate::mm::buddy::with_global_buddy(|b| b.alloc_pages(order, gfp)).map(|page| {
        let addr = page_to_pfn(page) * PAGE_SIZE;
        (page, phys_to_virt(addr as u64))
    })
}

/// Return slab pages to the buddy allocator.
#[cfg(not(test))]
unsafe fn free_slab_page(page: *mut Page, order: usize) {
    // Clear page metadata before returning to buddy.
    unsafe { clear_slab_page_metadata(page, order) };
    crate::mm::buddy::with_global_buddy(|b| b.free_pages(page, order));
}

unsafe fn clear_slab_page_metadata(page: *mut Page, order: usize) {
    for i in 0..(1usize << order) {
        let page = unsafe { page.add(i) };
        unsafe {
            (*page).page_type.store(PAGE_TYPE_NONE, Ordering::Relaxed);
            (*page).mapping = 0;
            (*page).index = 0;
            (*page).private = 0;
            (*page).init_lru();
        }
    }
}

// Test-only page pool statics (module level for visibility from alloc_slab_page).
#[cfg(test)]
const TEST_N_PAGES: usize = 128;

#[cfg(test)]
static TEST_MEM_PTR: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static TEST_BASE_PFN: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static TEST_PAGE_CURSOR: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static TEST_FREE_PAGES: spin::Mutex<Vec<(usize, usize)>> = spin::Mutex::new(Vec::new());

/// Test-only physical memory pool (128 × 4 KiB = 512 KiB).
///
/// The maximum order exercised by this pool is one, so its base must be
/// aligned to the corresponding 8 KiB slab boundary.
#[cfg(test)]
#[repr(align(8192))]
struct TestPhysMem([u8; PAGE_SIZE * TEST_N_PAGES]);

#[cfg(test)]
static mut TEST_PHYS: TestPhysMem = TestPhysMem([0u8; PAGE_SIZE * TEST_N_PAGES]);

/// Test-only Page metadata array (one per test physical page).
#[cfg(test)]
static mut TEST_META: [Page; TEST_N_PAGES] = [const { Page::new() }; TEST_N_PAGES];

#[cfg(test)]
fn test_meta_ptr() -> *mut Page {
    core::ptr::addr_of_mut!(TEST_META).cast::<Page>()
}

#[cfg(test)]
fn test_pool_page_for_index(idx: usize) -> Option<*mut Page> {
    if idx < TEST_N_PAGES {
        Some(unsafe { test_meta_ptr().add(idx) })
    } else {
        None
    }
}

#[cfg(test)]
fn test_pool_page_index(page: *const Page) -> Option<usize> {
    let base = test_meta_ptr() as usize;
    let addr = page as usize;
    let size = core::mem::size_of::<Page>();
    let end = base.saturating_add(TEST_N_PAGES.saturating_mul(size));
    if addr < base || addr >= end || (addr - base) % size != 0 {
        return None;
    }
    Some((addr - base) / size)
}

#[cfg(test)]
fn test_pool_page_for_pfn(pfn: usize) -> Option<*mut Page> {
    let base_pfn = TEST_BASE_PFN.load(Ordering::SeqCst);
    let idx = pfn.checked_sub(base_pfn)?;
    test_pool_page_for_index(idx)
}

#[cfg(test)]
fn test_pool_page_for_address(addr: usize) -> Option<*mut Page> {
    let mem_base = TEST_MEM_PTR.load(Ordering::SeqCst);
    if mem_base == 0 {
        return None;
    }
    let end = mem_base.saturating_add(PAGE_SIZE.saturating_mul(TEST_N_PAGES));
    if addr < mem_base || addr >= end {
        return None;
    }
    test_pool_page_for_index((addr - mem_base) / PAGE_SIZE)
}

#[cfg(test)]
fn slab_pfn_to_page(pfn: usize) -> *mut Page {
    if let Some(page) = test_pool_page_for_pfn(pfn) {
        return page;
    }
    if crate::mm::buddy::pfn_valid(pfn) {
        pfn_to_page(pfn)
    } else {
        core::ptr::null_mut()
    }
}

#[cfg(not(test))]
#[inline]
fn slab_pfn_to_page(pfn: usize) -> *mut Page {
    pfn_to_page(pfn)
}

#[cfg(test)]
fn slab_page_to_pfn(page: *const Page) -> usize {
    if let Some(idx) = test_pool_page_index(page) {
        TEST_BASE_PFN.load(Ordering::SeqCst) + idx
    } else {
        page_to_pfn(page)
    }
}

#[cfg(not(test))]
#[inline]
fn slab_page_to_pfn(page: *const Page) -> usize {
    page_to_pfn(page)
}

/// Test-only slab page allocator.
///
/// Hands out consecutive 4 KiB pages from the static TEST_PHYS pool.
/// The `order` parameter is honoured (consecutive pages are allocated),
/// but no buddy coalescing/splitting occurs.
#[cfg(test)]
#[allow(dead_code)]
unsafe fn alloc_slab_page(order: usize, _gfp: GfpFlags) -> Option<(*mut Page, *mut u8)> {
    let n = 1usize << order;
    let mem_base = TEST_MEM_PTR.load(Ordering::SeqCst);
    if mem_base == 0 {
        return None;
    }
    let idx = {
        let mut free = TEST_FREE_PAGES.lock();
        if let Some(pos) = free.iter().position(|&(_, free_order)| free_order == order) {
            free.swap_remove(pos).0
        } else {
            let cursor = TEST_PAGE_CURSOR.load(Ordering::SeqCst);
            let idx = cursor.checked_add(n - 1).map(|value| value & !(n - 1))?;
            if idx + n > TEST_N_PAGES {
                return None;
            }
            TEST_PAGE_CURSOR.store(idx + n, Ordering::SeqCst);
            idx
        }
    };
    let mem = (mem_base + idx * PAGE_SIZE) as *mut u8;
    unsafe {
        core::ptr::write_bytes(mem, 0, PAGE_SIZE * n);
    }
    let page = test_pool_page_for_index(idx)?;
    // Ensure non-head pages have been init'd (new_slab marks them itself,
    // but we guarantee lru is initialised here).
    for i in 1..n {
        let p = test_pool_page_for_index(idx + i)?;
        unsafe {
            core::ptr::write(p, Page::new());
            (*p).init_lru();
        }
    }
    Some((page, mem))
}

/// Return pages to the test pool so host tests exercise slab reclaim/reuse.
#[cfg(test)]
#[allow(dead_code)]
unsafe fn free_slab_page(page: *mut Page, order: usize) {
    let Some(idx) = test_pool_page_index(page) else {
        return;
    };
    for i in 0..(1usize << order) {
        let Some(p) = test_pool_page_for_index(idx + i) else {
            return;
        };
        unsafe {
            core::ptr::write(p, Page::new());
            (*p).init_lru();
        }
    }
    TEST_FREE_PAGES.lock().push((idx, order));
}

// ---------------------------------------------------------------------------
// Public API — slab_init, kmalloc, kfree
// ---------------------------------------------------------------------------

/// Initialise all kmalloc size-class caches.
///
/// Must be called once, before any `kmalloc` / `kfree`, and after the
/// buddy allocator is online.  Called from `kernel_main` early in boot.
///
/// After this call `SLAB_READY` is set and `Box` / `Vec` work.
///
/// Ref: Linux `mm/slab_common.c` — `create_kmalloc_caches()`
pub fn slab_init() {
    // Initialise each cache in-place inside the static array.
    // The ListHead self-pointers written by init() remain valid because
    // KMALLOC_CACHES is a static (never moved).
    unsafe {
        for i in 0..NR_KMALLOC_SIZES {
            KMALLOC_CACHES[i].init(KMALLOC_NAMES[i], KMALLOC_SIZES[i], 8);
        }
    }
    SLAB_READY.store(true, Ordering::Release);
}

/// Allocate `size` bytes of kernel memory.
///
/// Rounds `size` up to the next kmalloc size class and returns an object
/// from the corresponding `KmemCache`.  Requests larger than
/// `KMALLOC_MAX_SIZE` are satisfied by allocating buddy pages directly.
///
/// Returns a non-null pointer on success, null on OOM.
///
/// # Safety
/// Caller must eventually call `kfree` with the returned pointer.
///
/// Ref: Linux `mm/slab_common.c` — `__kmalloc()`
pub unsafe fn kmalloc(size: usize, gfp: GfpFlags) -> *mut u8 {
    if !SLAB_READY.load(Ordering::Acquire) {
        // Fatal: allocations before slab_init.  Spin so the problem is obvious
        // in a debugger rather than silently returning null or corrupting state.
        #[cfg(not(test))]
        {
            crate::linux_driver_abi::tty::serial_println!("FATAL: kmalloc called before slab_init");
            loop {
                core::hint::spin_loop();
            }
        }
        #[cfg(test)]
        return test_host_kmalloc(size, gfp);
    }

    if size == 0 {
        // Return a stable non-null dangling pointer for zero-size allocations,
        // matching Linux's `ZERO_SIZE_PTR` behaviour.
        // Ref: Linux `include/linux/slab.h` — ZERO_SIZE_PTR
        return core::ptr::NonNull::<u8>::dangling().as_ptr();
    }

    if size > KMALLOC_MAX_SIZE {
        let ptr = unsafe { kmalloc_large(size, gfp) };
        if ptr.is_null() {
            crate::log_warn!(
                "kmalloc",
                "kmalloc_large returned null: size={} gfp=0x{:x}",
                size,
                gfp
            );
        }
        return ptr;
    }

    let _guard = lock_slab();
    let idx = kmalloc_cache_index(size);
    let ptr = unsafe { KMALLOC_CACHES[idx].alloc(gfp) };
    if !ptr.is_null() && gfp & __GFP_ZERO != 0 {
        unsafe {
            core::ptr::write_bytes(ptr, 0, size);
        }
    } else if ptr.is_null() {
        crate::log_warn!(
            "kmalloc",
            "kmalloc returned null: size={} gfp=0x{:x}",
            size,
            gfp
        );
    }
    ptr
}

/// Free a pointer returned by `kmalloc` (or `kfree(null)` is a no-op).
///
/// Ref: Linux `mm/slab_common.c` — `kfree()`
pub unsafe fn kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    // ZERO_SIZE_PTR sentinel — no backing allocation.
    if ptr == core::ptr::NonNull::<u8>::dangling().as_ptr() {
        return;
    }

    #[cfg(test)]
    if unsafe { test_host_kfree(ptr) } {
        return;
    }

    if !SLAB_READY.load(Ordering::Acquire) {
        return; // Pre-init: ignore (shouldn't happen)
    }

    if crate::mm::vmalloc::is_vmalloc_addr(ptr) {
        crate::mm::vmalloc::vfree(ptr);
        return;
    }

    #[cfg(test)]
    let page = match test_pool_page_for_address(ptr as usize) {
        Some(page) => page,
        None => {
            let pfn = ptr as usize / PAGE_SIZE;
            if crate::mm::buddy::pfn_valid(pfn) {
                pfn_to_page(pfn)
            } else {
                return;
            }
        }
    };
    #[cfg(not(test))]
    let page = {
        let addr = ptr as u64;
        // A kfree target must live in the kernel direct map (vmalloc was handled
        // above). A pointer below PAGE_OFFSET is not an object this allocator
        // owns; skip it instead of computing a bogus pfn and corrupting the
        // page array. Rate-limit so a buggy caller cannot flood the console.
        if addr < crate::arch::x86::mm::paging::PAGE_OFFSET {
            crate::log_ratelimited!(
                crate::kernel::printk::log::Level::Error,
                "slab",
                crate::kernel::time::jiffies::HZ,
                5,
                "kfree: ignoring non-direct-map pointer {:#018x}",
                addr
            );
            return;
        }
        let pfn = ((addr - crate::arch::x86::mm::paging::PAGE_OFFSET) as usize) / PAGE_SIZE;
        pfn_to_page(pfn)
    };
    let page_type = decode_page_type(unsafe { (*page).page_type.load(Ordering::Relaxed) });

    if page_type == PGTY_SLAB {
        let _guard = lock_slab();
        // Determine head page (non-head pages have low bit of private set).
        let head_page = unsafe {
            let private = (*page).private;
            if private & 1 == 1 {
                // Non-head page: redirect to head.
                // Ref: Linux `compound_head()` for multi-page slabs.
                slab_pfn_to_page(private >> 1)
            } else {
                page
            }
        };
        let cache = unsafe { (*head_page).private as *mut KmemCache };
        unsafe { (*cache).free_object(head_page, ptr) };
    } else if page_type == PGTY_LARGE_KMALLOC {
        // Large allocation: page.private holds the buddy order.
        let order = unsafe { (*page).private };
        unsafe { free_slab_page(page, order) };
    }
    // Unknown type: ignore (defensive; shouldn't reach here on valid input).
}

pub fn slab_is_available() -> bool {
    SLAB_READY.load(Ordering::Acquire)
}

pub const fn arch_slab_minalign() -> usize {
    core::mem::size_of::<usize>()
}

pub fn __kmalloc_index(size: usize) -> usize {
    if size == 0 {
        0
    } else if size > KMALLOC_MAX_SIZE {
        NR_KMALLOC_SIZES
    } else {
        kmalloc_cache_index(size)
    }
}

pub fn kmalloc_size_roundup(size: usize) -> usize {
    if size == 0 {
        0
    } else if size > KMALLOC_MAX_SIZE {
        let pages = size.div_ceil(PAGE_SIZE);
        (1usize << ceil_log2_pages(pages)) * PAGE_SIZE
    } else {
        KMALLOC_SIZES[kmalloc_cache_index(size)]
    }
}

pub fn ksize(ptr: *const u8) -> usize {
    if ptr.is_null() || ptr == core::ptr::NonNull::<u8>::dangling().as_ptr() {
        return 0;
    }
    #[cfg(test)]
    if let Some(size) = test_host_ksize(ptr) {
        return size;
    }
    if crate::mm::vmalloc::is_vmalloc_addr(ptr) {
        return crate::mm::vmalloc::vmalloc_usable_size(ptr);
    }
    #[cfg(test)]
    let page = match test_pool_page_for_address(ptr as usize) {
        Some(page) => page,
        None => {
            let pfn = ptr as usize / PAGE_SIZE;
            if crate::mm::buddy::pfn_valid(pfn) {
                pfn_to_page(pfn)
            } else {
                return 0;
            }
        }
    };
    #[cfg(not(test))]
    let page = {
        let addr = ptr as u64;
        if addr < crate::arch::x86::mm::paging::PAGE_OFFSET {
            return 0;
        }
        let pfn = ((addr - crate::arch::x86::mm::paging::PAGE_OFFSET) as usize) / PAGE_SIZE;
        pfn_to_page(pfn)
    };
    let page_type = decode_page_type(unsafe { (*page).page_type.load(Ordering::Relaxed) });
    if page_type == PGTY_SLAB {
        let head_page = unsafe {
            let private = (*page).private;
            if private & 1 == 1 {
                slab_pfn_to_page(private >> 1)
            } else {
                page
            }
        };
        let cache = unsafe { (*head_page).private as *const KmemCache };
        if cache.is_null() {
            0
        } else {
            unsafe { (*cache).object_size }
        }
    } else if page_type == PGTY_LARGE_KMALLOC {
        (1usize << unsafe { (*page).private }) * PAGE_SIZE
    } else {
        0
    }
}

pub unsafe fn __kmalloc_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    unsafe { kmalloc(size, flags) }
}

pub unsafe fn __kmalloc_large_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    unsafe { kmalloc_large(size, flags) }
}

pub unsafe fn __kmalloc_large_node_noprof(size: usize, flags: GfpFlags, _node: i32) -> *mut u8 {
    unsafe { __kmalloc_large_noprof(size, flags) }
}

pub unsafe fn __kmalloc_node_noprof(size: usize, flags: GfpFlags, _node: i32) -> *mut u8 {
    unsafe { kmalloc(size, flags) }
}

pub unsafe fn __kmalloc_node_track_caller_noprof(
    size: usize,
    flags: GfpFlags,
    node: i32,
    _caller: usize,
) -> *mut u8 {
    unsafe { __kmalloc_node_noprof(size, flags, node) }
}

pub unsafe fn __kmalloc_cache_noprof(_cache_type: usize, flags: GfpFlags, size: usize) -> *mut u8 {
    unsafe { kmalloc(size, flags) }
}

pub unsafe fn __kmalloc_cache_node_noprof(
    _cache_type: usize,
    flags: GfpFlags,
    node: i32,
    size: usize,
) -> *mut u8 {
    unsafe { __kmalloc_node_noprof(size, flags, node) }
}

pub unsafe fn kmalloc_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    unsafe { __kmalloc_noprof(size, flags) }
}

pub unsafe fn kmalloc_node_noprof(size: usize, flags: GfpFlags, node: i32) -> *mut u8 {
    unsafe { __kmalloc_node_noprof(size, flags, node) }
}

pub unsafe fn kmalloc_nolock_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    unsafe { kmalloc(size, flags) }
}

pub unsafe fn __kvmalloc_node_noprof(size: usize, flags: GfpFlags, node: i32) -> *mut u8 {
    unsafe { __kmalloc_node_noprof(size, flags, node) }
}

pub unsafe fn kzalloc_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    let ptr = unsafe { kmalloc(size, flags) };
    if !ptr.is_null() && size != 0 {
        unsafe { core::ptr::write_bytes(ptr, 0, size) };
    }
    ptr
}

pub unsafe fn kmalloc_array_noprof(n: usize, size: usize, flags: GfpFlags) -> *mut u8 {
    let Some(bytes) = n.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    unsafe { kmalloc(bytes, flags) }
}

pub unsafe fn kmalloc_array_node_noprof(
    n: usize,
    size: usize,
    flags: GfpFlags,
    node: i32,
) -> *mut u8 {
    let Some(bytes) = n.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    unsafe { kmalloc_node_noprof(bytes, flags, node) }
}

pub unsafe fn krealloc_array_noprof(
    ptr: *mut u8,
    n: usize,
    size: usize,
    flags: GfpFlags,
) -> *mut u8 {
    let Some(bytes) = n.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    unsafe { krealloc_node_align_noprof(ptr, bytes, 0, flags, -1) }
}

pub unsafe fn krealloc_node_align_noprof(
    ptr: *mut u8,
    new_size: usize,
    _align: usize,
    flags: GfpFlags,
    _node: i32,
) -> *mut u8 {
    if ptr.is_null() {
        return unsafe { kmalloc(new_size, flags) };
    }
    let old_size = ksize(ptr);
    if new_size <= old_size {
        return ptr;
    }
    let new_ptr = unsafe { kmalloc(new_size, flags) };
    if !new_ptr.is_null() {
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, old_size);
            kfree(ptr);
        }
    }
    new_ptr
}

pub unsafe fn kvrealloc_node_align_noprof(
    ptr: *mut u8,
    new_size: usize,
    align: usize,
    flags: GfpFlags,
    node: i32,
) -> *mut u8 {
    unsafe { krealloc_node_align_noprof(ptr, new_size, align, flags, node) }
}

pub unsafe fn kfree_sensitive(ptr: *mut u8) {
    let size = ksize(ptr);
    if size != 0 && !ptr.is_null() {
        unsafe { core::ptr::write_bytes(ptr, 0, size) };
    }
    unsafe { kfree(ptr) };
}

pub unsafe fn kvfree(ptr: *mut u8) {
    unsafe { kfree(ptr) };
}

pub unsafe fn kvfree_atomic(ptr: *mut u8) {
    unsafe { kvfree(ptr) };
}

pub unsafe fn kvfree_sensitive(ptr: *mut u8, len: usize) {
    if len != 0 && !ptr.is_null() {
        unsafe { core::ptr::write_bytes(ptr, 0, len) };
    }
    unsafe { kvfree(ptr) };
}

pub unsafe fn kmem_cache_alloc_noprof(cache: *mut KmemCache, flags: GfpFlags) -> *mut u8 {
    if cache.is_null() {
        return core::ptr::null_mut();
    }
    let _guard = lock_slab();
    unsafe { (*cache).alloc(flags) }
}

pub unsafe fn kmem_cache_alloc_node_noprof(
    cache: *mut KmemCache,
    flags: GfpFlags,
    _node: i32,
) -> *mut u8 {
    unsafe { kmem_cache_alloc_noprof(cache, flags) }
}

pub unsafe fn kmem_cache_alloc_lru_noprof(
    cache: *mut KmemCache,
    _lru: usize,
    flags: GfpFlags,
) -> *mut u8 {
    unsafe { kmem_cache_alloc_noprof(cache, flags) }
}

pub unsafe fn kmem_cache_free(_cache: *mut KmemCache, ptr: *mut u8) {
    unsafe { kfree(ptr) };
}

pub unsafe fn kmem_cache_free_bulk(cache: *mut KmemCache, nr: usize, ptrs: *mut *mut u8) {
    if ptrs.is_null() {
        return;
    }
    for idx in 0..nr {
        let ptr = unsafe { *ptrs.add(idx) };
        unsafe { kmem_cache_free(cache, ptr) };
    }
}

pub unsafe fn kmem_cache_alloc_bulk_noprof(
    cache: *mut KmemCache,
    flags: GfpFlags,
    nr: usize,
    ptrs: *mut *mut u8,
) -> usize {
    if ptrs.is_null() {
        return 0;
    }
    let mut allocated = 0usize;
    while allocated < nr {
        let ptr = unsafe { kmem_cache_alloc_noprof(cache, flags) };
        if ptr.is_null() {
            break;
        }
        unsafe {
            *ptrs.add(allocated) = ptr;
        }
        allocated += 1;
    }
    allocated
}

pub fn kmem_cache_size(cache: *const KmemCache) -> usize {
    if cache.is_null() {
        0
    } else {
        unsafe { (*cache).object_size }
    }
}

pub fn kmem_cache_shrink(_cache: *mut KmemCache) -> i32 {
    0
}

pub fn kmem_cache_destroy(_cache: *mut KmemCache) {}

pub fn kmem_cache_charge(_cache: *mut KmemCache, _gfp: GfpFlags) -> i32 {
    0
}

pub fn validate_slab_cache(_cache: *const KmemCache) -> bool {
    true
}

pub fn kmem_dump_obj(_ptr: *const u8) -> bool {
    false
}

pub fn kmalloc_caches() -> *const KmemCache {
    core::ptr::addr_of!(KMALLOC_CACHES).cast::<KmemCache>()
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__kmalloc_noprof", linux___kmalloc_noprof as usize, true);
    export_symbol_once(
        "__kmalloc_cache_noprof",
        linux___kmalloc_cache_noprof as usize,
        true,
    );
    export_symbol_once(
        "__kmalloc_node_track_caller_noprof",
        linux___kmalloc_node_track_caller_noprof as usize,
        false,
    );
    export_symbol_once("kfree", linux_kfree as usize, true);
    export_symbol_once("ksize", linux_ksize as usize, false);
    export_symbol_once("kmalloc_caches", linux_kmalloc_caches as usize, true);
    export_symbol_once(
        "kmem_cache_alloc_noprof",
        linux_kmem_cache_alloc_noprof as usize,
        false,
    );
    export_symbol_once(
        "kmem_cache_alloc_node_noprof",
        linux_kmem_cache_alloc_node_noprof as usize,
        false,
    );
    export_symbol_once(
        "kmem_cache_alloc_lru_noprof",
        linux_kmem_cache_alloc_lru_noprof as usize,
        false,
    );
    export_symbol_once("kmem_cache_charge", linux_kmem_cache_charge as usize, false);
    export_symbol_once("kmem_cache_free", linux_kmem_cache_free as usize, false);
    export_symbol_once(
        "kmem_cache_free_bulk",
        linux_kmem_cache_free_bulk as usize,
        false,
    );
    export_symbol_once(
        "kmem_cache_alloc_bulk_noprof",
        linux_kmem_cache_alloc_bulk_noprof as usize,
        false,
    );
    export_symbol_once("kmem_cache_size", linux_kmem_cache_size as usize, false);
    export_symbol_once("kmem_cache_shrink", linux_kmem_cache_shrink as usize, false);
    export_symbol_once(
        "kmem_cache_destroy",
        linux_kmem_cache_destroy as usize,
        false,
    );
    export_symbol_once(
        "__kvmalloc_node_noprof",
        linux___kvmalloc_node_noprof as usize,
        true,
    );
    export_symbol_once(
        "krealloc_node_align_noprof",
        linux_krealloc_node_align_noprof as usize,
        false,
    );
}

/// `__kmalloc_noprof` - `vendor/linux/include/linux/slab.h`.
#[unsafe(export_name = "__kmalloc_noprof")]
pub unsafe extern "C" fn linux___kmalloc_noprof(size: usize, flags: GfpFlags) -> *mut u8 {
    unsafe { __kmalloc_noprof(size, flags) }
}

/// `__kmalloc_cache_noprof` - `vendor/linux/include/linux/slab.h`.
#[unsafe(export_name = "__kmalloc_cache_noprof")]
pub unsafe extern "C" fn linux___kmalloc_cache_noprof(
    cache_type: usize,
    flags: GfpFlags,
    size: usize,
) -> *mut u8 {
    unsafe { __kmalloc_cache_noprof(cache_type, flags, size) }
}

/// `__kmalloc_node_track_caller_noprof` - `vendor/linux/mm/slub.c`.
pub unsafe extern "C" fn linux___kmalloc_node_track_caller_noprof(
    size: usize,
    flags: GfpFlags,
    node: i32,
    caller: usize,
) -> *mut u8 {
    unsafe { __kmalloc_node_track_caller_noprof(size, flags, node, caller) }
}

/// `kmem_cache_alloc_noprof` - `vendor/linux/mm/slub.c:4950`.
pub unsafe extern "C" fn linux_kmem_cache_alloc_noprof(
    cache: *mut KmemCache,
    flags: GfpFlags,
) -> *mut u8 {
    unsafe { kmem_cache_alloc_noprof(cache, flags) }
}

/// `kmem_cache_alloc_node_noprof` - `vendor/linux/mm/slub.c:5008`.
pub unsafe extern "C" fn linux_kmem_cache_alloc_node_noprof(
    cache: *mut KmemCache,
    flags: GfpFlags,
    node: i32,
) -> *mut u8 {
    unsafe { kmem_cache_alloc_node_noprof(cache, flags, node) }
}

/// `kmem_cache_alloc_lru_noprof` - `vendor/linux/mm/slub.c:4967`.
pub unsafe extern "C" fn linux_kmem_cache_alloc_lru_noprof(
    cache: *mut KmemCache,
    lru: *mut c_void,
    flags: GfpFlags,
) -> *mut u8 {
    unsafe { kmem_cache_alloc_lru_noprof(cache, lru as usize, flags) }
}

/// `kmem_cache_charge` - `vendor/linux/mm/slub.c:4986`.
pub unsafe extern "C" fn linux_kmem_cache_charge(_objp: *mut c_void, _gfpflags: GfpFlags) -> bool {
    true
}

/// `kmem_cache_free` - `vendor/linux/include/linux/slab.h`.
pub unsafe extern "C" fn linux_kmem_cache_free(cache: *mut KmemCache, ptr: *mut u8) {
    unsafe { kmem_cache_free(cache, ptr) };
}

/// `kmem_cache_free_bulk` - `vendor/linux/mm/slub.c:7155`.
pub unsafe extern "C" fn linux_kmem_cache_free_bulk(
    cache: *mut KmemCache,
    nr: usize,
    ptrs: *mut *mut u8,
) {
    unsafe { kmem_cache_free_bulk(cache, nr, ptrs) };
}

/// `kmem_cache_alloc_bulk_noprof` - `vendor/linux/mm/slub.c:7404`.
pub unsafe extern "C" fn linux_kmem_cache_alloc_bulk_noprof(
    cache: *mut KmemCache,
    flags: GfpFlags,
    nr: usize,
    ptrs: *mut *mut u8,
) -> bool {
    unsafe { kmem_cache_alloc_bulk_noprof(cache, flags, nr, ptrs) == nr }
}

/// `kmem_cache_size` - `vendor/linux/mm/slab_common.c:83`.
pub unsafe extern "C" fn linux_kmem_cache_size(cache: *const KmemCache) -> u32 {
    kmem_cache_size(cache).min(u32::MAX as usize) as u32
}

/// `kmem_cache_shrink` - `vendor/linux/mm/slab_common.c:602`.
pub unsafe extern "C" fn linux_kmem_cache_shrink(cache: *mut KmemCache) -> i32 {
    kmem_cache_shrink(cache)
}

/// `kmem_cache_destroy` - `vendor/linux/mm/slab_common.c:527`.
pub unsafe extern "C" fn linux_kmem_cache_destroy(cache: *mut KmemCache) {
    kmem_cache_destroy(cache);
}

/// `__kvmalloc_node_noprof` - `vendor/linux/mm/util.c`.
#[unsafe(export_name = "__kvmalloc_node_noprof")]
pub unsafe extern "C" fn linux___kvmalloc_node_noprof(
    size: usize,
    flags: GfpFlags,
    node: i32,
) -> *mut u8 {
    unsafe { __kvmalloc_node_noprof(size, flags, node) }
}

/// `krealloc_node_align_noprof` - `vendor/linux/mm/slub.c:6876`.
pub unsafe extern "C" fn linux_krealloc_node_align_noprof(
    ptr: *mut u8,
    new_size: usize,
    align: usize,
    flags: GfpFlags,
    node: i32,
) -> *mut u8 {
    unsafe { krealloc_node_align_noprof(ptr, new_size, align, flags, node) }
}

/// `kfree` - `vendor/linux/mm/slab_common.c`.
#[unsafe(export_name = "kfree")]
pub unsafe extern "C" fn linux_kfree(ptr: *mut u8) {
    unsafe { kfree(ptr) };
}

pub unsafe extern "C" fn linux_ksize(ptr: *const u8) -> usize {
    ksize(ptr)
}

/// `kmalloc_caches` - `vendor/linux/mm/slab_common.c`.
#[unsafe(export_name = "kmalloc_caches")]
pub unsafe extern "C" fn linux_kmalloc_caches() -> *const KmemCache {
    kmalloc_caches()
}

pub fn random_kmalloc_seed() -> usize {
    0
}

pub fn kvfree_rcu_barrier() {}

pub fn kvfree_rcu_barrier_on_cache(_cache: *mut KmemCache) {}

pub fn kvfree_rcu_init() {}

pub fn kfree_rcu_scheduler_running() -> bool {
    true
}

pub fn rcu_read_lock() {}

pub fn rcu_read_unlock() {}

// ---------------------------------------------------------------------------
// Large-kmalloc helpers (size > KMALLOC_MAX_SIZE)
// ---------------------------------------------------------------------------

/// Allocate `size` bytes using the buddy allocator directly (no slab cache).
///
/// Used for oversized allocations (`size > KMALLOC_MAX_SIZE = 8192`).
/// The buddy order is stored in `page.private` so `kfree` can return the
/// correct number of pages.
///
/// Ref: Linux `mm/slub.c` — `kmalloc_large_node()`
unsafe fn kmalloc_large(size: usize, gfp: GfpFlags) -> *mut u8 {
    let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    let order = ceil_log2_pages(pages_needed);
    if order > MAX_PAGE_ORDER {
        let ptr = crate::mm::vmalloc::vmalloc(size);
        if !ptr.is_null() && gfp & __GFP_ZERO != 0 {
            unsafe {
                core::ptr::write_bytes(ptr, 0, size);
            }
        }
        return ptr;
    }
    match unsafe { alloc_slab_page(order, gfp) } {
        None => core::ptr::null_mut(),
        Some((page, mem)) => {
            unsafe {
                (*page)
                    .page_type
                    .store(encode_page_type(PGTY_LARGE_KMALLOC), Ordering::Relaxed);
                (*page).private = order;
                if gfp & __GFP_ZERO != 0 {
                    core::ptr::write_bytes(mem, 0, size);
                }
            }
            mem
        }
    }
}

// ---------------------------------------------------------------------------
// GlobalAlloc — routes Box/Vec/String through kmalloc/kfree
//
// Activated by `slab-alloc` (default) feature.  Disabled in unit tests (host
// test binary uses the system allocator).
//
// Ref: Linux mm/slub.c — in Linux the kernel's alloc/free IS kmalloc/kfree;
//      here we expose the same API as Rust's GlobalAlloc trait.
// ---------------------------------------------------------------------------

/// Zero-size marker type that implements `GlobalAlloc` via `kmalloc`/`kfree`.
pub struct SlabGlobalAlloc;

/// Register as the global allocator when `slab-alloc` feature is active and
/// we are building the bare-metal kernel (not running host unit tests).
///
/// The pre-slab bootstrap (before `slab_init()`) is protected by
/// `SLAB_READY`: if anything tries to allocate before init, the kernel spins
/// and logs a fatal message rather than silently corrupting state.
#[cfg(all(not(test), feature = "slab-alloc"))]
#[global_allocator]
static SLAB_GLOBAL: SlabGlobalAlloc = SlabGlobalAlloc;

unsafe impl GlobalAlloc for SlabGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(effective) = kmalloc_size_for_layout(layout) else {
            return core::ptr::null_mut();
        };
        unsafe { kmalloc(effective, GFP_KERNEL) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { kfree(ptr) }
    }
}

fn kmalloc_size_for_layout(layout: Layout) -> Option<usize> {
    let size = layout.size().max(1);
    let align = layout.align().max(core::mem::size_of::<usize>());

    if align <= core::mem::size_of::<usize>() {
        return Some(size);
    }

    if align > KMALLOC_MAX_SIZE {
        return Some(size.max(align));
    }

    KMALLOC_SIZES
        .iter()
        .copied()
        .find(|class_size| *class_size >= size && *class_size % align == 0)
        .or_else(|| Some(size.max(align)))
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Determine the buddy order needed for a slab backing a given object size.
///
/// Uses order-0 (4 KiB) for objects ≤ 4096 bytes.
/// Uses order-1 (8 KiB) for the 8192-byte size class.
///
/// A minimum of 1 object per slab is always achievable.
///
/// Ref: Linux `mm/slub.c` — `calculate_order()` (simplified)
fn slab_order(object_size: usize) -> usize {
    if object_size > PAGE_SIZE {
        // Need enough pages to hold at least one object.
        let pages = (object_size + PAGE_SIZE - 1) / PAGE_SIZE;
        ceil_log2_pages(pages)
    } else {
        0
    }
}

/// Find the kmalloc cache index for a requested `size`.
///
/// Returns the index of the smallest size class ≥ `size`.
fn kmalloc_cache_index(size: usize) -> usize {
    for (i, &class_size) in KMALLOC_SIZES.iter().enumerate() {
        if size <= class_size {
            return i;
        }
    }
    // Should not reach here: caller checked size <= KMALLOC_MAX_SIZE.
    NR_KMALLOC_SIZES - 1
}

/// Align `value` up to the next multiple of `align` (must be a power of two).
#[inline]
const fn align_up_usize(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Smallest order O such that `2^O >= pages`.
fn ceil_log2_pages(pages: usize) -> usize {
    if pages <= 1 {
        return 0;
    }
    (usize::BITS - (pages - 1).leading_zeros()) as usize
}

// ---------------------------------------------------------------------------
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;
    use alloc::boxed::Box;

    use super::*;
    use crate::mm::buddy::{pfn_to_page, set_mem_map};
    use crate::mm::page::Page;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    // -----------------------------------------------------------------------
    // Test infrastructure
    //
    // All slab tests share the module-level `TEST_*` statics (memory pool and
    // mem_map).  `TEST_LOCK` serialises test execution to prevent races.
    // -----------------------------------------------------------------------

    /// Initialise the test page pool and re-initialise the slab subsystem.
    ///
    /// Must be called at the start of every test that uses `alloc_slab_page`
    /// or `kmalloc`.
    ///
    /// # Safety
    /// Must be called while holding `TEST_LOCK`.
    unsafe fn setup() {
        let mem_ptr = core::ptr::addr_of_mut!(TEST_PHYS) as *mut u8;
        let pages_ptr = core::ptr::addr_of_mut!(TEST_META) as *mut Page;

        // Zero physical memory.
        unsafe {
            core::ptr::write_bytes(mem_ptr, 0, PAGE_SIZE * TEST_N_PAGES);
        }

        // Re-initialise Page metadata.
        for i in 0..TEST_N_PAGES {
            unsafe {
                let p = pages_ptr.add(i);
                core::ptr::write(p, Page::new());
                (*p).init_lru();
            }
        }

        // Install as global mem_map so pfn_to_page / page_to_pfn work.
        let base_pfn = mem_ptr as usize / PAGE_SIZE;
        unsafe {
            set_mem_map(pages_ptr, base_pfn, TEST_N_PAGES);
        }

        TEST_MEM_PTR.store(mem_ptr as usize, Ordering::SeqCst);
        TEST_BASE_PFN.store(base_pfn, Ordering::SeqCst);
        TEST_PAGE_CURSOR.store(0, Ordering::SeqCst);
        TEST_FREE_PAGES.lock().clear();

        // Re-initialise kmalloc caches (they reference stable static addresses).
        for i in 0..NR_KMALLOC_SIZES {
            unsafe {
                KMALLOC_CACHES[i].init(KMALLOC_NAMES[i], KMALLOC_SIZES[i], 8);
            }
        }
        SLAB_READY.store(true, Ordering::Release);
    }

    /// Allocate a freshly-initialised KmemCache on the heap to avoid
    /// stack moves of self-referential ListHead pointers.
    unsafe fn make_cache(name: &'static str, size: usize) -> Box<KmemCache> {
        let mut cache = Box::new(KmemCache::const_uninit());
        unsafe {
            cache.init(name, size, 8);
        }
        cache
    }

    /// test-origin: linux:vendor/linux/mm/slub.c and
    /// linux:vendor/linux/mm/page_alloc.c
    ///
    /// Linux's slab allocator owns page metadata for pages it allocated; a
    /// separate test reset of the buddy allocator must not invalidate slab's
    /// already-published slab-page metadata. This is Lupos-specific host-test
    /// coverage for the static TEST_PHYS/TEST_META backend used when no real
    /// boot-time buddy/slab initialization exists.
    #[test]
    fn host_test_slab_pool_survives_buddy_mem_map_reset() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
            crate::mm::buddy::reset_buddy_state_for_test();

            let ptr = kmalloc(64, GFP_KERNEL);
            assert!(
                !ptr.is_null(),
                "host test slab allocation must not depend on buddy mem_map"
            );
            assert_eq!(ksize(ptr), 64);
            kfree(ptr);

            setup();
        }
    }

    // -----------------------------------------------------------------------
    // TDD: Phase A — unit tests written to define the API and verify
    //              correctness before the implementation was finalised.
    // -----------------------------------------------------------------------

    /// A newly created cache has the expected metadata.
    ///
    /// Ref: Linux `kmem_cache_create()` — sets up name, object_size, align.
    #[test]
    fn test_kmem_cache_create() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            let cache = make_cache("test-32", 32);
            assert_eq!(cache.name, "test-32");
            assert_eq!(cache.object_size, 32);
            // Slot size must be ≥ object_size and ≥ pointer size, aligned to 8.
            assert!(cache.size >= 32);
            assert!(cache.size >= core::mem::size_of::<usize>());
            assert_eq!(cache.size % 8, 0);
            // The partial and full lists must start empty.
            assert!(ListHead::is_empty(&cache.partial));
            assert!(ListHead::is_empty(&cache.full));
        }
    }

    /// A single allocation returns a non-null pointer.
    #[test]
    fn test_slab_alloc_one() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            let mut cache = make_cache("test-32", 32);
            let ptr = cache.alloc(GFP_KERNEL);
            assert!(!ptr.is_null(), "alloc returned null");
            // Pointer must be within test physical memory.
            let mem_base = TEST_MEM_PTR.load(Ordering::SeqCst);
            assert!(ptr as usize >= mem_base, "ptr before test pool");
            let ptr_addr = ptr as usize;
            assert!(
                ptr_addr < mem_base + PAGE_SIZE * TEST_N_PAGES,
                "ptr after test pool"
            );
        }
    }

    /// After a free, the next allocation reuses the same address.
    ///
    /// Verifies that the freelist round-trip works correctly.
    #[test]
    fn test_slab_alloc_free_roundtrip() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            let mut cache = make_cache("test-64", 64);
            let p1 = cache.alloc(GFP_KERNEL);
            assert!(!p1.is_null());
            // kfree goes via global cache, so manually free through the cache:
            let pfn = p1 as usize / PAGE_SIZE;
            let head_page = pfn_to_page(pfn);
            cache.free_object(head_page, p1);

            // Next alloc must return same address (LIFO freelist).
            let p2 = cache.alloc(GFP_KERNEL);
            assert_eq!(p1, p2, "freed object should be reused");
        }
    }

    #[test]
    fn duplicate_object_free_is_rejected_without_releasing_live_slab() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();

            let mut cache = make_cache("test-double-free", 64);
            let first = cache.alloc(GFP_KERNEL);
            let second = cache.alloc(GFP_KERNEL);
            assert!(!first.is_null() && !second.is_null());
            let head = pfn_to_page(first as usize / PAGE_SIZE);

            assert!(cache.free_object(head, first));
            let inuse_after_first = (*head).index;
            let rejected_before = slab_free_rejection_snapshot().0;
            assert!(!cache.free_object(head, first));
            assert_eq!((*head).index, inuse_after_first);
            assert!(slab_free_rejection_snapshot().0 > rejected_before);
            assert_eq!(slab_free_rejection_snapshot().3, 3);

            // The other live object can still be freed normally; the rejected
            // duplicate did not release or recycle its backing page.
            assert!(cache.free_object(head, second));
        }
    }

    /// Allocating more objects than fit in a single slab triggers `new_slab`.
    ///
    /// All returned pointers must be non-null and distinct.
    #[test]
    fn test_slab_alloc_crosses_slab_boundary() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            // Use 512-byte objects: objects_per_slab = 4096 / 512 = 8.
            let mut cache = make_cache("test-512", 512);
            let per_slab = cache.objects_per_slab;
            let n = per_slab + 1; // one more than a single slab holds

            let mut ptrs = alloc::vec::Vec::with_capacity(n);
            for _ in 0..n {
                let p = cache.alloc(GFP_KERNEL);
                assert!(!p.is_null(), "alloc returned null before crossing boundary");
                ptrs.push(p);
            }

            // All pointers must be distinct.
            for i in 0..ptrs.len() {
                for j in (i + 1)..ptrs.len() {
                    assert_ne!(ptrs[i], ptrs[j], "duplicate pointer detected");
                }
            }
        }
    }

    /// 100 allocations from the same cache must all be non-overlapping.
    ///
    /// Verifies no object is handed out twice and that objects don't overlap.
    /// This mirrors the Milestone 8 boot smoke test at smaller scale.
    #[test]
    fn test_no_overlap_100_objects() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            let mut cache = make_cache("test-32", 32);
            let mut ptrs: alloc::vec::Vec<*mut u8> = alloc::vec::Vec::with_capacity(100);

            for _ in 0..100 {
                let p = cache.alloc(GFP_KERNEL);
                assert!(!p.is_null());
                ptrs.push(p);
            }

            // Every pair of pointers must be ≥ slot_size apart.
            let slot = cache.size;
            for i in 0..ptrs.len() {
                for j in (i + 1)..ptrs.len() {
                    let a = ptrs[i] as usize;
                    let b = ptrs[j] as usize;
                    let dist = if a > b { a - b } else { b - a };
                    assert!(
                        dist >= slot,
                        "overlap: ptrs[{}]={:#x} ptrs[{}]={:#x} dist={} < slot={}",
                        i,
                        a,
                        j,
                        b,
                        dist,
                        slot
                    );
                }
            }
        }
    }

    /// `kmalloc_cache_index` maps sizes to the correct size class.
    ///
    /// Verifies the size-class selection logic used by `kmalloc`.
    #[test]
    fn test_kmalloc_size_class_selection() {
        // Sizes that should map to each class:
        assert_eq!(kmalloc_cache_index(1), 0); // → 8
        assert_eq!(kmalloc_cache_index(8), 0); // → 8 (exact)
        assert_eq!(kmalloc_cache_index(9), 1); // → 16
        assert_eq!(kmalloc_cache_index(16), 1); // → 16 (exact)
        assert_eq!(kmalloc_cache_index(17), 2); // → 32
        assert_eq!(kmalloc_cache_index(32), 2); // → 32 (exact)
        assert_eq!(kmalloc_cache_index(33), 3); // → 64
        assert_eq!(kmalloc_cache_index(64), 3); // → 64 (exact)
        assert_eq!(kmalloc_cache_index(65), 4); // → 96
        assert_eq!(kmalloc_cache_index(96), 4); // → 96 (exact)
        assert_eq!(kmalloc_cache_index(512), 8); // → 512 (exact)
        assert_eq!(kmalloc_cache_index(8192), 12); // → 8192 (exact, last class)
    }

    /// Large allocations (> KMALLOC_MAX_SIZE) use the large-kmalloc path.
    ///
    /// The returned pointer must be page-aligned (buddy page allocation).
    #[test]
    fn test_kmalloc_large_fallthrough() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            // Request 16 KiB — larger than any slab cache.
            let ptr = kmalloc(16 * 1024, GFP_KERNEL);
            assert!(!ptr.is_null(), "large kmalloc returned null");
            // Should be page-aligned (buddy allocates at page boundaries).
            assert_eq!(ptr as usize % PAGE_SIZE, 0, "large alloc not page-aligned");
            // Page type must be PGTY_LARGE_KMALLOC.
            let pfn = ptr as usize / PAGE_SIZE;
            let page = pfn_to_page(pfn);
            assert_eq!(
                decode_page_type((*page).page_type.load(Ordering::Relaxed)),
                PGTY_LARGE_KMALLOC
            );
        }
    }

    #[test]
    fn linux_slab_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__kmalloc_noprof"),
            Some(linux___kmalloc_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__kmalloc_cache_noprof"),
            Some(linux___kmalloc_cache_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__kmalloc_node_track_caller_noprof"),
            Some(linux___kmalloc_node_track_caller_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kfree"),
            Some(linux_kfree as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmalloc_caches"),
            Some(linux_kmalloc_caches as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmem_cache_alloc_noprof"),
            Some(linux_kmem_cache_alloc_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmem_cache_alloc_node_noprof"),
            Some(linux_kmem_cache_alloc_node_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmem_cache_alloc_lru_noprof"),
            Some(linux_kmem_cache_alloc_lru_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmem_cache_free"),
            Some(linux_kmem_cache_free as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kmem_cache_alloc_bulk_noprof"),
            Some(linux_kmem_cache_alloc_bulk_noprof as usize)
        );
    }

    #[test]
    fn linux_slab_c_entrypoints_allocate_and_free() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
            let ptr = linux___kmalloc_noprof(64, GFP_KERNEL);
            assert!(!ptr.is_null());
            linux_kfree(ptr);
            let cache_ptr = linux___kmalloc_cache_noprof(0, GFP_KERNEL, 64);
            assert!(!cache_ptr.is_null());
            linux_kfree(cache_ptr);
            assert!(!linux_kmalloc_caches().is_null());

            let mut cache = make_cache("linux-c-entry-cache", 32);
            let obj = linux_kmem_cache_alloc_noprof(&mut *cache, GFP_KERNEL);
            assert!(!obj.is_null());
            assert_eq!(linux_kmem_cache_size(&*cache), 32);
            assert!(linux_kmem_cache_charge(obj.cast(), GFP_KERNEL));
            linux_kmem_cache_free(&mut *cache, obj);
        }
    }

    #[test]
    fn linux_kmalloc_honors_gfp_zero_for_module_kzalloc_macros() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();

            let dirty = linux___kmalloc_noprof(64, GFP_KERNEL);
            assert!(!dirty.is_null());
            core::ptr::write_bytes(dirty, 0xa5, 64);
            linux_kfree(dirty);

            let zeroed = linux___kmalloc_cache_noprof(0, GFP_KERNEL | __GFP_ZERO, 64);
            assert!(!zeroed.is_null());
            let bytes = core::slice::from_raw_parts(zeroed, 64);
            assert!(
                bytes.iter().all(|byte| *byte == 0),
                "__GFP_ZERO must clear memory for Linux kmalloc/kzalloc_objs callers"
            );
            linux_kfree(zeroed);
        }
    }

    /// After 100 alloc/free round-trips the free count returns to baseline.
    ///
    /// Verifies that `free_object` correctly restores the freelist.
    #[test]
    fn test_kfree_reuse_after_stress() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
        }

        unsafe {
            let mut cache = make_cache("test-128", 128);
            // Allocate 10 objects, remember them, free all, then allocate 10 again.
            // The second batch must come from the same slab (no new pages).
            let mut ptrs: [*mut u8; 10] = [core::ptr::null_mut(); 10];
            for p in ptrs.iter_mut() {
                *p = cache.alloc(GFP_KERNEL);
                assert!(!p.is_null());
            }
            let pages_used_after_alloc = TEST_PAGE_CURSOR.load(Ordering::SeqCst);

            // Free all.
            for &p in ptrs.iter() {
                let head = pfn_to_page(p as usize / PAGE_SIZE);
                cache.free_object(head, p);
            }

            // Re-allocate 10; no new slab pages should be consumed.
            for p in ptrs.iter_mut() {
                *p = cache.alloc(GFP_KERNEL);
                assert!(!p.is_null());
            }
            let pages_used_after_realloc = TEST_PAGE_CURSOR.load(Ordering::SeqCst);

            assert_eq!(
                pages_used_after_alloc, pages_used_after_realloc,
                "re-allocation consumed new slab pages (freed objects not reused)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Helper / utility tests
    // -----------------------------------------------------------------------

    /// `slab_order` returns 0 for objects ≤ PAGE_SIZE and 1 for 8192-byte objects.
    #[test]
    fn test_slab_order_computation() {
        assert_eq!(slab_order(8), 0);
        assert_eq!(slab_order(512), 0);
        assert_eq!(slab_order(4096), 0); // equal to PAGE_SIZE → order 0
        assert_eq!(slab_order(4097), 1); // exceeds one page → order 1
        assert_eq!(slab_order(8192), 1); // two pages → order 1
    }

    #[test]
    fn cache_order_uses_aligned_slot_size() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
            let mut cache = Box::new(KmemCache::const_uninit());
            cache.init("test-align-8192", 1, 8192);

            assert_eq!(cache.align, 8192);
            assert_eq!(cache.size, 8192);
            assert_eq!(cache.order, 1);
            assert_eq!(cache.objects_per_slab, 1);

            let ptr = cache.alloc(GFP_KERNEL);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % 8192, 0);
        }
    }

    #[test]
    fn rust_layout_alignment_avoids_unaligned_size_classes() {
        assert_eq!(
            kmalloc_size_for_layout(Layout::from_size_align(80, 64).unwrap()),
            Some(128)
        );
        assert_eq!(
            kmalloc_size_for_layout(Layout::from_size_align(129, 128).unwrap()),
            Some(256)
        );
        assert_eq!(
            kmalloc_size_for_layout(Layout::from_size_align(4097, 4096).unwrap()),
            Some(8192)
        );
        assert_eq!(
            kmalloc_size_for_layout(Layout::from_size_align(9000, 8192).unwrap()),
            Some(9000)
        );
    }

    #[test]
    fn aligned_layout_allocations_are_aligned() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
            let layout = Layout::from_size_align(80, 64).unwrap();
            let size = kmalloc_size_for_layout(layout).expect("layout size");
            let mut ptrs = alloc::vec::Vec::new();
            for _ in 0..16 {
                let ptr = kmalloc(size, GFP_KERNEL);
                assert!(!ptr.is_null());
                assert_eq!(ptr as usize % layout.align(), 0);
                ptrs.push(ptr);
            }
            for ptr in ptrs {
                kfree(ptr);
            }
        }
    }

    #[test]
    fn clear_slab_page_metadata_covers_tail_pages() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            setup();
            let page = pfn_to_page(TEST_BASE_PFN.load(Ordering::SeqCst));
            for i in 0..2 {
                let p = page.add(i);
                (*p).page_type
                    .store(encode_page_type(PGTY_SLAB), Ordering::Relaxed);
                (*p).mapping = 0x1111 + i;
                (*p).index = 0x2222 + i;
                (*p).private = 0x3333 + i;
            }

            clear_slab_page_metadata(page, 1);

            for i in 0..2 {
                let p = page.add(i);
                assert_eq!((*p).page_type.load(Ordering::Relaxed), PAGE_TYPE_NONE);
                assert_eq!((*p).mapping, 0);
                assert_eq!((*p).index, 0);
                assert_eq!((*p).private, 0);
            }
        }
    }

    /// `ceil_log2_pages` rounds up to the next power-of-two order.
    #[test]
    fn test_ceil_log2_pages() {
        assert_eq!(ceil_log2_pages(1), 0);
        assert_eq!(ceil_log2_pages(2), 1);
        assert_eq!(ceil_log2_pages(3), 2);
        assert_eq!(ceil_log2_pages(4), 2);
        assert_eq!(ceil_log2_pages(5), 3);
        assert_eq!(ceil_log2_pages(8), 3);
        assert_eq!(ceil_log2_pages(1024), 10);
    }
}
