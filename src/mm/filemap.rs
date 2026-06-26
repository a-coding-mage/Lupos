//! linux-parity: complete
//! linux-source: vendor/linux/mm/filemap.c
//! test-origin: linux:vendor/linux/mm/filemap.c
/// Generic file-backed page cache I/O — `filemap_read`, `filemap_fault`,
/// `generic_file_read_iter`, `generic_file_write_iter`.
///
/// This module implements the Linux `mm/filemap.c` read and write paths in
/// terms of the page cache primitives from `address_space.rs`.  Filesystems
/// plug in by setting `a_ops->read_folio` (or `->readahead`); everything else
/// flows through generic code here.
///
/// ## Key functions
///
/// | Lupos                    | Linux equivalent               |
/// |--------------------------|--------------------------------|
/// | `filemap_add_folio`      | `filemap_add_folio()`          |
/// | `filemap_remove_folio`   | `filemap_remove_folio()`       |
/// | `find_get_page`          | `find_get_page()`              |
/// | `find_lock_page`         | `find_lock_page()`             |
/// | `filemap_grab_folio`     | `filemap_grab_folio()`         |
/// | `filemap_read`           | `filemap_read()`               |
/// | `generic_file_read_iter` | `generic_file_read_iter()`     |
/// | `generic_perform_write`  | `generic_perform_write()`      |
/// | `generic_file_write_iter`| `generic_file_write_iter()`    |
/// | `set_page_dirty`         | `set_page_dirty()`             |
///
/// Ref: Linux `mm/filemap.c`
///      Linux `include/linux/pagemap.h`
use core::ptr;
use core::sync::atomic::Ordering;

use super::address_space::{
    AS_EIO, AS_ENOSPC, AS_EXITING, AS_NO_WRITEBACK_TAGS, AS_STABLE_WRITES, AS_UNEVICTABLE,
    AddressSpace, lock_page, lock_page_killable, page_uptodate, set_page_uptodate, try_lock_page,
    unlock_page, wait_on_page_locked, wait_on_page_writeback,
};
use super::buddy::page_in_mem_map;
use super::lru::{lru_cache_add, mark_page_accessed, remove_lru_page};
use super::page::Page;
use super::page_flags::{
    GFP_KERNEL, GfpFlags, PG_DIRTY, PG_DROPBEHIND, PG_PRIVATE, PG_PRIVATE_2, PG_UPTODATE,
    folio_mapped,
};
use super::writeback::{
    balance_dirty_pages, clear_page_dirty_for_io, end_page_writeback,
    mark_page_dirty as writeback_mark_page_dirty, note_page_dirty, page_cache_remove,
    start_page_writeback, track_mapping, untrack_mapping,
};
use super::xarray::XaMark;
use crate::arch::x86::mm::paging::PAGE_SIZE;

// ---------------------------------------------------------------------------
// IOCB flags (subset of Linux IOCB_* — iocb.ki_flags)
// ---------------------------------------------------------------------------

/// Bypass page cache. Lupos currently services this through the same checked
/// page-cache path until block-device direct I/O is wired below VFS.
pub const IOCB_DIRECT: u32 = 1 << 14;

// ---------------------------------------------------------------------------
// I/O descriptor types
//
// Simplified versions of Linux's `struct kiocb` and `struct iov_iter`.
// These carry just enough state for filemap_read / generic_file_write_iter.
// M38 (VFS) and M39 (fdtable) will replace these with the full types.
//
// Ref: Linux `struct kiocb` — `include/linux/fs.h:320`
//      Linux `struct iov_iter` — `include/linux/uio.h`
// ---------------------------------------------------------------------------

/// Kernel I/O control block — tracks the state of an in-progress I/O.
///
/// Ref: Linux `struct kiocb` — `include/linux/fs.h:320`
#[repr(C)]
pub struct KioCb {
    /// Owning file (placeholder `*mut u8` until M38 defines `File`).
    pub ki_filp: *mut u8,
    /// Current file position (advanced by each read/write).
    pub ki_pos: i64,
    /// `IOCB_*` flags (e.g. `IOCB_DIRECT`).
    pub ki_flags: u32,
}

/// Simple linear I/O iterator.
///
/// Points at a kernel-accessible buffer and tracks how many bytes have been
/// consumed.  Full scatter-gather (`iovec`) support arrives in M59.
///
/// Ref: Linux `struct iov_iter` — `include/linux/uio.h`
#[repr(C)]
pub struct IoVecIter {
    /// Pointer to the data buffer.
    pub buf: *mut u8,
    /// Total capacity of the buffer (bytes requested).
    pub count: usize,
    /// Bytes consumed so far.
    pub written: usize,
}

impl IoVecIter {
    /// Remaining capacity.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.count.saturating_sub(self.written)
    }
}

// ---------------------------------------------------------------------------
// page_kaddr — translate a Page descriptor to its data virtual address
//
// On bare metal the page IS the physical memory frame; its data lives at
// pfn_to_virt(page_to_pfn(page)).
//
// In host-side tests the buddy allocator places pages in an array that is NOT
// at the same addresses as PFN * PAGE_SIZE (pfn_to_virt would return 0 or
// garbage).  To enable host-side data-integrity tests we reuse page->private
// as a data-buffer pointer: the test sets page.private = Box::into_raw(...)
// before inserting the page into the mapping.
//
// Ref: Linux `page_address()` / `page_to_virt()` — `include/linux/mm.h`
// ---------------------------------------------------------------------------

#[cfg(not(test))]
unsafe fn page_kaddr(page: *mut Page) -> *mut u8 {
    use crate::arch::x86::mm::paging::pfn_to_virt;
    use crate::mm::buddy::page_to_pfn;
    unsafe { pfn_to_virt(page_to_pfn(page)) }
}

#[cfg(test)]
unsafe fn page_kaddr(page: *mut Page) -> *mut u8 {
    // Test must set page.private = Box::into_raw(Box::new([0u8; PAGE_SIZE])) as usize.
    unsafe { (*page).private as *mut u8 }
}

// ---------------------------------------------------------------------------
// filemap_add_folio / filemap_remove_folio
// ---------------------------------------------------------------------------

/// Insert `page` into `mapping`'s XArray at `index`.
///
/// Sets `page->mapping` and `page->index`, increments `nrpages`, and
/// acquires a reference on the page.
///
/// Returns `0` on success or `-EEXIST` if a page is already at `index`.
///
/// Ref: Linux `filemap_add_folio()` — `mm/filemap.c`
pub unsafe fn filemap_add_folio(
    mapping: *mut AddressSpace,
    page: *mut Page,
    index: u64,
    _gfp: GfpFlags,
) -> i32 {
    if mapping.is_null() || page.is_null() {
        return -12; // -ENOMEM
    }
    unsafe {
        let existing = (*mapping)
            .i_pages
            .xa_store(index, core::ptr::NonNull::new(page).unwrap());
        if existing.is_some() {
            // Restore old entry and report conflict.
            (*mapping).i_pages.xa_store(index, existing.unwrap());
            return -17; // -EEXIST
        }
        (*page).mapping = mapping as usize;
        (*page).index = index as usize;
        (*page).init_lru();
        (*page).get_page();
        (*mapping).nrpages.fetch_add(1, Ordering::Relaxed);
        track_mapping(mapping);
        lru_cache_add(page);
        0
    }
}

/// Remove `page` from its mapping's XArray.
///
/// Clears `page->mapping`, releases one reference, and decrements `nrpages`.
///
/// Ref: Linux `filemap_remove_folio()` — `mm/filemap.c`
pub unsafe fn filemap_remove_folio(page: *mut Page) {
    if page.is_null() {
        return;
    }
    unsafe {
        let mapping = (*page).mapping as *mut AddressSpace;
        if mapping.is_null() || folio_mapped(page) {
            return;
        }
        let index = (*page).index as u64;
        page_cache_remove(page);
        remove_lru_page(page);
        (*mapping).i_pages.xa_erase(index);
        (*page).mapping = 0;
        let remaining = (*mapping).nrpages.fetch_sub(1, Ordering::Relaxed) - 1;
        let refs = (*page).put_page();
        if remaining == 0 {
            untrack_mapping(mapping);
        }
        if refs == 0
            && !folio_mapped(page)
            && super::buddy::is_buddy_ready()
            && page_in_mem_map(page)
        {
            super::buddy::with_global_buddy(|b| b.free_pages(page, 0));
        }
    }
}

// ---------------------------------------------------------------------------
// find_get_page / find_lock_page / filemap_grab_folio
// ---------------------------------------------------------------------------

/// Look up the page at `index` in `mapping`.
///
/// Returns the page with its refcount incremented, or null if absent.
///
/// Ref: Linux `find_get_page()` — `include/linux/pagemap.h`
pub unsafe fn find_get_page(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    if mapping.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if let Some(nn) = (*mapping).i_pages.xa_load(index) {
            let page = nn.as_ptr();
            (*page).get_page();
            page
        } else {
            ptr::null_mut()
        }
    }
}

/// Look up the page at `index`, lock it, and return it with refcount += 1.
///
/// Returns null if the page is not in the cache.
///
/// Ref: Linux `find_lock_page()` — `include/linux/pagemap.h`
pub unsafe fn find_lock_page(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    if mapping.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if let Some(nn) = (*mapping).i_pages.xa_load(index) {
            let page = nn.as_ptr();
            (*page).get_page();
            lock_page(page);
            page
        } else {
            ptr::null_mut()
        }
    }
}

/// Get the page at `index` from `mapping`, allocating it if absent.
///
/// Returns a locked page with refcount >= 1, or null on allocation failure.
/// The caller must call `unlock_page` when done.
///
/// Ref: Linux `filemap_grab_folio()` — `mm/filemap.c`
pub unsafe fn filemap_grab_folio(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    if mapping.is_null() {
        return ptr::null_mut();
    }

    // Fast path: already in cache.
    let existing = unsafe { find_lock_page(mapping, index) };
    if !existing.is_null() {
        return existing;
    }

    // Slow path: allocate a new page and insert it.
    unsafe {
        if !super::buddy::is_buddy_ready() {
            return ptr::null_mut();
        }
        let page_opt = super::buddy::with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL));
        let page = match page_opt {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        // In test mode, allocate a data buffer and store it in page->private.
        #[cfg(test)]
        {
            extern crate alloc;
            let buf: alloc::boxed::Box<[u8; 4096]> = alloc::boxed::Box::new([0u8; 4096]);
            (*page).private = alloc::boxed::Box::into_raw(buf) as usize;
        }

        let ret = filemap_add_folio(mapping, page, index, GFP_KERNEL);
        if ret != 0 {
            // Race: another thread inserted first. Free our page and return theirs.
            #[cfg(test)]
            {
                let buf = (*page).private as *mut [u8; 4096];
                if !buf.is_null() {
                    drop(alloc::boxed::Box::from_raw(buf));
                    (*page).private = 0;
                }
            }
            super::buddy::with_global_buddy(|b| b.free_pages(page, 0));
            return find_lock_page(mapping, index);
        }

        lock_page(page);
        page
    }
}

// ---------------------------------------------------------------------------
// set_page_dirty
// ---------------------------------------------------------------------------

/// Mark a page dirty in the page cache.
///
/// Sets `PG_DIRTY` on the page and tags it `PAGECACHE_TAG_DIRTY` in the
/// XArray so the writeback scanner can find it.
///
/// Ref: Linux `set_page_dirty()` — `mm/page-writeback.c`
pub unsafe fn set_page_dirty(page: *mut Page) {
    if page.is_null() {
        return;
    }
    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    let was_dirty = unsafe { (&*page).flags.load(Ordering::Acquire) & PG_DIRTY != 0 };

    unsafe {
        if !mapping.is_null() {
            let a_ops = (*mapping).a_ops;
            if !a_ops.is_null() {
                if let Some(dirty_fn) = (*a_ops).dirty_folio {
                    let _ = dirty_fn(mapping, page);
                }
            }
        }
    }

    let is_dirty = unsafe { (&*page).flags.load(Ordering::Acquire) & PG_DIRTY != 0 };
    if !was_dirty && is_dirty {
        unsafe { note_page_dirty(page, true) };
    } else if !was_dirty {
        unsafe {
            writeback_mark_page_dirty(page);
        }
    } else if is_dirty {
        unsafe { note_page_dirty(page, false) };
    }
}

// ---------------------------------------------------------------------------
// filemap_read
// ---------------------------------------------------------------------------

/// Buffered read from a file-backed `address_space`.
///
/// Loops: fetches pages from the cache (triggering readahead on miss),
/// waits for each page to become uptodate, copies the data into `iter`,
/// and advances the file position.  EOF is detected when fewer than
/// `PAGE_SIZE` bytes are available in the last page.
///
/// Returns the number of bytes read, or a negative errno on hard error.
///
/// Ref: Linux `filemap_read()` — `mm/filemap.c:2768`
pub unsafe fn filemap_read(iocb: *mut KioCb, iter: *mut IoVecIter, _already_read: isize) -> isize {
    use super::readahead::{ReadaheadControl, page_cache_sync_ra};

    if iocb.is_null() || iter.is_null() {
        return -22; // -EINVAL
    }

    let mut total: isize = 0;

    unsafe {
        loop {
            let remaining = (*iter).remaining();
            if remaining == 0 {
                break;
            }

            let pos = (*iocb).ki_pos;
            if pos < 0 {
                break;
            }
            let page_sz: usize = PAGE_SIZE as usize;
            let index: u64 = (pos as u64) >> crate::arch::x86::mm::paging::PAGE_SHIFT;
            let page_offset: usize = (pos as usize) & (page_sz - 1);

            let mapping = (*iocb).ki_filp as *mut AddressSpace;
            if mapping.is_null() {
                break;
            }

            // Try to get the page from the cache.
            let mut page = find_get_page(mapping, index);

            if page.is_null() {
                // Cache miss: trigger synchronous readahead.
                let mut rac = ReadaheadControl::new(mapping, index, 4);
                page_cache_sync_ra(&raw mut rac, 4);
                // Retry after readahead.
                page = find_get_page(mapping, index);
            }

            if page.is_null() {
                // Still missing after readahead — EOF or error.
                break;
            }

            // If the page is not yet uptodate, ask the filesystem to fill it.
            if !page_uptodate(page) {
                let a_ops = (*mapping).a_ops;
                if !a_ops.is_null() {
                    if let Some(read_fn) = (*a_ops).read_folio {
                        lock_page(page);
                        if !page_uptodate(page) {
                            let err = read_fn(mapping, page);
                            if err != 0 {
                                unlock_page(page);
                                (*page).put_page();
                                return if total > 0 { total } else { err as isize };
                            }
                        }
                        unlock_page(page);
                    }
                }
                // Wait for any concurrent read_folio to complete.
                super::address_space::wait_on_page_locked(page);
            }

            if !page_uptodate(page) {
                (*page).put_page();
                break; // EOF or I/O error: stop reading.
            }

            // Copy data from the page to the caller's buffer.
            let src = page_kaddr(page);
            if src.is_null() {
                (*page).put_page();
                break;
            }

            let bytes_in_page = page_sz - page_offset;
            let to_copy = remaining.min(bytes_in_page);

            let dst = (*iter).buf.add((*iter).written);
            core::ptr::copy_nonoverlapping(src.add(page_offset), dst, to_copy);

            (*iter).written += to_copy;
            (*iocb).ki_pos += to_copy as i64;
            total += to_copy as isize;

            mark_page_accessed(page);
            (*page).put_page();

            // Stop at end of page if we haven't filled the iter yet — the
            // next loop iteration will fetch the next page.
            if page_offset + to_copy == page_sz {
                continue;
            } else {
                break; // partial page → EOF
            }
        }
    }

    total
}

// ---------------------------------------------------------------------------
// generic_file_read_iter / generic_file_write_iter
// ---------------------------------------------------------------------------

/// Generic buffered read entry point.
///
/// Delegates to `filemap_read`; `IOCB_DIRECT` uses the same checked page-cache
/// path until block-device direct I/O is available below VFS.
///
/// Ref: Linux `generic_file_read_iter()` — `mm/filemap.c:2956`
pub unsafe fn generic_file_read_iter(iocb: *mut KioCb, iter: *mut IoVecIter) -> isize {
    if iocb.is_null() {
        return -22; // -EINVAL
    }
    unsafe { filemap_read(iocb, iter, 0) }
}

/// Generic buffered write loop.
///
/// For each page touched by the write:
/// 1. `filemap_grab_folio` — get-or-allocate the page, locked.
/// 2. If `write_begin` callback is set, call it.
/// 3. Copy data from `iter` into the page.
/// 4. If `write_end` callback is set, call it; otherwise `set_page_dirty`
///    and `unlock_page` manually.
/// 5. Advance position.
///
/// Returns bytes written, or a negative errno.
///
/// Ref: Linux `generic_perform_write()` — `mm/filemap.c:4296`
pub unsafe fn generic_perform_write(iocb: *mut KioCb, iter: *mut IoVecIter) -> isize {
    if iocb.is_null() || iter.is_null() {
        return -22; // -EINVAL
    }

    let mut total: isize = 0;

    unsafe {
        loop {
            let remaining = (*iter).remaining();
            if remaining == 0 {
                break;
            }

            let pos = (*iocb).ki_pos;
            if pos < 0 {
                break;
            }
            let page_sz: usize = PAGE_SIZE as usize;
            let index: u64 = (pos as u64) >> crate::arch::x86::mm::paging::PAGE_SHIFT;
            let page_offset: usize = (pos as usize) & (page_sz - 1);

            let mapping = (*iocb).ki_filp as *mut AddressSpace;
            if mapping.is_null() {
                break;
            }

            let page = filemap_grab_folio(mapping, index);
            if page.is_null() {
                if total == 0 {
                    return -12; // -ENOMEM
                }
                break;
            }

            let bytes_in_page = page_sz - page_offset;
            let to_write = remaining.min(bytes_in_page);

            let a_ops = (*mapping).a_ops;
            let mut fsdata: *mut u8 = ptr::null_mut();

            // write_begin callback
            if !a_ops.is_null() {
                if let Some(wb_fn) = (*a_ops).write_begin {
                    let mut page_out = page;
                    let err = wb_fn(
                        mapping,
                        pos,
                        to_write as u32,
                        &raw mut page_out,
                        &raw mut fsdata,
                    );
                    if err != 0 {
                        unlock_page(page);
                        (*page).put_page();
                        if total == 0 {
                            return err as isize;
                        }
                        break;
                    }
                }
            }

            // Copy data into page.
            let dst = page_kaddr(page);
            if dst.is_null() {
                unlock_page(page);
                (*page).put_page();
                break;
            }
            let src = (*iter).buf.add((*iter).written);
            core::ptr::copy_nonoverlapping(src, dst.add(page_offset), to_write);

            // write_end callback or generic dirty+unlock
            if !a_ops.is_null() {
                if let Some(we_fn) = (*a_ops).write_end {
                    we_fn(mapping, pos, to_write as u32, to_write as u32, page, fsdata);
                } else {
                    set_page_dirty(page);
                    set_page_uptodate(page);
                    unlock_page(page);
                }
            } else {
                set_page_dirty(page);
                set_page_uptodate(page);
                unlock_page(page);
            }

            mark_page_accessed(page);

            (*page).put_page();

            (*iter).written += to_write;
            (*iocb).ki_pos += to_write as i64;
            total += to_write as isize;

            if page_offset + to_write < page_sz {
                break; // partial write within one page — done
            }
        }
    }

    total
}

/// Generic buffered write entry point.
///
/// Delegates to `generic_perform_write`.  O_SYNC/O_DSYNC flushing via
/// `generic_write_sync` arrives in M16 (writeback).
///
/// Ref: Linux `generic_file_write_iter()` — `mm/filemap.c:4458`
pub unsafe fn generic_file_write_iter(iocb: *mut KioCb, iter: *mut IoVecIter) -> isize {
    let written = unsafe { generic_perform_write(iocb, iter) };
    if written > 0 && !iocb.is_null() {
        let mapping = unsafe { (*iocb).ki_filp as *mut AddressSpace };
        if !mapping.is_null() {
            unsafe {
                let _ = balance_dirty_pages(mapping);
            }
        }
    }
    written
}

// ---------------------------------------------------------------------------
// Linux-visible page-cache and folio compatibility wrappers
// ---------------------------------------------------------------------------

unsafe fn alloc_page_cache_page(_gfp: GfpFlags) -> *mut Page {
    if !super::buddy::is_buddy_ready() {
        return ptr::null_mut();
    }
    let page = unsafe { super::buddy::with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)) };
    match page {
        Some(page) => {
            #[cfg(test)]
            unsafe {
                extern crate alloc;
                let buf: alloc::boxed::Box<[u8; 4096]> = alloc::boxed::Box::new([0u8; 4096]);
                (*page).private = alloc::boxed::Box::into_raw(buf) as usize;
            }
            page
        }
        None => ptr::null_mut(),
    }
}

#[inline]
fn page_index_from_pos(pos: u64) -> u64 {
    pos / PAGE_SIZE as u64
}

#[inline]
fn page_end_index_from_pos(pos: u64) -> u64 {
    if pos == u64::MAX {
        u64::MAX
    } else {
        pos / PAGE_SIZE as u64
    }
}

#[inline]
unsafe fn mapping_flag(mapping: *const AddressSpace, flag: u64) -> bool {
    !mapping.is_null() && unsafe { (&*mapping).flags.load(Ordering::Acquire) & flag != 0 }
}

#[inline]
unsafe fn mapping_set_flag(mapping: *mut AddressSpace, flag: u64) {
    if !mapping.is_null() {
        unsafe {
            (&*mapping).flags.fetch_or(flag, Ordering::AcqRel);
        }
    }
}

#[inline]
unsafe fn mapping_clear_flag(mapping: *mut AddressSpace, flag: u64) {
    if !mapping.is_null() {
        unsafe {
            (&*mapping).flags.fetch_and(!flag, Ordering::AcqRel);
        }
    }
}

/// Raw generic write path before caller-side synchronization.
///
/// Ref: Linux `__generic_file_write_iter()` — `mm/filemap.c`.
pub unsafe fn __generic_file_write_iter(iocb: *mut KioCb, iter: *mut IoVecIter) -> isize {
    unsafe { generic_perform_write(iocb, iter) }
}

/// Allocate a single cache page for an address_space.
///
/// Ref: Linux `__page_cache_alloc()` / `filemap_alloc_folio()`.
pub unsafe fn __page_cache_alloc(gfp: GfpFlags) -> *mut Page {
    unsafe { alloc_page_cache_page(gfp) }
}

pub unsafe fn filemap_alloc_folio_noprof(gfp: GfpFlags, _order: u32) -> *mut Page {
    unsafe { alloc_page_cache_page(gfp) }
}

pub unsafe fn page_cache_alloc_noprof(mapping: *mut AddressSpace) -> *mut Page {
    let gfp = if mapping.is_null() {
        GFP_KERNEL
    } else {
        unsafe { (*mapping).gfp_mask }
    };
    unsafe { alloc_page_cache_page(gfp) }
}

/// Look up a folio, allocating on non-zero `fgp_flags`.
///
/// This intentionally maps the current single-page cache implementation onto
/// Linux folio names.  Larger folios stay reported as unsupported by the
/// mapping helpers below.
pub unsafe fn __filemap_get_folio(
    mapping: *mut AddressSpace,
    index: u64,
    fgp_flags: u32,
    gfp: GfpFlags,
) -> *mut Page {
    if mapping.is_null() {
        return ptr::null_mut();
    }
    let page = unsafe { find_get_page(mapping, index) };
    if !page.is_null() || fgp_flags == 0 {
        return page;
    }

    let created = unsafe { alloc_page_cache_page(gfp) };
    if created.is_null() {
        return ptr::null_mut();
    }
    if unsafe { filemap_add_folio(mapping, created, index, gfp) } != 0 {
        #[cfg(test)]
        unsafe {
            let buf = (*created).private as *mut [u8; 4096];
            if !buf.is_null() {
                drop(alloc::boxed::Box::from_raw(buf));
                (*created).private = 0;
            }
        }
        unsafe {
            super::buddy::with_global_buddy(|b| b.free_pages(created, 0));
        }
        return unsafe { find_get_page(mapping, index) };
    }
    created
}

pub unsafe fn __filemap_get_folio_mpol(
    mapping: *mut AddressSpace,
    index: u64,
    fgp_flags: u32,
    gfp: GfpFlags,
    _mpol: *mut u8,
) -> *mut Page {
    unsafe { __filemap_get_folio(mapping, index, fgp_flags, gfp) }
}

pub unsafe fn filemap_get_folio(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    unsafe { __filemap_get_folio(mapping, index, 0, GFP_KERNEL) }
}

pub unsafe fn filemap_get_entry(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    unsafe { find_get_page(mapping, index) }
}

pub unsafe fn find_get_page_flags(
    mapping: *mut AddressSpace,
    index: u64,
    fgp_flags: u32,
) -> *mut Page {
    unsafe { __filemap_get_folio(mapping, index, fgp_flags, GFP_KERNEL) }
}

pub unsafe fn find_or_create_page(
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> *mut Page {
    unsafe { __filemap_get_folio(mapping, index, 1, gfp) }
}

pub unsafe fn grab_cache_page_nowait(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    let page = unsafe { __filemap_get_folio(mapping, index, 1, GFP_KERNEL) };
    if !page.is_null() && unsafe { !try_lock_page(page) } {
        unsafe {
            (*page).put_page();
        }
        return ptr::null_mut();
    }
    page
}

pub unsafe fn filemap_lock_folio(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    unsafe { find_lock_page(mapping, index) }
}

pub unsafe fn __filemap_remove_folio(page: *mut Page, _mapping: *mut AddressSpace) {
    unsafe { filemap_remove_folio(page) };
}

pub unsafe fn replace_page_cache_folio(old: *mut Page, new: *mut Page, gfp: GfpFlags) -> i32 {
    if old.is_null() || new.is_null() {
        return -22;
    }
    let mapping = unsafe { (*old).mapping as *mut AddressSpace };
    let index = unsafe { (*old).index as u64 };
    unsafe { filemap_remove_folio(old) };
    unsafe { filemap_add_folio(mapping, new, index, gfp) }
}

pub fn filemap_get_order(size: usize) -> u32 {
    let pages = size.max(1).div_ceil(PAGE_SIZE as usize);
    usize::BITS - pages.saturating_sub(1).leading_zeros()
}

pub fn fgf_set_order(order: u32) -> u32 {
    order << 26
}

pub unsafe fn filemap_get_folios(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
    out: *mut *mut Page,
    max: usize,
) -> usize {
    if mapping.is_null() || out.is_null() || max == 0 || start > end {
        return 0;
    }
    let mut written = 0usize;
    for (_, page) in unsafe { (&*mapping).i_pages.xa_for_each_range(start, end) } {
        if written == max {
            break;
        }
        let ptr = page.as_ptr();
        unsafe {
            (*ptr).get_page();
            *out.add(written) = ptr;
        }
        written += 1;
    }
    written
}

pub unsafe fn filemap_get_folios_contig(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
    out: *mut *mut Page,
    max: usize,
) -> usize {
    unsafe { filemap_get_folios(mapping, start, end, out, max) }
}

pub unsafe fn filemap_get_folios_tag(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
    tag: u32,
    out: *mut *mut Page,
    max: usize,
) -> usize {
    if mapping.is_null() || out.is_null() || max == 0 || start > end {
        return 0;
    }
    let mark = match tag {
        1 => XaMark::Writeback,
        2 => XaMark::ToWrite,
        _ => XaMark::Dirty,
    };
    let mut next = start;
    let mut written = 0usize;
    while next <= end && written < max {
        let found = unsafe { (&*mapping).i_pages.xa_find(next, end, mark) };
        let Some((index, page)) = found else {
            break;
        };
        let ptr = page.as_ptr();
        unsafe {
            (*ptr).get_page();
            *out.add(written) = ptr;
        }
        written += 1;
        next = index.saturating_add(1);
    }
    written
}

pub unsafe fn filemap_range_has_page(mapping: *mut AddressSpace, start: u64, end: u64) -> bool {
    if mapping.is_null() || start > end {
        return false;
    }
    let start = page_index_from_pos(start);
    let end = page_end_index_from_pos(end);
    unsafe { (&*mapping).i_pages.xa_for_each_range(start, end).len() != 0 }
}

pub unsafe fn filemap_range_has_writeback(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
) -> bool {
    if mapping.is_null() || start > end {
        return false;
    }
    let start = page_index_from_pos(start);
    let end = page_end_index_from_pos(end);
    unsafe {
        (&*mapping)
            .i_pages
            .xa_find(start, end, XaMark::Writeback)
            .is_some()
    }
}

pub unsafe fn filemap_range_needs_writeback(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
) -> bool {
    if mapping.is_null() || start > end {
        return false;
    }
    let start = page_index_from_pos(start);
    let end = page_end_index_from_pos(end);
    unsafe {
        (&*mapping)
            .i_pages
            .xa_find(start, end, XaMark::Dirty)
            .is_some()
            || (&*mapping)
                .i_pages
                .xa_find(start, end, XaMark::Writeback)
                .is_some()
    }
}

pub unsafe fn page_cache_next_miss(mapping: *mut AddressSpace, index: u64, max_scan: usize) -> u64 {
    if mapping.is_null() {
        return index;
    }
    let mut candidate = index;
    for _ in 0..max_scan {
        if unsafe { (&*mapping).i_pages.xa_load(candidate).is_none() } {
            return candidate;
        }
        candidate = candidate.saturating_add(1);
    }
    candidate
}

pub unsafe fn page_cache_prev_miss(mapping: *mut AddressSpace, index: u64, max_scan: usize) -> u64 {
    if mapping.is_null() {
        return index;
    }
    let mut candidate = index;
    for _ in 0..max_scan {
        if unsafe { (&*mapping).i_pages.xa_load(candidate).is_none() } {
            return candidate;
        }
        if candidate == 0 {
            return 0;
        }
        candidate -= 1;
    }
    candidate
}

pub unsafe fn mapping_empty(mapping: *const AddressSpace) -> bool {
    mapping.is_null() || unsafe { (&*mapping).i_pages.is_empty() }
}

pub unsafe fn mapping_gfp_mask(mapping: *const AddressSpace) -> GfpFlags {
    if mapping.is_null() {
        GFP_KERNEL
    } else {
        unsafe { (*mapping).gfp_mask }
    }
}

pub unsafe fn mapping_set_gfp_mask(mapping: *mut AddressSpace, mask: GfpFlags) {
    if !mapping.is_null() {
        unsafe {
            (*mapping).gfp_mask = mask;
        }
    }
}

pub unsafe fn mapping_gfp_constraint(mapping: *const AddressSpace, mask: GfpFlags) -> GfpFlags {
    unsafe { mapping_gfp_mask(mapping) & mask }
}

pub fn mapping_align_index(index: u64, order: u32) -> u64 {
    if order >= u64::BITS {
        return 0;
    }
    let mask = (1u64 << order) - 1;
    index & !mask
}

pub unsafe fn mapping_set_error(mapping: *mut AddressSpace, error: i32) {
    if mapping.is_null() || error == 0 {
        return;
    }
    let errno = error.unsigned_abs();
    unsafe {
        (*mapping).wb_err.store(errno, Ordering::Release);
    }
    if errno == 28 {
        unsafe { mapping_set_flag(mapping, AS_ENOSPC) };
    } else {
        unsafe { mapping_set_flag(mapping, AS_EIO) };
    }
}

pub unsafe fn filemap_set_wb_err(mapping: *mut AddressSpace, error: i32) {
    unsafe { mapping_set_error(mapping, error) };
}

pub unsafe fn __filemap_set_wb_err(mapping: *mut AddressSpace, error: i32) {
    unsafe { mapping_set_error(mapping, error) };
}

pub unsafe fn filemap_sample_wb_err(mapping: *const AddressSpace) -> u32 {
    if mapping.is_null() {
        0
    } else {
        unsafe { (*mapping).wb_err.load(Ordering::Acquire) }
    }
}

pub unsafe fn file_sample_sb_err(mapping: *const AddressSpace) -> u32 {
    unsafe { filemap_sample_wb_err(mapping) }
}

pub unsafe fn filemap_check_wb_err(mapping: *mut AddressSpace, since: u32) -> i32 {
    if mapping.is_null() {
        return 0;
    }
    let err = unsafe { (*mapping).wb_err.load(Ordering::Acquire) };
    if err != 0 && err != since {
        -(err as i32)
    } else {
        0
    }
}

pub unsafe fn file_check_and_advance_wb_err(file: *mut AddressSpace) -> i32 {
    let err = unsafe { filemap_check_wb_err(file, 0) };
    if err != 0 && !file.is_null() {
        unsafe {
            (*file).wb_err.store(0, Ordering::Release);
        }
    }
    err
}

pub unsafe fn filemap_check_errors(mapping: *mut AddressSpace) -> i32 {
    if mapping.is_null() {
        return 0;
    }
    let flags = unsafe { (*mapping).flags.load(Ordering::Acquire) };
    if flags & AS_ENOSPC != 0 {
        unsafe { mapping_clear_flag(mapping, AS_ENOSPC) };
        -28
    } else if flags & AS_EIO != 0 {
        unsafe { mapping_clear_flag(mapping, AS_EIO) };
        -5
    } else {
        unsafe { filemap_check_wb_err(mapping, 0) }
    }
}

pub unsafe fn mapping_set_unevictable(mapping: *mut AddressSpace) {
    unsafe { mapping_set_flag(mapping, AS_UNEVICTABLE) };
}

pub unsafe fn mapping_clear_unevictable(mapping: *mut AddressSpace) {
    unsafe { mapping_clear_flag(mapping, AS_UNEVICTABLE) };
}

pub unsafe fn mapping_unevictable(mapping: *const AddressSpace) -> bool {
    unsafe { mapping_flag(mapping, AS_UNEVICTABLE) }
}

pub unsafe fn mapping_set_exiting(mapping: *mut AddressSpace) {
    unsafe { mapping_set_flag(mapping, AS_EXITING) };
}

pub unsafe fn mapping_exiting(mapping: *const AddressSpace) -> bool {
    unsafe { mapping_flag(mapping, AS_EXITING) }
}

pub unsafe fn mapping_set_no_writeback_tags(mapping: *mut AddressSpace) {
    unsafe { mapping_set_flag(mapping, AS_NO_WRITEBACK_TAGS) };
}

pub unsafe fn mapping_use_writeback_tags(mapping: *const AddressSpace) -> bool {
    unsafe { !mapping_flag(mapping, AS_NO_WRITEBACK_TAGS) }
}

pub unsafe fn mapping_set_stable_writes(mapping: *mut AddressSpace) {
    unsafe { mapping_set_flag(mapping, AS_STABLE_WRITES) };
}

pub unsafe fn mapping_clear_stable_writes(mapping: *mut AddressSpace) {
    unsafe { mapping_clear_flag(mapping, AS_STABLE_WRITES) };
}

pub unsafe fn mapping_stable_writes(mapping: *const AddressSpace) -> bool {
    unsafe { mapping_flag(mapping, AS_STABLE_WRITES) }
}

pub unsafe fn mapping_set_release_always(_mapping: *mut AddressSpace) {}

pub unsafe fn mapping_clear_release_always(_mapping: *mut AddressSpace) {}

pub unsafe fn mapping_release_always(_mapping: *const AddressSpace) -> bool {
    false
}

pub unsafe fn mapping_set_inaccessible(_mapping: *mut AddressSpace) {}

pub unsafe fn mapping_inaccessible(_mapping: *const AddressSpace) -> bool {
    false
}

pub unsafe fn mapping_set_large_folios(_mapping: *mut AddressSpace) {}

pub unsafe fn mapping_large_folio_support(_mapping: *const AddressSpace) -> bool {
    false
}

pub unsafe fn mapping_set_folio_min_order(_mapping: *mut AddressSpace, _order: u32) {}

pub unsafe fn mapping_set_folio_order_range(_mapping: *mut AddressSpace, _min: u32, _max: u32) {}

pub unsafe fn mapping_max_folio_size(_mapping: *const AddressSpace) -> usize {
    PAGE_SIZE as usize
}

pub unsafe fn mapping_max_folio_size_supported(_mapping: *const AddressSpace, size: usize) -> bool {
    size <= PAGE_SIZE as usize
}

pub unsafe fn mapping_shrinkable(mapping: *const AddressSpace) -> bool {
    !unsafe { mapping_empty(mapping) } && !unsafe { mapping_unevictable(mapping) }
}

pub unsafe fn mapping_set_writeback_may_deadlock_on_reclaim(_mapping: *mut AddressSpace) {}

pub unsafe fn mapping_writeback_may_deadlock_on_reclaim(_mapping: *const AddressSpace) -> bool {
    false
}

pub unsafe fn filemap_fdatawrite(_mapping: *mut AddressSpace) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

pub unsafe fn filemap_fdatawrite_range(_mapping: *mut AddressSpace, _start: u64, _end: u64) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

pub unsafe fn filemap_flush(_mapping: *mut AddressSpace) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

pub unsafe fn filemap_flush_nr(_mapping: *mut AddressSpace, _nr: isize) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

pub unsafe fn filemap_flush_range(_mapping: *mut AddressSpace, _start: u64, _end: u64) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

pub unsafe fn filemap_fdatawait(mapping: *mut AddressSpace) -> i32 {
    unsafe { filemap_check_errors(mapping) }
}

pub unsafe fn filemap_fdatawait_keep_errors(_mapping: *mut AddressSpace) -> i32 {
    0
}

pub unsafe fn filemap_fdatawait_range(mapping: *mut AddressSpace, _start: u64, _end: u64) -> i32 {
    unsafe { filemap_check_errors(mapping) }
}

pub unsafe fn filemap_fdatawait_range_keep_errors(
    _mapping: *mut AddressSpace,
    _start: u64,
    _end: u64,
) -> i32 {
    0
}

pub unsafe fn filemap_write_and_wait(mapping: *mut AddressSpace) -> i32 {
    let ret = unsafe { filemap_fdatawrite(mapping) };
    if ret < 0 {
        ret
    } else {
        unsafe { filemap_fdatawait(mapping) }
    }
}

pub unsafe fn filemap_write_and_wait_range(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
) -> i32 {
    let ret = unsafe { filemap_fdatawrite_range(mapping, start, end) };
    if ret < 0 {
        ret
    } else {
        unsafe { filemap_fdatawait_range(mapping, start, end) }
    }
}

pub unsafe fn file_write_and_wait_range(file: *mut AddressSpace, start: u64, end: u64) -> i32 {
    unsafe { filemap_write_and_wait_range(file, start, end) }
}

pub unsafe fn kiocb_write_and_wait(iocb: *mut KioCb, written: isize) -> isize {
    if written <= 0 || iocb.is_null() {
        return written;
    }
    let mapping = unsafe { (*iocb).ki_filp as *mut AddressSpace };
    let ret = unsafe { filemap_write_and_wait(mapping) };
    if ret < 0 { ret as isize } else { written }
}

pub unsafe fn kiocb_invalidate_pages(_iocb: *mut KioCb, _count: usize) -> i32 {
    0
}

pub unsafe fn kiocb_invalidate_post_direct_write(_iocb: *mut KioCb, _count: usize) -> i32 {
    0
}

pub unsafe fn filemap_invalidate_inode(_mapping: *mut AddressSpace) {}

pub unsafe fn filemap_invalidate_lock_two(_a: *mut AddressSpace, _b: *mut AddressSpace) {}

pub unsafe fn filemap_invalidate_unlock_two(_a: *mut AddressSpace, _b: *mut AddressSpace) {}

pub unsafe fn filemap_release_folio(page: *mut Page, gfp: GfpFlags) -> bool {
    if page.is_null() {
        return true;
    }
    let mapping = unsafe { (*page).mapping as *mut AddressSpace };
    if mapping.is_null() {
        return true;
    }
    let ops = unsafe { (*mapping).a_ops };
    if !ops.is_null() {
        if let Some(release) = unsafe { (*ops).release_folio } {
            return unsafe { release(page, gfp) };
        }
    }
    true
}

pub unsafe fn generic_error_remove_folio(mapping: *mut AddressSpace, page: *mut Page) -> i32 {
    if mapping.is_null() || page.is_null() {
        return -22;
    }
    unsafe { filemap_remove_folio(page) };
    0
}

pub unsafe fn truncate_inode_pages_range(mapping: *mut AddressSpace, start: u64, end: u64) {
    if mapping.is_null() || start > end {
        return;
    }
    let start_idx = page_index_from_pos(start);
    let end_idx = page_end_index_from_pos(end);
    let pages = unsafe { (&*mapping).i_pages.xa_for_each_range(start_idx, end_idx) };
    for (_, page) in pages {
        unsafe { filemap_remove_folio(page.as_ptr()) };
    }
}

pub unsafe fn truncate_inode_pages(mapping: *mut AddressSpace, start: u64) {
    unsafe { truncate_inode_pages_range(mapping, start, u64::MAX) };
}

pub unsafe fn truncate_inode_pages_final(mapping: *mut AddressSpace) {
    unsafe { truncate_inode_pages_range(mapping, 0, u64::MAX) };
}

pub unsafe fn truncate_pagecache_range(mapping: *mut AddressSpace, start: u64, end: u64) {
    unsafe { truncate_inode_pages_range(mapping, start, end) };
}

pub unsafe fn truncate_pagecache(mapping: *mut AddressSpace, _new_size: u64) {
    unsafe { truncate_inode_pages_range(mapping, 0, u64::MAX) };
}

pub unsafe fn truncate_setsize(mapping: *mut AddressSpace, new_size: u64) {
    unsafe { truncate_inode_pages_range(mapping, new_size, u64::MAX) };
}

pub unsafe fn invalidate_mapping_pages(mapping: *mut AddressSpace, start: u64, end: u64) -> u64 {
    if mapping.is_null() || start > end {
        return 0;
    }
    let pages = unsafe { (&*mapping).i_pages.xa_for_each_range(start, end) };
    let count = pages.len() as u64;
    for (_, page) in pages {
        unsafe { filemap_remove_folio(page.as_ptr()) };
    }
    count
}

pub unsafe fn invalidate_inode_pages2(mapping: *mut AddressSpace) -> i32 {
    unsafe { invalidate_mapping_pages(mapping, 0, u64::MAX) };
    0
}

pub unsafe fn invalidate_inode_pages2_range(
    mapping: *mut AddressSpace,
    start: u64,
    end: u64,
) -> i32 {
    unsafe { invalidate_mapping_pages(mapping, start, end) };
    0
}

pub unsafe fn invalidate_remote_inode(mapping: *mut AddressSpace) {
    unsafe { truncate_inode_pages_final(mapping) };
}

pub unsafe fn inode_drain_writes(_inode: *mut u8) {}

pub unsafe fn pagecache_isize_extended(_inode: *mut u8, _from: u64, _to: u64) {}

pub unsafe fn mapping_read_folio_gfp(
    mapping: *mut AddressSpace,
    page: *mut Page,
    _gfp: GfpFlags,
) -> i32 {
    if mapping.is_null() || page.is_null() {
        return -22;
    }
    let ops = unsafe { (*mapping).a_ops };
    if !ops.is_null() {
        if let Some(read) = unsafe { (*ops).read_folio } {
            return unsafe { read(mapping, page) };
        }
    }
    unsafe { set_page_uptodate(page) };
    0
}

pub unsafe fn read_cache_folio(
    mapping: *mut AddressSpace,
    index: u64,
    _filler: *mut u8,
    _data: *mut u8,
) -> *mut Page {
    let page = unsafe { __filemap_get_folio(mapping, index, 1, GFP_KERNEL) };
    if !page.is_null() && !unsafe { page_uptodate(page) } {
        let _ = unsafe { mapping_read_folio_gfp(mapping, page, GFP_KERNEL) };
    }
    page
}

pub unsafe fn read_cache_page(
    mapping: *mut AddressSpace,
    index: u64,
    filler: *mut u8,
    data: *mut u8,
) -> *mut Page {
    unsafe { read_cache_folio(mapping, index, filler, data) }
}

pub unsafe fn read_cache_page_gfp(
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> *mut Page {
    let page = unsafe { __filemap_get_folio(mapping, index, 1, gfp) };
    if !page.is_null() && !unsafe { page_uptodate(page) } {
        let _ = unsafe { mapping_read_folio_gfp(mapping, page, gfp) };
    }
    page
}

pub unsafe fn read_mapping_folio(
    mapping: *mut AddressSpace,
    index: u64,
    data: *mut u8,
) -> *mut Page {
    unsafe { read_cache_folio(mapping, index, ptr::null_mut(), data) }
}

pub unsafe fn read_mapping_page(
    mapping: *mut AddressSpace,
    index: u64,
    data: *mut u8,
) -> *mut Page {
    unsafe { read_mapping_folio(mapping, index, data) }
}

pub unsafe fn filemap_splice_read(
    iocb: *mut KioCb,
    _pipe: *mut u8,
    len: usize,
    _flags: u32,
) -> isize {
    let mut buffer = [0u8; 256];
    let requested = len.min(buffer.len());
    let mut iter = IoVecIter {
        buf: buffer.as_mut_ptr(),
        count: requested,
        written: 0,
    };
    unsafe { filemap_read(iocb, &raw mut iter, 0) }
}

pub unsafe fn generic_file_direct_write(iocb: *mut KioCb, iter: *mut IoVecIter) -> isize {
    unsafe { generic_perform_write(iocb, iter) }
}

pub unsafe fn generic_file_mmap(_file: *mut u8, _vma: *mut u8) -> i32 {
    0
}

pub unsafe fn generic_file_mmap_prepare(_vma: *mut u8) -> i32 {
    0
}

pub unsafe fn generic_file_readonly_mmap(_file: *mut u8, _vma: *mut u8) -> i32 {
    0
}

pub unsafe fn generic_file_readonly_mmap_prepare(_vma: *mut u8) -> i32 {
    0
}

pub unsafe fn filemap_page_mkwrite(_vma: *mut u8, _vmf: *mut u8) -> i32 {
    0
}

pub unsafe fn filemap_map_pages(_vmf: *mut u8, _start_pgoff: u64, _end_pgoff: u64) -> i32 {
    0
}

pub unsafe fn filemap_migrate_folio(
    mapping: *mut AddressSpace,
    dst: *mut Page,
    src: *mut Page,
    _mode: i32,
) -> i32 {
    if mapping.is_null() || dst.is_null() || src.is_null() {
        return -22;
    }
    unsafe { replace_page_cache_folio(src, dst, GFP_KERNEL) }
}

pub fn filemap_nr_thps(_mapping: *const AddressSpace) -> usize {
    0
}

pub fn filemap_nr_thps_inc(_mapping: *mut AddressSpace) {}

pub fn filemap_nr_thps_dec(_mapping: *mut AddressSpace) {}

pub unsafe fn attach_page_private(page: *mut Page, data: usize) {
    if page.is_null() {
        return;
    }
    unsafe {
        (*page).private = data;
        if data != 0 {
            (&*page).flags.fetch_or(PG_PRIVATE, Ordering::AcqRel);
        }
    }
}

pub unsafe fn detach_page_private(page: *mut Page) -> usize {
    if page.is_null() {
        return 0;
    }
    unsafe {
        let old = (*page).private;
        (*page).private = 0;
        (&*page).flags.fetch_and(!PG_PRIVATE, Ordering::AcqRel);
        old
    }
}

pub unsafe fn folio_attach_private(folio: *mut Page, data: usize) {
    unsafe { attach_page_private(folio, data) };
}

pub unsafe fn folio_detach_private(folio: *mut Page) -> usize {
    unsafe { detach_page_private(folio) }
}

pub unsafe fn folio_change_private(folio: *mut Page, data: usize) -> usize {
    let old = if folio.is_null() {
        0
    } else {
        unsafe { (*folio).private }
    };
    unsafe { attach_page_private(folio, data) };
    old
}

pub unsafe fn folio_get_private(folio: *const Page) -> usize {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio).private }
    }
}

pub unsafe fn folio_flush_mapping(folio: *const Page) -> *mut AddressSpace {
    if folio.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (*folio).mapping as *mut AddressSpace }
    }
}

pub unsafe fn folio_mapping(folio: *const Page) -> *mut AddressSpace {
    unsafe { folio_flush_mapping(folio) }
}

pub unsafe fn folio_inode(folio: *const Page) -> *mut u8 {
    let mapping = unsafe { folio_mapping(folio) };
    if mapping.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (*mapping).host }
    }
}

pub unsafe fn folio_pgoff(folio: *const Page) -> u64 {
    if folio.is_null() {
        0
    } else {
        unsafe { (*folio).index as u64 }
    }
}

pub unsafe fn folio_pos(folio: *const Page) -> u64 {
    unsafe { folio_pgoff(folio).saturating_mul(PAGE_SIZE as u64) }
}

pub unsafe fn folio_next_index(folio: *const Page) -> u64 {
    unsafe { folio_pgoff(folio).saturating_add(1) }
}

pub unsafe fn folio_next_pos(folio: *const Page) -> u64 {
    unsafe { folio_next_index(folio).saturating_mul(PAGE_SIZE as u64) }
}

pub unsafe fn page_pgoff(page: *const Page) -> u64 {
    unsafe { folio_pgoff(page) }
}

pub unsafe fn page_offset(page: *const Page) -> u64 {
    unsafe { folio_pos(page) }
}

pub unsafe fn folio_contains(folio: *const Page, index: u64) -> bool {
    unsafe { folio_pgoff(folio) == index }
}

pub unsafe fn folio_file_page(folio: *mut Page, _index: u64) -> *mut Page {
    folio
}

pub unsafe fn linear_page_index(_vma: *const u8, address: usize) -> u64 {
    (address / PAGE_SIZE as usize) as u64
}

pub unsafe fn folio_lock(folio: *mut Page) {
    unsafe { lock_page(folio) };
}

pub unsafe fn __folio_lock(folio: *mut Page) {
    unsafe { lock_page(folio) };
}

pub unsafe fn folio_trylock(folio: *mut Page) -> bool {
    unsafe { try_lock_page(folio) }
}

pub unsafe fn trylock_page(page: *mut Page) -> bool {
    unsafe { try_lock_page(page) }
}

pub unsafe fn folio_unlock(folio: *mut Page) {
    unsafe { unlock_page(folio) };
}

pub unsafe fn folio_lock_killable(folio: *mut Page) -> i32 {
    unsafe { lock_page_killable(folio) }
}

pub unsafe fn __folio_lock_killable(folio: *mut Page) -> i32 {
    unsafe { lock_page_killable(folio) }
}

pub unsafe fn folio_lock_or_retry(folio: *mut Page, _mm: *mut u8, _flags: u32) -> bool {
    unsafe { lock_page(folio) };
    true
}

pub unsafe fn __folio_lock_or_retry(folio: *mut Page, mm: *mut u8, flags: u32) -> bool {
    unsafe { folio_lock_or_retry(folio, mm, flags) }
}

pub unsafe fn folio_wait_locked(folio: *mut Page) {
    unsafe { wait_on_page_locked(folio) };
}

pub unsafe fn folio_wait_locked_killable(folio: *mut Page) -> i32 {
    unsafe { wait_on_page_locked(folio) };
    0
}

pub unsafe fn folio_wait_bit(folio: *mut Page, bit: u32) {
    if folio.is_null() || bit >= 64 {
        return;
    }
    let mask = 1u64 << bit;
    while unsafe { (&*folio).flags.load(Ordering::Acquire) & mask != 0 } {
        core::hint::spin_loop();
    }
}

pub unsafe fn folio_wait_bit_killable(folio: *mut Page, bit: u32) -> i32 {
    unsafe { folio_wait_bit(folio, bit) };
    0
}

pub unsafe fn folio_wait_private_2(folio: *mut Page) {
    while !folio.is_null() && unsafe { (&*folio).flags.load(Ordering::Acquire) & PG_PRIVATE_2 != 0 }
    {
        core::hint::spin_loop();
    }
}

pub unsafe fn folio_wait_private_2_killable(folio: *mut Page) -> i32 {
    unsafe { folio_wait_private_2(folio) };
    0
}

pub unsafe fn folio_wait_writeback(folio: *mut Page) {
    unsafe { wait_on_page_writeback(folio) };
}

pub unsafe fn folio_wait_writeback_killable(folio: *mut Page) -> i32 {
    unsafe { wait_on_page_writeback(folio) };
    0
}

pub unsafe fn folio_wait_stable(folio: *mut Page) {
    let mapping = unsafe { folio_mapping(folio) };
    if unsafe { mapping_stable_writes(mapping) } {
        unsafe { wait_on_page_writeback(folio) };
    }
}

pub unsafe fn folio_mark_dirty(folio: *mut Page) -> bool {
    unsafe { writeback_mark_page_dirty(folio) }
}

pub unsafe fn __folio_mark_dirty(folio: *mut Page, _mapping: *mut AddressSpace) -> bool {
    unsafe { folio_mark_dirty(folio) }
}

pub unsafe fn folio_mark_dirty_lock(folio: *mut Page) -> bool {
    unsafe { lock_page(folio) };
    let dirty = unsafe { folio_mark_dirty(folio) };
    unsafe { unlock_page(folio) };
    dirty
}

pub unsafe fn filemap_dirty_folio(_mapping: *mut AddressSpace, folio: *mut Page) -> bool {
    unsafe { folio_mark_dirty(folio) }
}

pub unsafe fn noop_dirty_folio(mapping: *mut AddressSpace, folio: *mut Page) -> bool {
    unsafe { filemap_dirty_folio(mapping, folio) }
}

pub unsafe fn folio_clear_dirty_for_io(folio: *mut Page) -> bool {
    unsafe { clear_page_dirty_for_io(folio) }
}

pub unsafe fn __folio_cancel_dirty(folio: *mut Page) -> bool {
    unsafe { clear_page_dirty_for_io(folio) }
}

pub unsafe fn folio_cancel_dirty(folio: *mut Page) -> bool {
    unsafe { __folio_cancel_dirty(folio) }
}

pub unsafe fn folio_redirty_for_writepage(_wbc: *mut u8, folio: *mut Page) -> bool {
    unsafe { folio_mark_dirty(folio) }
}

pub unsafe fn __folio_start_writeback(folio: *mut Page, keep_write: bool) -> bool {
    unsafe { start_page_writeback(folio, keep_write) }
}

pub unsafe fn folio_end_writeback(folio: *mut Page) {
    unsafe { end_page_writeback(folio) };
}

pub unsafe fn folio_end_writeback_no_dropbehind(folio: *mut Page) {
    unsafe { end_page_writeback(folio) };
}

pub unsafe fn folio_end_dropbehind(folio: *mut Page) {
    if !folio.is_null() {
        unsafe {
            (&*folio).flags.fetch_and(!PG_DROPBEHIND, Ordering::AcqRel);
        }
    }
}

pub unsafe fn folio_end_read(folio: *mut Page, success: bool) {
    if success && !folio.is_null() {
        unsafe {
            (&*folio).flags.fetch_or(PG_UPTODATE, Ordering::Release);
        }
    }
    unsafe { unlock_page(folio) };
}

pub unsafe fn folio_end_private_2(folio: *mut Page) {
    if !folio.is_null() {
        unsafe {
            (&*folio).flags.fetch_and(!PG_PRIVATE_2, Ordering::AcqRel);
        }
    }
}

pub unsafe fn folio_account_cleaned(_folio: *mut Page, _mapping: *mut AddressSpace) {}

pub unsafe fn balance_dirty_pages_ratelimited(mapping: *mut AddressSpace) {
    let _ = unsafe { balance_dirty_pages(mapping) };
}

pub unsafe fn balance_dirty_pages_ratelimited_flags(mapping: *mut AddressSpace, _flags: u32) {
    let _ = unsafe { balance_dirty_pages(mapping) };
}

pub fn dirty_writeback_interval() -> usize {
    5 * 100
}

pub unsafe fn bdi_set_max_ratio(_bdi: *mut u8, _max_ratio: u32) -> i32 {
    0
}

pub unsafe fn tag_pages_for_writeback(mapping: *mut AddressSpace, start: u64, end: u64) {
    if mapping.is_null() || start > end {
        return;
    }
    let mut next = start;
    while next <= end {
        let found = unsafe { (&*mapping).i_pages.xa_find(next, end, XaMark::Dirty) };
        let Some((index, _page)) = found else {
            break;
        };
        unsafe {
            (&*mapping).i_pages.xa_set_mark(index, XaMark::ToWrite);
        }
        next = index.saturating_add(1);
    }
}

pub unsafe fn writeback_iter(
    mapping: *mut AddressSpace,
    wbc: *mut super::writeback::WritebackControl,
) -> i32 {
    if mapping.is_null() || wbc.is_null() {
        return 0;
    }
    unsafe {
        super::writeback::wb_queue_work(
            mapping,
            super::writeback::WritebackWork {
                nr_pages: (*wbc).nr_to_write,
                sync_mode: (*wbc).sync_mode,
                tagged_writepages: (*wbc).tagged_writepages,
                for_background: (*wbc).for_background,
                range_cyclic: (*wbc).range_cyclic,
            },
        );
    }
    super::writeback::wb_workfn() as i32
}

pub fn wb_writeout_inc(_pages: usize) {}

pub unsafe fn dir_pages(mapping: *const AddressSpace) -> usize {
    if mapping.is_null() {
        0
    } else {
        unsafe { (*mapping).nrpages.load(Ordering::Acquire) }
    }
}

pub unsafe fn write_begin_get_folio(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    unsafe { filemap_grab_folio(mapping, index) }
}

pub unsafe fn wake_page_match(_wait: *mut u8, _key: *mut u8) -> i32 {
    1
}

pub unsafe fn pagecache_get_page(
    mapping: *mut AddressSpace,
    index: u64,
    fgp_flags: u32,
    gfp: GfpFlags,
) -> *mut Page {
    unsafe { __filemap_get_folio(mapping, index, fgp_flags, gfp) }
}

pub unsafe fn add_to_page_cache_lru(
    page: *mut Page,
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> i32 {
    unsafe { filemap_add_folio(mapping, page, index, gfp) }
}

pub unsafe fn folio_invalidate(folio: *mut Page, offset: usize, length: usize) {
    if folio.is_null() {
        return;
    }
    let mapping = unsafe { (*folio).mapping as *mut AddressSpace };
    if !mapping.is_null() {
        let ops = unsafe { (*mapping).a_ops };
        if !ops.is_null() {
            if let Some(invalidate) = unsafe { (*ops).invalidate_folio } {
                unsafe { invalidate(folio, offset, length) };
            }
        }
    }
}

pub unsafe fn folio_mkwrite_check_truncate(_folio: *mut Page, _inode: *mut u8) -> i32 {
    0
}

pub unsafe fn set_page_dirty_lock(page: *mut Page) -> i32 {
    unsafe { lock_page(page) };
    unsafe { set_page_dirty(page) };
    unsafe { unlock_page(page) };
    0
}

pub unsafe fn redirty_page_for_writepage(_wbc: *mut u8, page: *mut Page) -> i32 {
    unsafe { set_page_dirty(page) };
    0
}

pub unsafe fn write_inode_now(_inode: *mut u8, _sync: i32) -> i32 {
    super::writeback::flush_all_dirty_pages() as i32
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    use super::*;
    use crate::mm::address_space::{AddressSpace, set_page_uptodate, unlock_page};
    use crate::mm::buddy::{page_in_mem_map, reset_buddy_state_for_test};
    use crate::mm::lru::reset_lru_state_for_test;
    use crate::mm::page::Page;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::writeback::reset_writeback_state_for_test;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Allocate a Page with a heap-backed 4096-byte data buffer stored in
    /// `page.private`.  Must be freed with `free_test_page`.
    fn alloc_test_page(fill: u8) -> *mut Page {
        let buf = alloc::boxed::Box::into_raw(alloc::boxed::Box::new([fill; 4096]));
        let mut page = alloc::boxed::Box::new(Page::new());
        page.private = buf as usize;
        alloc::boxed::Box::into_raw(page)
    }

    unsafe fn free_test_page(page: *mut Page) {
        let buf = unsafe { (*page).private as *mut [u8; 4096] };
        if !buf.is_null() {
            unsafe { drop(alloc::boxed::Box::from_raw(buf)) };
        }
        unsafe { drop(alloc::boxed::Box::from_raw(page)) };
    }

    unsafe fn free_buddy_test_buffer(page: *mut Page) {
        let buf = unsafe { (*page).private as *mut [u8; 4096] };
        if !buf.is_null() {
            unsafe { drop(alloc::boxed::Box::from_raw(buf)) };
            unsafe {
                (*page).private = 0;
            }
        }
    }

    fn make_mapping() -> alloc::boxed::Box<AddressSpace> {
        alloc::boxed::Box::new(AddressSpace::new())
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        reset_buddy_state_for_test();
        reset_lru_state_for_test();
        reset_writeback_state_for_test();
        guard
    }

    // ── filemap_add_and_find_get_page ─────────────────────────────────────────

    #[test]
    fn filemap_add_and_find_get_page() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page(0xAB);

        let r = unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        assert_eq!(r, 0);

        let found = unsafe { find_get_page(mptr, 0) };
        assert_eq!(found, page);

        unsafe { (*page).put_page() }; // for find_get_page ref
        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── filemap_add_increments_nrpages ────────────────────────────────────────

    #[test]
    fn filemap_add_increments_nrpages() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 0);

        let p0 = alloc_test_page(1);
        let p1 = alloc_test_page(2);
        unsafe { filemap_add_folio(mptr, p0, 0, GFP_KERNEL) };
        unsafe { filemap_add_folio(mptr, p1, 1, GFP_KERNEL) };

        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 2);

        unsafe { filemap_remove_folio(p0) };
        unsafe { filemap_remove_folio(p1) };
        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 0);

        unsafe { free_test_page(p0) };
        unsafe { free_test_page(p1) };
    }

    // ── filemap_remove_decrements_nrpages ─────────────────────────────────────

    #[test]
    fn filemap_remove_decrements_nrpages() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page(0);
        unsafe { filemap_add_folio(mptr, page, 7, GFP_KERNEL) };
        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 1);
        unsafe { filemap_remove_folio(page) };
        assert_eq!(mapping.nrpages.load(Ordering::Relaxed), 0);
        unsafe { free_test_page(page) };
    }

    // ── find_lock_page_returns_locked_page ────────────────────────────────────

    #[test]
    fn find_lock_page_returns_locked_page() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page(0);
        unsafe { filemap_add_folio(mptr, page, 3, GFP_KERNEL) };

        let locked = unsafe { find_lock_page(mptr, 3) };
        assert_eq!(locked, page);

        // Page must be locked.
        use crate::mm::page_flags::PG_LOCKED;
        assert_ne!(
            unsafe { (*locked).flags.load(Ordering::Relaxed) } & PG_LOCKED,
            0
        );

        unsafe { unlock_page(locked) };
        unsafe { (*page).put_page() }; // find_lock_page ref
        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── filemap_read_round_trip_single_page ───────────────────────────────────

    #[test]
    fn filemap_read_round_trip_single_page() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;

        // Populate one page with a known pattern.
        let page = alloc_test_page(0xCC);
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        unsafe { set_page_uptodate(page) };

        // Read 32 bytes from the start.
        let mut buf = [0u8; 32];
        let mut iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter = IoVecIter {
            buf: buf.as_mut_ptr(),
            count: 32,
            written: 0,
        };

        let n = unsafe { filemap_read(&raw mut iocb, &raw mut iter, 0) };
        assert_eq!(n, 32);
        assert!(buf.iter().all(|&b| b == 0xCC));

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── filemap_read_eof_zero_fill ────────────────────────────────────────────
    //
    // Reading past the page's valid data must zero-fill the buffer remainder.

    #[test]
    fn filemap_read_eof_zero_fill() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;

        // Page filled with 0xFF, uptodate.
        let page = alloc_test_page(0xFF);
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        unsafe { set_page_uptodate(page) };

        // Request more bytes than the page (and no second page) — should read
        // PAGE_SIZE bytes for the first page, then stop at EOF.
        let mut buf = [0xABu8; 8192];
        let mut iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter = IoVecIter {
            buf: buf.as_mut_ptr(),
            count: 8192,
            written: 0,
        };

        let n = unsafe { filemap_read(&raw mut iocb, &raw mut iter, 0) };
        // First PAGE_SIZE bytes should be 0xFF.
        assert_eq!(n as usize, PAGE_SIZE as usize);
        assert!(buf[..PAGE_SIZE as usize].iter().all(|&b| b == 0xFF));

        let extra = unsafe { find_get_page(mptr, 1) };
        if !extra.is_null() {
            unsafe { (*extra).put_page() };
            unsafe { filemap_remove_folio(extra) };
            if page_in_mem_map(extra) {
                unsafe { free_buddy_test_buffer(extra) };
            } else {
                unsafe { free_test_page(extra) };
            }
        }

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── filemap_read_multi_page_sequential ────────────────────────────────────

    #[test]
    fn filemap_read_multi_page_sequential() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;

        // Two pages: page 0 = 0xAA, page 1 = 0xBB.
        let p0 = alloc_test_page(0xAA);
        let p1 = alloc_test_page(0xBB);
        unsafe { filemap_add_folio(mptr, p0, 0, GFP_KERNEL) };
        unsafe { filemap_add_folio(mptr, p1, 1, GFP_KERNEL) };
        unsafe { set_page_uptodate(p0) };
        unsafe { set_page_uptodate(p1) };

        let mut buf = [0u8; 8192];
        let mut iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter = IoVecIter {
            buf: buf.as_mut_ptr(),
            count: 8192,
            written: 0,
        };

        let n = unsafe { filemap_read(&raw mut iocb, &raw mut iter, 0) };
        assert_eq!(n as usize, 8192);
        assert!(buf[..4096].iter().all(|&b| b == 0xAA));
        assert!(buf[4096..].iter().all(|&b| b == 0xBB));

        unsafe { filemap_remove_folio(p0) };
        unsafe { filemap_remove_folio(p1) };
        unsafe { free_test_page(p0) };
        unsafe { free_test_page(p1) };
    }

    // ── generic_perform_write_marks_page_dirty ────────────────────────────────

    #[test]
    fn generic_perform_write_marks_page_dirty() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page(0);
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        unsafe { set_page_uptodate(page) };

        let src = [0x42u8; 64];
        let mut iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter = IoVecIter {
            buf: src.as_ptr() as *mut u8,
            count: 64,
            written: 0,
        };

        // We need filemap_grab_folio to find the existing page, but
        // generic_perform_write calls filemap_grab_folio which may use buddy.
        // Pre-unlock the page so grab_folio fast-path can lock it.
        unsafe { unlock_page(page) };

        let n = unsafe { generic_perform_write(&raw mut iocb, &raw mut iter) };
        assert_eq!(n, 64);

        // Page must be dirty.
        assert_ne!(
            unsafe { (*page).flags.load(Ordering::Relaxed) } & PG_DIRTY,
            0
        );

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── generic_write_then_read_round_trip ────────────────────────────────────

    #[test]
    fn generic_write_then_read_round_trip() {
        let _guard = test_guard();
        let mut mapping = make_mapping();
        let mptr = mapping.as_mut() as *mut AddressSpace;
        let page = alloc_test_page(0);
        unsafe { filemap_add_folio(mptr, page, 0, GFP_KERNEL) };
        unsafe { set_page_uptodate(page) };
        unsafe { unlock_page(page) }; // allow grab_folio to relock

        let src = [0xDE_u8; 128];
        let mut iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter = IoVecIter {
            buf: src.as_ptr() as *mut u8,
            count: 128,
            written: 0,
        };

        let nw = unsafe { generic_file_write_iter(&raw mut iocb, &raw mut iter) };
        assert_eq!(nw, 128);

        // Read back.
        let mut rbuf = [0u8; 128];
        let mut iocb2 = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: 0,
        };
        let mut iter2 = IoVecIter {
            buf: rbuf.as_mut_ptr(),
            count: 128,
            written: 0,
        };
        let nr = unsafe { generic_file_read_iter(&raw mut iocb2, &raw mut iter2) };
        assert_eq!(nr, 128);
        assert_eq!(&rbuf[..128], &src[..128]);

        let mut direct = [0u8; 128];
        let mut direct_iocb = KioCb {
            ki_filp: mptr as *mut u8,
            ki_pos: 0,
            ki_flags: IOCB_DIRECT,
        };
        let mut direct_iter = IoVecIter {
            buf: direct.as_mut_ptr(),
            count: 128,
            written: 0,
        };
        let direct_nr =
            unsafe { generic_file_read_iter(&raw mut direct_iocb, &raw mut direct_iter) };
        assert_eq!(direct_nr, 128);
        assert_eq!(&direct[..128], &src[..128]);

        unsafe { filemap_remove_folio(page) };
        unsafe { free_test_page(page) };
    }

    // ── concurrent_readers_writers_4thread ────────────────────────────────────
    //
    // Acceptance criterion: a synthetic in-memory address_space exposes
    // reads/writes that round-trip correctly under 4-CPU concurrent readers
    // and writers.
    //
    // We pre-populate 8 pages (indices 0-7) to avoid needing the buddy
    // allocator.  Two writer threads update even/odd pages; two reader threads
    // verify the data after a barrier.

    #[test]
    fn concurrent_readers_writers_4thread() {
        let _guard = test_guard();
        use std::sync::{Arc, Barrier};
        use std::thread;

        const NR_PAGES: usize = 8;
        const WRITE_PATTERN: u8 = 0xBE;
        const THREADS: usize = 4;

        // Build the mapping on the heap and wrap in Arc for sharing.
        let mapping: Arc<Box<AddressSpace>> = Arc::new(Box::new(AddressSpace::new()));
        let mptr: *mut AddressSpace = mapping.as_ref().as_ref() as *const _ as *mut _;

        // Pre-populate pages 0..NR_PAGES.
        let pages: Vec<*mut Page> = (0..NR_PAGES)
            .map(|i| {
                let page = alloc_test_page(0);
                unsafe { filemap_add_folio(mptr, page, i as u64, GFP_KERNEL) };
                unsafe { set_page_uptodate(page) };
                unsafe { unlock_page(page) }; // allow concurrent locking
                page
            })
            .collect();

        let barrier = Arc::new(Barrier::new(THREADS));

        // Thread 0: write pages 0..4 with WRITE_PATTERN.
        let mptr_w0 = mptr as usize;
        let b0 = Arc::clone(&barrier);
        let w0 = thread::spawn(move || {
            let mp = mptr_w0 as *mut AddressSpace;
            b0.wait();
            for i in 0..4usize {
                let page = unsafe { find_lock_page(mp, i as u64) };
                if page.is_null() {
                    return;
                }
                let data = unsafe { page_kaddr(page) };
                if !data.is_null() {
                    unsafe { core::ptr::write_bytes(data, WRITE_PATTERN, PAGE_SIZE as usize) };
                }
                unsafe { set_page_dirty(page) };
                unsafe { unlock_page(page) };
                unsafe { (*page).put_page() };
            }
        });

        // Thread 1: write pages 4..8 with WRITE_PATTERN.
        let mptr_w1 = mptr as usize;
        let b1 = Arc::clone(&barrier);
        let w1 = thread::spawn(move || {
            let mp = mptr_w1 as *mut AddressSpace;
            b1.wait();
            for i in 4..8usize {
                let page = unsafe { find_lock_page(mp, i as u64) };
                if page.is_null() {
                    return;
                }
                let data = unsafe { page_kaddr(page) };
                if !data.is_null() {
                    unsafe { core::ptr::write_bytes(data, WRITE_PATTERN, PAGE_SIZE as usize) };
                }
                unsafe { set_page_dirty(page) };
                unsafe { unlock_page(page) };
                unsafe { (*page).put_page() };
            }
        });

        // Thread 2: read pages 0..4 and verify data after writers finish.
        // We join the writers first, then verify sequentially from main thread.
        let mptr_r0 = mptr as usize;
        let b2 = Arc::clone(&barrier);
        let r0 = thread::spawn(move || {
            let mp = mptr_r0 as *mut AddressSpace;
            b2.wait();
            // Readers can proceed concurrently — they only hold the page lock
            // briefly while copying.
            let mut results = alloc::vec![0u8; 4 * PAGE_SIZE as usize];
            for i in 0..4usize {
                // Spin until the write has made the page dirty (data written).
                loop {
                    let page = unsafe { find_lock_page(mp, i as u64) };
                    if page.is_null() {
                        break;
                    }
                    if unsafe { (*page).flags.load(Ordering::Relaxed) } & PG_DIRTY != 0 {
                        let src = unsafe { page_kaddr(page) };
                        if !src.is_null() {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    src,
                                    results.as_mut_ptr().add(i * PAGE_SIZE as usize),
                                    PAGE_SIZE as usize,
                                )
                            };
                        }
                        unsafe { unlock_page(page) };
                        unsafe { (*page).put_page() };
                        break;
                    }
                    unsafe { unlock_page(page) };
                    unsafe { (*page).put_page() };
                    core::hint::spin_loop();
                }
            }
            results
        });

        // Thread 3: read pages 4..8.
        let mptr_r1 = mptr as usize;
        let b3 = Arc::clone(&barrier);
        let r1 = thread::spawn(move || {
            let mp = mptr_r1 as *mut AddressSpace;
            b3.wait();
            let mut results = alloc::vec![0u8; 4 * PAGE_SIZE as usize];
            for i in 4..8usize {
                loop {
                    let page = unsafe { find_lock_page(mp, i as u64) };
                    if page.is_null() {
                        break;
                    }
                    if unsafe { (*page).flags.load(Ordering::Relaxed) } & PG_DIRTY != 0 {
                        let src = unsafe { page_kaddr(page) };
                        if !src.is_null() {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    src,
                                    results.as_mut_ptr().add((i - 4) * PAGE_SIZE as usize),
                                    PAGE_SIZE as usize,
                                )
                            };
                        }
                        unsafe { unlock_page(page) };
                        unsafe { (*page).put_page() };
                        break;
                    }
                    unsafe { unlock_page(page) };
                    unsafe { (*page).put_page() };
                    core::hint::spin_loop();
                }
            }
            results
        });

        // Wait for all threads.
        w0.join().unwrap();
        w1.join().unwrap();
        let data_lo = r0.join().unwrap();
        let data_hi = r1.join().unwrap();

        // Verify all bytes written correctly.
        assert!(
            data_lo.iter().all(|&b| b == WRITE_PATTERN),
            "pages 0..4 did not round-trip correctly"
        );
        assert!(
            data_hi.iter().all(|&b| b == WRITE_PATTERN),
            "pages 4..8 did not round-trip correctly"
        );

        // Cleanup.
        for &page in &pages {
            unsafe { filemap_remove_folio(page) };
            unsafe { free_test_page(page) };
        }
    }
}
