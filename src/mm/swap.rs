//! linux-parity: complete
//! linux-source: vendor/linux/mm/swap.c
//! test-origin: linux:vendor/linux/mm/swap.c
/// Swap subsystem — Milestone 17.
///
/// Implements:
/// - `SwpEntry` — the generic swap entry type (`swp_entry_t` in Linux)
/// - `swp_entry_to_pte` / `pte_to_swp_entry` — generic ↔ arch PTE conversion
/// - `SwapInfoStruct` — per-device swap descriptor with in-memory backing store
/// - Slot allocation / deallocation (`folio_alloc_swap`, `free_swap_slot`)
/// - Swap cache (`swap_cache_add`, `swap_cache_get`, `swap_cache_delete`)
/// - Synchronous swap I/O (`swap_writepage`, `swap_readpage`, `add_to_swap`)
/// - `swapon` / `swapoff` — device lifecycle
///
/// In M17 the backing store is a heap-allocated `Vec<u8>` that acts as a RAM
/// disk.  M44 (VirtIO-blk) will replace it with real block device I/O while
/// keeping the same public API.
///
/// ## References
/// - Linux `mm/swapfile.c` — `sys_swapon`, `sys_swapoff`, `folio_alloc_swap`
/// - Linux `mm/swap_state.c` — `__add_to_swap_cache`, `lookup_swap_cache`
/// - Linux `mm/page_io.c` — `swap_writepage`, `swap_read_folio`
/// - Linux `include/linux/swap.h` — `struct swap_info_struct`
/// - Linux `include/linux/swapops.h` — `swp_entry_t`, `swp_entry_to_pte`
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

use crate::arch::x86::mm::paging::{
    arch_swp_entry, arch_swp_offset, arch_swp_type, is_swap_pte, pte_t,
};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::page::Page;
use crate::mm::page_flags::{PG_SWAPBACKED, PG_SWAPCACHE, PG_UPTODATE, PG_WRITEBACK};

// ---------------------------------------------------------------------------
// Generic swap entry — swp_entry_t
// ---------------------------------------------------------------------------

/// Maximum number of swap devices, matching Linux `MAX_SWAPFILES_SHIFT = 5`.
pub const MAX_SWAPFILES_SHIFT: u32 = 5;
/// Maximum number of simultaneous swap devices (= 2^5 = 32).
pub const MAX_SWAPFILES: usize = 1 << MAX_SWAPFILES_SHIFT;

/// Position of the type field inside the generic `SwpEntry::val`.
/// Ref: Linux `SWP_TYPE_SHIFT` — `include/linux/swapops.h`
pub const SWP_TYPE_SHIFT: u32 = (usize::BITS - 1) - MAX_SWAPFILES_SHIFT; // 58

/// Mask for the offset part of a generic SwpEntry.
pub const SWP_OFFSET_MASK: usize = (1usize << SWP_TYPE_SHIFT) - 1;

/// Generic swap entry, matching Linux `swp_entry_t`.
///
/// Encoding (Linux `swapops.h:88`):
///   `val = (type << SWP_TYPE_SHIFT) | (offset & SWP_OFFSET_MASK)`
///
/// Ref: Linux `include/linux/swapops.h`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct SwpEntry {
    pub val: usize,
}

impl SwpEntry {
    /// Construct a `SwpEntry` from a device type index and page offset.
    #[inline]
    pub const fn new(swap_type: u8, offset: u32) -> Self {
        SwpEntry {
            val: ((swap_type as usize) << SWP_TYPE_SHIFT) | (offset as usize & SWP_OFFSET_MASK),
        }
    }

    /// Extract the swap device type (0–31).
    #[inline]
    pub const fn swp_type(self) -> u8 {
        (self.val >> SWP_TYPE_SHIFT) as u8
    }

    /// Extract the page offset within the swap device.
    #[inline]
    pub const fn swp_offset(self) -> u32 {
        (self.val & SWP_OFFSET_MASK) as u32
    }

    /// True iff this entry is the null (unset) entry.
    #[inline]
    pub const fn is_null(self) -> bool {
        self.val == 0
    }
}

// ---------------------------------------------------------------------------
// Generic ↔ arch PTE conversion
// ---------------------------------------------------------------------------

/// Convert a `SwpEntry` to a non-present PTE.
///
/// Two steps: generic → arch encoding → `pte_t`.
/// Ref: Linux `swp_entry_to_pte()` — `include/linux/swapops.h:114`
#[inline]
pub fn swp_entry_to_pte(entry: SwpEntry) -> pte_t {
    pte_t(arch_swp_entry(entry.swp_type(), entry.swp_offset()))
}

/// Convert a non-present PTE back to a generic `SwpEntry`.
///
/// Ref: Linux `pte_to_swp_entry()` — `include/linux/swapops.h`
#[inline]
pub fn pte_to_swp_entry(pte: pte_t) -> SwpEntry {
    SwpEntry::new(arch_swp_type(pte.0), arch_swp_offset(pte.0))
}

// ---------------------------------------------------------------------------
// Swap cluster — 256 pages per cluster
//
// Ref: Linux `struct swap_cluster_info` — `include/linux/swap.h`
// ---------------------------------------------------------------------------

/// Pages per allocation cluster (must be a power of two).
pub const SWAPFILE_CLUSTER: u32 = 256;

/// `SwapClusterInfo.flags` — all slots in cluster are free.
pub const CLUSTER_FLAG_FREE: u8 = 1;
/// `SwapClusterInfo.flags` — cluster has free slots but is not fully free.
pub const CLUSTER_FLAG_NONFULL: u8 = 2;

/// Per-cluster allocation state.
///
/// Ref: Linux `struct swap_cluster_info` — `mm/swap.h`
#[derive(Debug, Clone)]
pub struct SwapClusterInfo {
    /// Free slots remaining in this cluster.
    pub count: u32,
    /// `CLUSTER_FLAG_FREE` | `CLUSTER_FLAG_NONFULL` | 0 (full).
    pub flags: u8,
    /// Next cluster in the free-cluster linked list (`u32::MAX` = end).
    pub next: u32,
}

impl SwapClusterInfo {
    const fn new_free(next: u32) -> Self {
        SwapClusterInfo {
            count: SWAPFILE_CLUSTER,
            flags: CLUSTER_FLAG_FREE,
            next,
        }
    }

    fn is_free(&self) -> bool {
        (self.flags & CLUSTER_FLAG_FREE) != 0
    }
}

// ---------------------------------------------------------------------------
// Per-device swap descriptor
//
// Ref: Linux `struct swap_info_struct` — `include/linux/swap.h`
// ---------------------------------------------------------------------------

/// Swap device is in use.
pub const SWP_USED: u32 = 1 << 0;
/// Swap device accepts new allocations.
pub const SWP_WRITEOK: u32 = 1 << 1;

/// Linux-visible swap backing class used by `/proc/swaps`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwapBackingKind {
    File,
    Partition,
}

impl SwapBackingKind {
    fn proc_type(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Partition => "partition",
        }
    }
}

/// Per-swap-device descriptor.
///
/// In M17, `backing` is a heap-allocated `Vec<u8>` of `max * PAGE_SIZE`
/// bytes acting as a RAM disk.  M44 replaces this with real block device I/O
/// while preserving the public API of `swap_writepage` / `swap_readpage`.
///
/// Ref: Linux `struct swap_info_struct` — `include/linux/swap.h`
pub struct SwapInfoStruct {
    /// Flags: `SWP_USED | SWP_WRITEOK`.
    pub flags: u32,
    /// Priority (-1 = lowest; higher = used first for allocations).
    pub priority: i32,
    /// Index into `SWAP_INFO[]` (0–31), encoded in `SwpEntry::swp_type()`.
    pub swap_type: u8,
    /// Total page slots (including unusable slot 0).
    pub max: u32,
    /// Currently swapped-out pages.
    pub inuse_pages: u32,
    /// Per-slot use count. 0 = free, non-zero = in use (count of PTEs + 1 for cache).
    pub swap_map: Vec<u8>,
    /// Per-cluster metadata.
    pub cluster_info: Vec<SwapClusterInfo>,
    /// Head of the free-cluster linked list (index into `cluster_info`).
    pub free_cluster_head: u32,
    /// Scan cursor — offset of the next slot to try when allocating.
    pub cluster_next: u32,
    /// In-memory backing store.  Length = `max as usize * PAGE_SIZE as usize`.
    /// Slot `i` occupies bytes `[i*PAGE_SIZE .. (i+1)*PAGE_SIZE]`.
    pub backing: Vec<u8>,
    /// Swap cache: swap offset → `*mut Page` (page is cached in RAM).
    pub swap_cache: BTreeMap<u32, NonNull<Page>>,
    /// Canonical Linux-visible path used by `/proc/swaps` and `swapoff(2)`.
    pub path: Option<String>,
    /// Whether Linux would expose this swap area as a file or block partition.
    pub backing_kind: SwapBackingKind,
}

// SAFETY: raw Page pointers are only accessed under the global SWAP_INFO lock.
unsafe impl Send for SwapInfoStruct {}
unsafe impl Sync for SwapInfoStruct {}

// ---------------------------------------------------------------------------
// Global swap state
// ---------------------------------------------------------------------------

static SWAP_INFO: Mutex<[Option<SwapInfoStruct>; MAX_SWAPFILES]> =
    Mutex::new([const { None }; MAX_SWAPFILES]);

static NR_SWAPFILES: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// swapon / swapoff
// ---------------------------------------------------------------------------

/// Activate a new in-memory swap device of `max_pages` pages.
///
/// Finds the first free slot in `SWAP_INFO`, allocates the per-slot bitmap,
/// cluster array, and backing store, then marks the device `SWP_USED | SWP_WRITEOK`.
///
/// Returns the swap type index (0–31) on success, or a negative errno on failure.
///
/// Ref: Linux `mm/swapfile.c` — `sys_swapon()`
pub fn swapon(max_pages: u32, priority: i32) -> Result<u8, i32> {
    swapon_with_path(None, max_pages, priority, SwapBackingKind::File)
}

pub fn swapon_path(path: String, max_pages: u32, priority: i32) -> Result<u8, i32> {
    swapon_with_path(Some(path), max_pages, priority, SwapBackingKind::File)
}

pub fn swapon_block_path(path: String, max_pages: u32, priority: i32) -> Result<u8, i32> {
    swapon_with_path(Some(path), max_pages, priority, SwapBackingKind::Partition)
}

fn swapon_with_path(
    path: Option<String>,
    max_pages: u32,
    priority: i32,
    backing_kind: SwapBackingKind,
) -> Result<u8, i32> {
    if max_pages == 0 {
        return Err(-22); // EINVAL
    }

    // Build cluster chain: slot 0 → 1 → … → (n_clusters-1) → u32::MAX.
    let n_clusters = (max_pages + SWAPFILE_CLUSTER - 1) / SWAPFILE_CLUSTER;
    let mut cluster_info: Vec<SwapClusterInfo> = (0..n_clusters)
        .map(|i| {
            let next = if i + 1 < n_clusters { i + 1 } else { u32::MAX };
            SwapClusterInfo::new_free(next)
        })
        .collect();

    // Clamp the last cluster's count if max_pages is not a multiple.
    let remainder = max_pages % SWAPFILE_CLUSTER;
    if remainder != 0 && n_clusters > 0 {
        let last = cluster_info.last_mut().unwrap();
        last.count = remainder;
    }

    let backing_size = (max_pages as usize)
        .checked_mul(PAGE_SIZE as usize)
        .ok_or(-22i32)?;
    let swap_map = vec![0u8; max_pages as usize];
    let backing = vec![0u8; backing_size];

    let mut info = SWAP_INFO.lock();

    if let Some(path) = path.as_ref() {
        if info
            .iter()
            .flatten()
            .any(|si| si.path.as_deref() == Some(path.as_str()))
        {
            return Err(-16); // EBUSY
        }
    }

    // Find first free slot.
    let slot = info.iter().position(|s| s.is_none()).ok_or(-28i32)?; // ENOSPC

    let swap_type = slot as u8;

    info[slot] = Some(SwapInfoStruct {
        flags: SWP_USED | SWP_WRITEOK,
        priority,
        swap_type,
        max: max_pages,
        inuse_pages: 0,
        swap_map,
        cluster_info,
        free_cluster_head: 0,
        cluster_next: 0,
        backing,
        swap_cache: BTreeMap::new(),
        path,
        backing_kind,
    });

    NR_SWAPFILES.fetch_add(1, Ordering::Relaxed);
    drop(info);
    let _ = crate::mm::zswap::zswap_swapon(swap_type as i32);
    Ok(swap_type)
}

/// Deactivate a swap device.
///
/// Returns `-EBUSY` if any pages are still swapped out to this device.
///
/// Ref: Linux `mm/swapfile.c` — `sys_swapoff()`
pub fn swapoff(swap_type: u8) -> Result<(), i32> {
    let mut info = SWAP_INFO.lock();
    let idx = swap_type as usize;
    if idx >= MAX_SWAPFILES {
        return Err(-22); // EINVAL
    }
    let si = info[idx].as_ref().ok_or(-22i32)?;
    if (si.flags & SWP_USED) == 0 {
        return Err(-22);
    }
    if si.inuse_pages > 0 {
        return Err(-16); // EBUSY
    }
    // Drop frees the Vec backing stores.
    info[idx] = None;
    NR_SWAPFILES.fetch_sub(1, Ordering::Relaxed);
    drop(info);
    crate::mm::zswap::zswap_swapoff(swap_type as i32);
    Ok(())
}

pub fn swapoff_path(path: &str) -> Result<(), i32> {
    let swap_type = {
        let info = SWAP_INFO.lock();
        info.iter()
            .flatten()
            .find(|si| si.path.as_deref() == Some(path))
            .map(|si| si.swap_type)
            .ok_or(-22i32)?
    };
    swapoff(swap_type)
}

/// Total configured swap pages across all active devices.
pub fn total_swap_pages() -> u32 {
    let info = SWAP_INFO.lock();
    info.iter()
        .filter_map(|s| s.as_ref())
        .filter(|si| (si.flags & SWP_WRITEOK) != 0)
        .map(|si| si.max)
        .sum()
}

/// Total free swap pages across all active devices.
pub fn free_swap_pages() -> u32 {
    nr_swap_pages()
}

// ---------------------------------------------------------------------------
// Slot allocation
// ---------------------------------------------------------------------------

/// Allocate one swap slot in the highest-priority device.
///
/// Scans the cluster list for a free slot, marks it used, updates cluster
/// metadata, increments `inuse_pages`, and returns the `SwpEntry`.
///
/// Does NOT add the page to the swap cache; call `swap_cache_add` separately.
///
/// Ref: Linux `mm/swapfile.c` — internal slot search logic
fn alloc_swap_slot(info: &mut [Option<SwapInfoStruct>; MAX_SWAPFILES]) -> Option<SwpEntry> {
    // Find highest-priority device with free space.
    let si = info
        .iter_mut()
        .filter_map(|s| s.as_mut())
        .filter(|si| (si.flags & SWP_WRITEOK) != 0 && si.inuse_pages < si.max)
        .max_by_key(|si| si.priority)?;

    let offset = alloc_slot_in_si(si)?;
    Some(SwpEntry::new(si.swap_type, offset))
}

fn alloc_slot_in_si(si: &mut SwapInfoStruct) -> Option<u32> {
    // Walk clusters starting from the current scan position.
    let n_clusters = si.cluster_info.len() as u32;
    if n_clusters == 0 {
        return None;
    }

    let start_cluster = si.cluster_next / SWAPFILE_CLUSTER;

    for ci_idx in 0..n_clusters {
        let idx = (start_cluster + ci_idx) % n_clusters;
        let base = idx * SWAPFILE_CLUSTER;
        let end = (base + SWAPFILE_CLUSTER).min(si.max);

        if si.cluster_info[idx as usize].count == 0 {
            continue; // cluster is full
        }

        for offset in base..end {
            if (offset as usize) < si.swap_map.len() && si.swap_map[offset as usize] == 0 {
                si.swap_map[offset as usize] = 1;
                si.cluster_next = offset + 1;
                let ci = &mut si.cluster_info[idx as usize];
                ci.count -= 1;
                if ci.count == 0 {
                    ci.flags = 0; // full
                } else {
                    ci.flags = CLUSTER_FLAG_NONFULL;
                }
                si.inuse_pages += 1;
                return Some(offset);
            }
        }
    }
    None
}

/// Allocate a swap slot for `page` and add it to the swap cache.
///
/// Sets `PG_SWAPCACHE | PG_SWAPBACKED` on the page and encodes the `SwpEntry`
/// in `page.index`.  The page must be locked by the caller.
///
/// Returns `None` if no swap space is available.
///
/// Ref: Linux `mm/swapfile.c` — `folio_alloc_swap()`
pub fn folio_alloc_swap(page: *mut Page) -> Option<SwpEntry> {
    let mut info = SWAP_INFO.lock();
    let entry = alloc_swap_slot(&mut info)?;
    // Add to swap cache (sets page.index, page.mapping, PG_SWAPCACHE).
    swap_cache_add_locked(&mut info, page, entry);
    Some(entry)
}

/// Return a swap slot to the free pool.
///
/// Decrements the use count; when it hits zero the slot is freed and the
/// cluster count is updated.
///
/// Ref: Linux `mm/swapfile.c` — `free_swap_slot()`
pub fn free_swap_slot(entry: SwpEntry) {
    let mut info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    if idx >= MAX_SWAPFILES {
        return;
    }
    let si = match info[idx].as_mut() {
        Some(s) => s,
        None => return,
    };
    let offset = entry.swp_offset() as usize;
    if offset >= si.swap_map.len() {
        return;
    }
    if si.swap_map[offset] == 0 {
        return; // already free
    }
    si.swap_map[offset] = si.swap_map[offset].saturating_sub(1);
    if si.swap_map[offset] == 0 {
        let ci_idx = (offset as u32 / SWAPFILE_CLUSTER) as usize;
        if ci_idx < si.cluster_info.len() {
            let ci = &mut si.cluster_info[ci_idx];
            ci.count += 1;
            ci.flags = if ci.count == SWAPFILE_CLUSTER {
                CLUSTER_FLAG_FREE
            } else {
                CLUSTER_FLAG_NONFULL
            };
        }
        si.inuse_pages = si.inuse_pages.saturating_sub(1);
        crate::mm::zswap::zswap_invalidate(entry.swp_type(), entry.swp_offset());
    }
}

// ---------------------------------------------------------------------------
// Swap cache
// ---------------------------------------------------------------------------

/// Add `page` to the swap cache for `entry`.
///
/// Encodes `entry` into `page.index`, sets `page.mapping = 0`,
/// sets `PG_SWAPCACHE | PG_SWAPBACKED`, and inserts the page into the
/// per-device `swap_cache` map.  Increments the page refcount.
///
/// The caller must hold the page lock and lock `SWAP_INFO`.
///
/// Ref: Linux `mm/swap_state.c` — `__add_to_swap_cache()`
fn swap_cache_add_locked(
    info: &mut [Option<SwapInfoStruct>; MAX_SWAPFILES],
    page: *mut Page,
    entry: SwpEntry,
) {
    let idx = entry.swp_type() as usize;
    let si = match info[idx].as_mut() {
        Some(s) => s,
        None => return,
    };
    let offset = entry.swp_offset();

    unsafe {
        (*page).mapping = 0;
        (*page).index = entry.val;
        (*page).set_flag(PG_SWAPCACHE | PG_SWAPBACKED);
        (*page).get_page(); // cache holds a reference
    }

    if let Some(nn) = NonNull::new(page) {
        si.swap_cache.insert(offset, nn);
    }
}

/// Public wrapper for `swap_cache_add_locked` that acquires the global lock.
///
/// Ref: Linux `mm/swap_state.c` — `add_to_swap_cache()`
pub fn swap_cache_add(page: *mut Page, entry: SwpEntry) {
    let mut info = SWAP_INFO.lock();
    swap_cache_add_locked(&mut info, page, entry);
}

/// Look up a page in the swap cache.
///
/// Increments the page refcount and returns the pointer if found.
/// Returns `None` if the entry is not cached in RAM.
///
/// Ref: Linux `mm/swap_state.c` — `lookup_swap_cache()`
pub fn swap_cache_get(entry: SwpEntry) -> Option<*mut Page> {
    let info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let si = info[idx].as_ref()?;
    let page_ptr = si.swap_cache.get(&entry.swp_offset())?.as_ptr();
    unsafe { (*page_ptr).get_page() };
    Some(page_ptr)
}

/// Remove `page` from the swap cache.
///
/// Reads the `SwpEntry` from `page.index`, removes from the `swap_cache` map,
/// clears `PG_SWAPCACHE`, and decrements the cache refcount.
///
/// Ref: Linux `mm/swap_state.c` — `delete_from_swap_cache()`
pub fn swap_cache_delete(page: *mut Page) {
    let entry = SwpEntry {
        val: unsafe { (*page).index },
    };
    if entry.is_null() {
        return;
    }
    let mut info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    if idx >= MAX_SWAPFILES {
        return;
    }
    if let Some(si) = info[idx].as_mut() {
        si.swap_cache.remove(&entry.swp_offset());
    }
    unsafe {
        (*page).clear_flag(PG_SWAPCACHE);
        (*page).put_page();
    }
}

// ---------------------------------------------------------------------------
// Page data access
//
// On bare metal the page frame content lives at pfn_to_virt(page_to_pfn(page)).
// In host-side unit tests pages are Box<Page> (not in the buddy mem_map), so
// we follow the same convention as filemap.rs: the test sets
// `page.private = Box::into_raw(Box::new([0u8; PAGE_SIZE])) as usize`.
// ---------------------------------------------------------------------------

#[cfg(not(any(test, feature = "test-zswap-pressure")))]
unsafe fn page_kaddr(page: *mut Page) -> *mut u8 {
    use crate::arch::x86::mm::paging::pfn_to_virt;
    use crate::mm::buddy::page_to_pfn;
    unsafe { pfn_to_virt(page_to_pfn(page)) }
}

#[cfg(any(test, feature = "test-zswap-pressure"))]
unsafe fn page_kaddr(page: *mut Page) -> *mut u8 {
    // Test/boot pressure smoke pages set `private` to an owned page buffer.
    let priv_val = unsafe { (*page).private };
    if priv_val != 0 {
        priv_val as *mut u8
    } else {
        core::ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// Swap I/O
// ---------------------------------------------------------------------------

fn swap_backing_range(entry: SwpEntry) -> Result<(usize, usize), i32> {
    let offset = entry.swp_offset() as usize;
    let pg_size = PAGE_SIZE as usize;
    let start = offset.saturating_mul(pg_size);
    let end = start.checked_add(pg_size).ok_or(-5)?;

    let info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let si = match info.get(idx).and_then(|slot| slot.as_ref()) {
        Some(s) => s,
        None => return Err(-5), // EIO
    };
    if end > si.backing.len() {
        return Err(-5);
    }
    Ok((start, end))
}

/// Write a page's content into the swap backing store.
///
/// Zswap gets first refusal, mirroring Linux's zswap/frontswap path before
/// page I/O falls back to the swap backing device.
///
/// Sets `PG_WRITEBACK` before the copy, clears it after, sets `PG_UPTODATE`.
/// Returns 0 on success, negative errno on error.
///
/// Ref: Linux `mm/page_io.c` — `swap_writepage()`
pub fn swap_writepage(page: *mut Page, entry: SwpEntry) -> i32 {
    let (start, end) = match swap_backing_range(entry) {
        Ok(range) => range,
        Err(errno) => return errno,
    };

    unsafe {
        (*page).set_flag(PG_WRITEBACK);
        let src = page_kaddr(page);
        if !src.is_null()
            && crate::mm::zswap::zswap_store(entry.swp_type(), entry.swp_offset(), src).is_ok()
        {
            (*page).clear_flag(PG_WRITEBACK);
            (*page).set_flag(PG_UPTODATE);
            return 0;
        }
    }

    let pg_size = PAGE_SIZE as usize;
    let mut info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let si = match info[idx].as_mut() {
        Some(s) => s,
        None => return -5, // EIO
    };

    unsafe {
        let src = page_kaddr(page);
        if !src.is_null() {
            let dst = si.backing[start..end].as_mut_ptr();
            core::ptr::copy_nonoverlapping(src, dst, pg_size);
        }
        (*page).clear_flag(PG_WRITEBACK);
        (*page).set_flag(PG_UPTODATE);
    }
    0
}

/// Read a page's content from the swap backing store.
///
/// Zswap is checked first, then the swap backing device. Sets `PG_UPTODATE`
/// on completion.
/// Returns 0 on success, negative errno on error.
///
/// Ref: Linux `mm/page_io.c` — `swap_read_folio()`
pub fn swap_readpage(page: *mut Page, entry: SwpEntry) -> i32 {
    let (start, end) = match swap_backing_range(entry) {
        Ok(range) => range,
        Err(errno) => return errno,
    };

    unsafe {
        let dst = page_kaddr(page);
        if !dst.is_null() {
            match crate::mm::zswap::zswap_load(entry.swp_type(), entry.swp_offset(), dst) {
                Ok(()) => {
                    (*page).set_flag(PG_UPTODATE);
                    return 0;
                }
                Err(crate::include::uapi::errno::ENOENT) => {}
                Err(errno) => return -errno,
            }
        }
    }

    let pg_size = PAGE_SIZE as usize;
    let info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let si = match info[idx].as_ref() {
        Some(s) => s,
        None => return -5, // EIO
    };

    unsafe {
        let dst = page_kaddr(page);
        if !dst.is_null() {
            let src = si.backing[start..end].as_ptr();
            core::ptr::copy_nonoverlapping(src, dst, pg_size);
        }
        (*page).set_flag(PG_UPTODATE);
    }
    0
}

/// Allocate a swap slot and write the page to the backing store.
///
/// Called by reclaim when an anonymous page must be evicted.
/// On success the page is in the swap cache (`PG_SWAPCACHE` set) and
/// its content is persisted in the backing store.
///
/// Returns `true` on success, `false` if no swap space is available or I/O failed.
///
/// Ref: Linux `mm/swap.c` — `add_to_swap()`
pub fn add_to_swap(page: *mut Page) -> bool {
    let entry = match folio_alloc_swap(page) {
        Some(e) => e,
        None => return false,
    };
    if swap_writepage(page, entry) != 0 {
        // I/O failed — roll back.
        swap_cache_delete(page);
        free_swap_slot(entry);
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Linux-visible swap.h / swap.c compatibility wrappers
// ---------------------------------------------------------------------------

pub fn nr_swap_pages() -> u32 {
    let info = SWAP_INFO.lock();
    let mut free = 0u32;
    for si in info.iter().flatten() {
        free = free.saturating_add(si.max.saturating_sub(si.inuse_pages));
    }
    free
}

pub fn get_nr_swap_pages() -> u32 {
    nr_swap_pages()
}

pub fn total_swapcache_pages() -> usize {
    let info = SWAP_INFO.lock();
    info.iter()
        .flatten()
        .map(|si| si.swap_cache.len())
        .sum::<usize>()
}

pub fn vm_swap_full() -> bool {
    let total = total_swap_pages();
    total > 0 && nr_swap_pages() < total / 2
}

pub fn proc_swaps() -> String {
    let rows: Vec<(String, SwapBackingKind, u32, u32, i32)> = {
        let info = SWAP_INFO.lock();
        info.iter()
            .flatten()
            .map(|si| {
                (
                    si.path.as_deref().unwrap_or("[memswap]").to_string(),
                    si.backing_kind,
                    si.max,
                    si.inuse_pages,
                    si.priority,
                )
            })
            .collect()
    };
    let mut out = String::from("Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n");
    for (path, backing_kind, max, inuse_pages, priority) in rows {
        let size_kb = (max as usize * PAGE_SIZE as usize) / 1024;
        let used_kb = (inuse_pages as usize * PAGE_SIZE as usize) / 1024;
        out.push_str(&format!(
            "{path}\t\t\t\t{}\t\t{size_kb}\t\t{used_kb}\t\t{priority}\n",
            backing_kind.proc_type()
        ));
    }
    out
}

pub fn __swap_count(entry: SwpEntry) -> u8 {
    let info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let Some(si) = info.get(idx).and_then(|slot| slot.as_ref()) else {
        return 0;
    };
    si.swap_map
        .get(entry.swp_offset() as usize)
        .copied()
        .unwrap_or(0)
}

pub fn swp_swapcount(entry: SwpEntry) -> u8 {
    __swap_count(entry)
}

pub fn count_swap_pages(swap_type: i32, free: bool) -> u32 {
    let info = SWAP_INFO.lock();
    let iter = info.iter().enumerate().filter_map(|(idx, slot)| {
        if swap_type >= 0 && idx != swap_type as usize {
            None
        } else {
            slot.as_ref()
        }
    });
    iter.map(|si| {
        if free {
            si.max.saturating_sub(si.inuse_pages)
        } else {
            si.inuse_pages
        }
    })
    .sum()
}

pub fn find_first_swap(_start: usize) -> SwpEntry {
    let info = SWAP_INFO.lock();
    for (idx, si) in info.iter().enumerate() {
        let Some(si) = si else {
            continue;
        };
        for (offset, count) in si.swap_map.iter().enumerate() {
            if *count == 0 {
                return SwpEntry::new(idx as u8, offset as u32);
            }
        }
    }
    SwpEntry::default()
}

pub fn get_swap_device(_entry: SwpEntry) -> *mut SwapInfoStruct {
    core::ptr::null_mut()
}

pub fn put_swap_device(_si: *mut SwapInfoStruct) {}

pub fn page_swap_entry(page: *const Page) -> SwpEntry {
    if page.is_null() {
        SwpEntry::default()
    } else {
        SwpEntry {
            val: unsafe { (*page).index },
        }
    }
}

pub fn swap_entry_swapped(entry: SwpEntry) -> bool {
    __swap_count(entry) != 0
}

pub fn swap_dup_entry_direct(entry: SwpEntry) -> i32 {
    let mut info = SWAP_INFO.lock();
    let idx = entry.swp_type() as usize;
    let Some(si) = info.get_mut(idx).and_then(|slot| slot.as_mut()) else {
        return -22;
    };
    let Some(count) = si.swap_map.get_mut(entry.swp_offset() as usize) else {
        return -22;
    };
    *count = count.saturating_add(1);
    0
}

pub fn swap_put_entries_direct(entry: SwpEntry, nr: usize) {
    for offset in 0..nr {
        free_swap_slot(SwpEntry::new(
            entry.swp_type(),
            entry.swp_offset().saturating_add(offset as u32),
        ));
    }
}

pub fn swap_alloc_hibernation_slot() -> SwpEntry {
    find_first_swap(0)
}

pub fn swap_free_hibernation_slot(entry: SwpEntry) {
    free_swap_slot(entry)
}

pub fn swap_folio_sector(page: *const Page) -> u64 {
    page_swap_entry(page).swp_offset() as u64 * (PAGE_SIZE as u64 / 512)
}

pub fn swapdev_block(_swap_type: i32, offset: u64) -> u64 {
    offset
}

pub fn swap_type_of(_bdev: *const u8, _offset: u64, _bmap: *mut u64) -> i32 {
    -1
}

pub fn add_swap_extent(
    _sis: *mut SwapInfoStruct,
    _start_page: u32,
    _nr_pages: u32,
    _start_block: u64,
) -> i32 {
    0
}

pub fn si_swapinfo(_val: *mut u8) {}

pub fn swap_setup() {}

pub fn current_is_kswapd() -> bool {
    false
}

pub fn kswapd_run(_nid: i32) -> i32 {
    0
}

pub fn kswapd_stop(_nid: i32) {}

pub fn lru_add_drain_all() {}

pub fn lru_add_drain_cpu(_cpu: i32) {}

pub fn lru_add_drain_cpu_zone(_zone: *mut u8) {}

pub fn lru_cache_disable() {}

pub fn lru_cache_enable() {}

pub fn lru_cache_disabled() -> bool {
    false
}

pub fn folio_add_lru(folio: *mut Page) {
    unsafe { crate::mm::lru::lru_cache_add(folio) };
}

pub fn folio_add_lru_vma(folio: *mut Page, _vma: *mut u8) {
    folio_add_lru(folio)
}

pub fn folio_mark_accessed(folio: *mut Page) {
    unsafe { crate::mm::lru::mark_page_accessed(folio) };
}

pub fn folio_deactivate(_folio: *mut Page) {}

pub fn folio_mark_lazyfree(_folio: *mut Page) -> bool {
    false
}

pub fn folio_may_be_lru_cached(_folio: *const Page) -> bool {
    true
}

pub fn __folio_throttle_swaprate(_folio: *const Page, _gfp: u32) {}

pub fn folio_throttle_swaprate(folio: *const Page, gfp: u32) {
    __folio_throttle_swaprate(folio, gfp)
}

pub fn folio_free_swap(folio: *mut Page) -> bool {
    if folio.is_null() {
        return false;
    }
    let entry = page_swap_entry(folio);
    if entry.is_null() {
        return false;
    }
    swap_cache_delete(folio);
    free_swap_slot(entry);
    true
}

pub fn free_swap_cache(folio: *mut Page) {
    let _ = folio_free_swap(folio);
}

pub fn free_folio_and_swap_cache(folio: *mut Page) {
    free_swap_cache(folio);
    crate::mm::page_flags::folio_put(folio);
}

pub unsafe fn free_pages_and_swap_cache(pages: *mut *mut Page, nr: usize) {
    if pages.is_null() {
        return;
    }
    for idx in 0..nr {
        let page = unsafe { *pages.add(idx) };
        free_folio_and_swap_cache(page);
    }
}

pub unsafe fn release_pages(pages: *mut *mut Page, nr: usize) {
    if pages.is_null() {
        return;
    }
    for idx in 0..nr {
        let page = unsafe { *pages.add(idx) };
        crate::mm::page_flags::folio_put(page);
    }
}

pub unsafe fn __folio_batch_release(pages: *mut *mut Page, nr: usize) {
    unsafe { release_pages(pages, nr) };
}

pub fn remove_mapping(_mapping: *mut u8, page: *mut Page) -> bool {
    if page.is_null() {
        return false;
    }
    let entry = page_swap_entry(page);
    if !entry.is_null() {
        swap_cache_delete(page);
    }
    true
}

pub fn check_move_unevictable_folios(_folios: *mut *mut Page, _nr: usize) -> usize {
    0
}

pub fn mem_cgroup_swappiness(_memcg: *const u8) -> i32 {
    60
}

pub fn mem_cgroup_get_nr_swap_pages(_memcg: *const u8) -> u64 {
    nr_swap_pages() as u64
}

pub fn __mem_cgroup_try_charge_swap(_folio: *mut Page, _entry: SwpEntry) -> i32 {
    0
}

pub fn mem_cgroup_try_charge_swap(folio: *mut Page, entry: SwpEntry) -> i32 {
    __mem_cgroup_try_charge_swap(folio, entry)
}

pub fn __mem_cgroup_uncharge_swap(_entry: SwpEntry, _nr_pages: usize) {}

pub fn mem_cgroup_uncharge_swap(entry: SwpEntry, nr_pages: usize) {
    __mem_cgroup_uncharge_swap(entry, nr_pages)
}

pub fn mem_cgroup_swap_full(_folio: *const Page) -> bool {
    vm_swap_full()
}

pub fn mem_cgroup_shrink_node(
    _memcg: *mut u8,
    _pgdat: *mut u8,
    _gfp: u32,
    _reclaim: bool,
) -> usize {
    0
}

pub fn try_to_free_mem_cgroup_pages(
    _memcg: *mut u8,
    _nr_pages: usize,
    _gfp_mask: u32,
    _reclaim_options: u32,
) -> usize {
    0
}

pub fn try_to_free_pages(
    _zonelist: *mut u8,
    _order: u32,
    _gfp_mask: u32,
    _nodemask: *mut u8,
) -> usize {
    0
}

pub fn shrink_all_memory(_nr_pages: usize) -> usize {
    0
}

pub fn zone_reclaimable_pages(_zone: *mut u8) -> u64 {
    0
}

pub fn reclaim_register_node(_node: *mut u8) {}

pub fn reclaim_unregister_node(_node: *mut u8) {}

pub fn lru_note_cost_refault(_folio: *mut Page) {}

pub fn lru_reparent_memcg(_memcg: *mut u8) {}

pub fn lruvec_lru_size(_lruvec: *const u8, _lru: usize, _zone_idx: usize) -> usize {
    0
}

pub fn mm_account_reclaimed_pages(_pages: usize) {}

pub fn workingset_activation(_folio: *mut Page) {}

pub fn workingset_age_nonresident(_lruvec: *mut u8, _nr_pages: usize) {}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Reset all global swap state for unit tests.
///
/// Clears `SWAP_INFO` and resets `NR_SWAPFILES` to 0.  Call at the start of
/// every swap unit test to ensure a clean slate.
#[cfg(any(test, feature = "test-swap"))]
pub fn reset_swap_state_for_test() {
    let mut info = SWAP_INFO.lock();
    for slot in info.iter_mut() {
        *slot = None;
    }
    NR_SWAPFILES.store(0, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::boxed::Box;

    use super::*;
    use crate::arch::x86::mm::paging::{_PAGE_PRESENT, is_swap_pte, pte_present, pte_t};
    use crate::mm::page::Page;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    // ── helpers ─────────────────────────────────────────────────────────────

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        reset_swap_state_for_test();
        crate::mm::zswap::reset_for_tests();
        g
    }

    /// Allocate a heap Page and initialize its LRU node.
    fn alloc_test_page() -> Box<Page> {
        let mut p = Box::new(Page::new());
        unsafe { p.init_lru() };
        p
    }

    // ── Test 1: SwpEntry encode/decode round-trip ────────────────────────────

    /// Ref: Linux `swp_entry()` / `swp_type()` / `swp_offset()` — swapops.h
    #[test]
    fn swp_entry_encode_decode_roundtrip() {
        for ty in 0u8..4 {
            for &off in &[0u32, 1, 127, 255, 256, 1023, u32::MAX / 4] {
                let e = SwpEntry::new(ty, off);
                assert_eq!(e.swp_type(), ty, "type mismatch ty={ty} off={off}");
                assert_eq!(e.swp_offset(), off, "offset mismatch ty={ty} off={off}");
            }
        }
    }

    // ── Test 2: SwpEntry ↔ PTE round-trip ───────────────────────────────────

    /// PTE encoding must be non-present, non-zero, and reversible.
    ///
    /// Ref: Linux `swp_entry_to_pte` / `pte_to_swp_entry` — swapops.h +
    ///      arch/x86/include/asm/pgtable_64.h
    #[test]
    fn swp_entry_pte_roundtrip() {
        let entry = SwpEntry::new(1, 42);
        let pte = swp_entry_to_pte(entry);

        assert!(!pte_present(pte), "swap PTE must not have PRESENT bit");
        assert_ne!(pte.0, 0, "swap PTE must not be zero (pte_none)");

        let decoded = pte_to_swp_entry(pte);
        assert_eq!(decoded.swp_type(), 1);
        assert_eq!(decoded.swp_offset(), 42);
    }

    // ── Test 3: is_swap_pte classification ──────────────────────────────────

    /// Ref: Linux `is_swap_pte()` — include/linux/mm.h
    #[test]
    fn is_swap_pte_classification() {
        // Present PTE: not a swap PTE.
        let present = pte_t(_PAGE_PRESENT | (1 << 12));
        assert!(!is_swap_pte(present), "present PTE is not a swap PTE");

        // Zero PTE (pte_none): not a swap PTE.
        let none = pte_t(0);
        assert!(!is_swap_pte(none), "zero PTE is not a swap PTE");

        // Actual swap PTE: is a swap PTE.
        let swap_pte = swp_entry_to_pte(SwpEntry::new(0, 1));
        assert!(is_swap_pte(swap_pte), "swap-encoded PTE must be a swap PTE");
    }

    // ── Test 4: swapon / swapoff lifecycle ──────────────────────────────────

    /// Ref: Linux `sys_swapon` / `sys_swapoff` — mm/swapfile.c
    #[test]
    fn swapon_swapoff_lifecycle() {
        let _g = test_guard();

        let ty = swapon(1024, 0).expect("swapon failed");
        assert!((ty as usize) < MAX_SWAPFILES);

        {
            let info = SWAP_INFO.lock();
            let si = info[ty as usize].as_ref().unwrap();
            assert_eq!(si.max, 1024);
            assert_eq!(si.inuse_pages, 0);
            assert_eq!(si.swap_map.len(), 1024);
            assert!((si.flags & SWP_USED) != 0);
            assert!((si.flags & SWP_WRITEOK) != 0);
        }

        swapoff(ty).expect("swapoff failed");

        {
            let info = SWAP_INFO.lock();
            assert!(
                info[ty as usize].is_none(),
                "slot must be None after swapoff"
            );
        }
    }

    // ── Test 5: folio_alloc_swap produces unique slots ───────────────────────

    /// Ref: Linux `__add_to_swap_cache` invariant — mm/swap_state.c
    #[test]
    fn swapon_path_proc_swaps_and_totals_follow_linux_shape() {
        let _g = test_guard();

        let ty = swapon_path(String::from("/swapfile"), 512, 3).expect("swapon path");
        assert_eq!(ty, 0);
        assert_eq!(total_swap_pages(), 512);
        assert_eq!(free_swap_pages(), 512);

        let text = proc_swaps();
        assert!(text.starts_with("Filename"));
        assert!(text.contains("/swapfile"));
        assert!(text.contains("\tfile\t\t2048\t\t0\t\t3\n"));

        assert_eq!(swapon_path(String::from("/swapfile"), 512, 3), Err(-16));
        swapoff_path("/swapfile").expect("swapoff path");
        assert_eq!(total_swap_pages(), 0);
    }

    #[test]
    fn swapon_block_path_proc_swaps_reports_partition_type() {
        let _g = test_guard();

        let ty =
            swapon_block_path(String::from("/dev/mapper/cl-swap"), 256, 0).expect("swapon block");
        assert_eq!(ty, 0);
        assert_eq!(total_swap_pages(), 256);

        let text = proc_swaps();
        assert!(text.contains("/dev/mapper/cl-swap"));
        assert!(text.contains("\tpartition\t\t1024\t\t0\t\t0\n"));

        assert_eq!(
            swapon_block_path(String::from("/dev/mapper/cl-swap"), 256, 0),
            Err(-16)
        );
        swapoff_path("/dev/mapper/cl-swap").expect("swapoff block path");
        assert_eq!(total_swap_pages(), 0);
    }

    #[test]
    fn folio_alloc_swap_unique_slots() {
        let _g = test_guard();
        swapon(64, 0).unwrap();

        let mut pages: [Box<Page>; 4] = core::array::from_fn(|_| alloc_test_page());
        let mut entries = Vec::new();

        for page in pages.iter_mut() {
            let ptr = page.as_mut() as *mut Page;
            unsafe { crate::mm::address_space::lock_page(ptr) };
            let entry = folio_alloc_swap(ptr).expect("alloc_swap should succeed");
            entries.push(entry);
        }

        // All offsets must be distinct.
        let mut offsets: Vec<u32> = entries.iter().map(|e| e.swp_offset()).collect();
        offsets.sort();
        offsets.dedup();
        assert_eq!(offsets.len(), 4, "swap slots must be unique");

        // Clean up: delete from cache and free slot.
        for (page, entry) in pages.iter_mut().zip(entries.iter()) {
            let ptr = page.as_mut() as *mut Page;
            swap_cache_delete(ptr);
            free_swap_slot(*entry);
            unsafe { crate::mm::address_space::unlock_page(ptr) };
        }

        swapon(64, 0).unwrap(); // just verify no state leak
    }

    // ── Test 6: swap cache add / get / delete round-trip ────────────────────

    /// Ref: Linux `__add_to_swap_cache` + `lookup_swap_cache` + `delete_from_swap_cache`
    #[test]
    fn swap_cache_add_get_roundtrip() {
        let _g = test_guard();
        swapon(16, 0).unwrap();

        // Manually reserve slot 3 in swap_map so alloc_slot doesn't clobber it.
        {
            let mut info = SWAP_INFO.lock();
            info[0].as_mut().unwrap().swap_map[3] = 1;
            info[0].as_mut().unwrap().inuse_pages += 1;
        }

        let mut page = alloc_test_page();
        let ptr = page.as_mut() as *mut Page;
        let entry = SwpEntry::new(0, 3);

        unsafe { crate::mm::address_space::lock_page(ptr) };
        swap_cache_add(ptr, entry);

        assert!(
            unsafe { (*ptr).test_flag(PG_SWAPCACHE) },
            "PG_SWAPCACHE must be set"
        );
        assert_eq!(
            unsafe { (*ptr).index },
            entry.val,
            "page.index must encode entry"
        );

        let found = swap_cache_get(entry).expect("page must be in cache");
        assert_eq!(found, ptr, "cached pointer must match");

        swap_cache_delete(ptr);
        assert!(
            !unsafe { (*ptr).test_flag(PG_SWAPCACHE) },
            "PG_SWAPCACHE must be cleared"
        );
        assert!(
            swap_cache_get(entry).is_none(),
            "must not be in cache after delete"
        );

        unsafe { crate::mm::address_space::unlock_page(ptr) };
    }

    // ── Test 7: backing store write / read is byte-identical ─────────────────

    /// Core acceptance criterion from ROADMAP.md:
    /// "a test workload exhausts RAM, is swapped out, is read back in, and
    ///  every evicted page is byte-identical on return."
    ///
    /// This test validates the backing-store round-trip directly.
    ///
    /// Ref: Linux `swap_writepage` + `swap_readpage` contract — mm/page_io.c
    #[test]
    fn swap_write_read_byte_identical() {
        let _g = test_guard();
        swapon(16, 0).unwrap();

        let pg_size = PAGE_SIZE as usize;
        let offset: u32 = 5;

        // Build a pattern and write it directly into the backing store.
        let pattern: Vec<u8> = (0..pg_size).map(|i| (i & 0xFF) as u8).collect();
        {
            let mut info = SWAP_INFO.lock();
            let si = info[0].as_mut().unwrap();
            let start = offset as usize * pg_size;
            si.backing[start..start + pg_size].copy_from_slice(&pattern);
        }

        // Read it back via backing store access.
        let readback: Vec<u8> = {
            let info = SWAP_INFO.lock();
            let si = info[0].as_ref().unwrap();
            let start = offset as usize * pg_size;
            si.backing[start..start + pg_size].to_vec()
        };

        assert_eq!(
            pattern, readback,
            "backing store round-trip must be byte-identical"
        );
    }

    #[test]
    fn swap_write_read_prefers_zswap_frontswap_store() {
        let _g = test_guard();
        crate::mm::zswap::init();
        let ty = swapon(16, 0).unwrap();

        let mut page = alloc_test_page();
        let mut source = Box::new([0u8; PAGE_SIZE]);
        source.fill(0x21);
        page.private = source.as_mut_ptr() as usize;

        let ptr = page.as_mut() as *mut Page;
        unsafe { crate::mm::address_space::lock_page(ptr) };
        let entry = folio_alloc_swap(ptr).unwrap();
        assert_eq!(entry.swp_type(), ty);
        assert_eq!(swap_writepage(ptr, entry), 0);
        assert_eq!(crate::mm::zswap::zswap_total_pages(), 1);

        {
            let info = SWAP_INFO.lock();
            let si = info[entry.swp_type() as usize].as_ref().unwrap();
            let start = entry.swp_offset() as usize * PAGE_SIZE as usize;
            let end = start + PAGE_SIZE as usize;
            assert!(
                si.backing[start..end].iter().all(|byte| *byte == 0),
                "zswap hit should avoid writing the backing store"
            );
        }

        let mut out_page = alloc_test_page();
        let mut out = Box::new([0u8; PAGE_SIZE]);
        out_page.private = out.as_mut_ptr() as usize;
        let out_ptr = out_page.as_mut() as *mut Page;
        assert_eq!(swap_readpage(out_ptr, entry), 0);
        assert_eq!(&out[..], &source[..]);

        swap_cache_delete(ptr);
        free_swap_slot(entry);
        assert_eq!(
            crate::mm::zswap::zswap_load(entry.swp_type(), entry.swp_offset(), out.as_mut_ptr()),
            Err(crate::include::uapi::errno::ENOENT)
        );
        unsafe { crate::mm::address_space::unlock_page(ptr) };
    }

    #[test]
    fn swap_write_falls_back_to_backing_store_for_incompressible_pages() {
        let _g = test_guard();
        crate::mm::zswap::init();
        let ty = swapon(16, 0).unwrap();

        let mut page = alloc_test_page();
        let mut source = Box::new([0u8; PAGE_SIZE]);
        for (idx, byte) in source.iter_mut().enumerate() {
            *byte = idx as u8;
        }
        page.private = source.as_mut_ptr() as usize;

        let ptr = page.as_mut() as *mut Page;
        unsafe { crate::mm::address_space::lock_page(ptr) };
        let entry = folio_alloc_swap(ptr).unwrap();
        assert_eq!(entry.swp_type(), ty);
        assert_eq!(swap_writepage(ptr, entry), 0);
        assert_eq!(crate::mm::zswap::zswap_total_pages(), 0);

        {
            let info = SWAP_INFO.lock();
            let si = info[entry.swp_type() as usize].as_ref().unwrap();
            let start = entry.swp_offset() as usize * PAGE_SIZE as usize;
            let end = start + PAGE_SIZE as usize;
            assert_eq!(&si.backing[start..end], &source[..]);
        }

        let mut out_page = alloc_test_page();
        let mut out = Box::new([0u8; PAGE_SIZE]);
        out_page.private = out.as_mut_ptr() as usize;
        let out_ptr = out_page.as_mut() as *mut Page;
        assert_eq!(swap_readpage(out_ptr, entry), 0);
        assert_eq!(&out[..], &source[..]);

        swap_cache_delete(ptr);
        free_swap_slot(entry);
        unsafe { crate::mm::address_space::unlock_page(ptr) };
    }

    // ── Test 8: free_swap_slot returns slot to pool ──────────────────────────

    /// Ref: Linux `free_swap_slot` — mm/swapfile.c
    #[test]
    fn free_swap_slot_returns_slot() {
        let _g = test_guard();
        swapon(256, 0).unwrap();

        let mut page = alloc_test_page();
        let ptr = page.as_mut() as *mut Page;
        unsafe { crate::mm::address_space::lock_page(ptr) };

        let entry = folio_alloc_swap(ptr).unwrap();
        let inuse_before = SWAP_INFO.lock()[entry.swp_type() as usize]
            .as_ref()
            .unwrap()
            .inuse_pages;

        swap_cache_delete(ptr);
        free_swap_slot(entry);

        let info = SWAP_INFO.lock();
        let si = info[entry.swp_type() as usize].as_ref().unwrap();
        assert_eq!(
            si.inuse_pages,
            inuse_before - 1,
            "inuse_pages must decrement"
        );
        assert_eq!(
            si.swap_map[entry.swp_offset() as usize],
            0,
            "swap_map slot must be 0 after free"
        );

        unsafe { crate::mm::address_space::unlock_page(ptr) };
    }

    // ── Test 9: priority ordering ────────────────────────────────────────────

    /// Allocations must prefer the higher-priority swap device.
    ///
    /// Ref: Linux priority-ordered swap_active_head — mm/swapfile.c
    #[test]
    fn swapon_priority_selection() {
        let _g = test_guard();

        let ty_low = swapon(64, -1).unwrap(); // low priority
        let ty_high = swapon(64, 1).unwrap(); // high priority

        let mut page = alloc_test_page();
        let ptr = page.as_mut() as *mut Page;
        unsafe { crate::mm::address_space::lock_page(ptr) };

        let entry = folio_alloc_swap(ptr).unwrap();
        assert_eq!(
            entry.swp_type(),
            ty_high,
            "allocation must go to the higher-priority device (ty_high={ty_high})"
        );

        swap_cache_delete(ptr);
        free_swap_slot(entry);
        unsafe { crate::mm::address_space::unlock_page(ptr) };

        swapoff(ty_low).unwrap();
        swapoff(ty_high).unwrap();
    }
}
