//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// XArray — sparse page-index with mark (tag) support.
///
/// This is the Lupos equivalent of Linux's `struct xarray` from `lib/xarray.c`.
/// The page cache uses this structure to index cached pages by file offset.
///
/// Our implementation is backed by a spinlock-protected `BTreeMap` rather than
/// Linux's radix-tree-derived XArray.  The semantics (operations, mark invariants,
/// ordering) are identical; the internal layout is Rust-idiomatic.
///
/// Three marks correspond to Linux's XA_MARK_0/1/2:
///   - `XaMark::Dirty`     → `PAGECACHE_TAG_DIRTY`
///   - `XaMark::Writeback` → `PAGECACHE_TAG_WRITEBACK`
///   - `XaMark::ToWrite`   → `PAGECACHE_TAG_TOWRITE`
///
/// Ref: Linux `lib/xarray.c`, `include/linux/xarray.h`
///      Linux `include/linux/fs.h:497-500` — PAGECACHE_TAG_* values
extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::ptr::NonNull;

use spin::Mutex;

use super::page::Page;

// ---------------------------------------------------------------------------
// XaMark — the three XArray mark (tag) types
// ---------------------------------------------------------------------------

/// Mark index for `XArray` — mirrors Linux `xa_mark_t` (XA_MARK_0/1/2).
///
/// Stored as a `BTreeSet<u64>` per mark; only indices with live entries
/// can be marked.
///
/// Ref: Linux `include/linux/xarray.h:68-77`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum XaMark {
    /// `PAGECACHE_TAG_DIRTY` (XA_MARK_0) — page contains dirty data.
    Dirty = 0,
    /// `PAGECACHE_TAG_WRITEBACK` (XA_MARK_1) — page is under writeback.
    Writeback = 1,
    /// `PAGECACHE_TAG_TOWRITE` (XA_MARK_2) — page is queued for writeback.
    ToWrite = 2,
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

struct XArrayInner {
    entries: BTreeMap<u64, NonNull<Page>>,
    marks: [BTreeSet<u64>; 3],
}

// Safety: NonNull<Page> is raw pointer; we guard all access behind the Mutex.
unsafe impl Send for XArrayInner {}

// ---------------------------------------------------------------------------
// XArray
// ---------------------------------------------------------------------------

/// Sparse page-index with mark support.
///
/// All operations acquire the internal spinlock, so the type is safe to share
/// across CPUs.  Callers are responsible for page reference counting.
///
/// Ref: Linux `struct xarray` — `include/linux/xarray.h:300-305`
pub struct XArray {
    inner: Mutex<XArrayInner>,
}

unsafe impl Send for XArray {}
unsafe impl Sync for XArray {}

impl XArray {
    /// Create an empty XArray.
    ///
    /// Ref: Linux `DEFINE_XARRAY()` / `xa_init()`
    pub fn new() -> Self {
        XArray {
            inner: Mutex::new(XArrayInner {
                entries: BTreeMap::new(),
                marks: [BTreeSet::new(), BTreeSet::new(), BTreeSet::new()],
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Core CRUD
    // -----------------------------------------------------------------------

    /// Load the page stored at `index`, or `None` if absent.
    ///
    /// Does NOT increment the page refcount.
    ///
    /// Ref: Linux `xa_load()` — `lib/xarray.c`
    pub fn xa_load(&self, index: u64) -> Option<NonNull<Page>> {
        self.inner.lock().entries.get(&index).copied()
    }

    /// Load a page and acquire its caller reference while the XArray lock is
    /// still held.  Linux's `filemap_get_entry()` performs the equivalent
    /// `folio_try_get_rcu()` before an entry can be removed and freed.
    pub fn xa_load_get(&self, index: u64) -> Option<NonNull<Page>> {
        let inner = self.inner.lock();
        let page = inner.entries.get(&index).copied()?;
        unsafe { page.as_ref() }.get_page();
        Some(page)
    }

    /// Insert only when `index` is empty, under one XArray critical section.
    /// Returns the supplied page on success or the existing entry on conflict.
    pub fn xa_insert(
        &self,
        index: u64,
        page: NonNull<Page>,
    ) -> Result<NonNull<Page>, NonNull<Page>> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner.entries.get(&index).copied() {
            Err(existing)
        } else {
            inner.entries.insert(index, page);
            Ok(page)
        }
    }

    /// Store `page` at `index`.  Returns the previous entry, if any.
    ///
    /// Callers must manage refcounts themselves (consistent with Linux).
    ///
    /// Ref: Linux `xa_store()` — `lib/xarray.c`
    pub fn xa_store(&self, index: u64, page: NonNull<Page>) -> Option<NonNull<Page>> {
        self.inner.lock().entries.insert(index, page)
    }

    /// Remove the entry at `index` and clear all its marks.
    ///
    /// Returns the removed page, if any.
    ///
    /// Ref: Linux `xa_erase()` — `lib/xarray.c`
    pub fn xa_erase(&self, index: u64) -> Option<NonNull<Page>> {
        let mut inner = self.inner.lock();
        let old = inner.entries.remove(&index);
        if old.is_some() {
            for mark_set in &mut inner.marks {
                mark_set.remove(&index);
            }
        }
        old
    }

    // -----------------------------------------------------------------------
    // Mark operations
    // -----------------------------------------------------------------------

    /// Set `mark` on the entry at `index`.
    ///
    /// No-op if no entry exists at `index` (consistent with Linux — marks
    /// are only meaningful on live entries).
    ///
    /// Ref: Linux `xa_set_mark()` — `lib/xarray.c`
    pub fn xa_set_mark(&self, index: u64, mark: XaMark) {
        let mut inner = self.inner.lock();
        if inner.entries.contains_key(&index) {
            inner.marks[mark as usize].insert(index);
        }
    }

    /// Clear `mark` on the entry at `index`.
    ///
    /// Ref: Linux `xa_clear_mark()` — `lib/xarray.c`
    pub fn xa_clear_mark(&self, index: u64, mark: XaMark) {
        self.inner.lock().marks[mark as usize].remove(&index);
    }

    /// Test whether `mark` is set on the entry at `index`.
    ///
    /// Ref: Linux `xa_get_mark()` — `lib/xarray.c`
    pub fn xa_get_mark(&self, index: u64, mark: XaMark) -> bool {
        self.inner.lock().marks[mark as usize].contains(&index)
    }

    // -----------------------------------------------------------------------
    // Range queries
    // -----------------------------------------------------------------------

    /// Find the first entry with `mark` set in the range `[start, end]`.
    ///
    /// Returns `(index, page)` of the first match, or `None`.
    ///
    /// Ref: Linux `xa_find()` — `lib/xarray.c`
    pub fn xa_find(&self, start: u64, end: u64, mark: XaMark) -> Option<(u64, NonNull<Page>)> {
        let inner = self.inner.lock();
        for &idx in inner.marks[mark as usize].range(start..=end) {
            if let Some(&page) = inner.entries.get(&idx) {
                return Some((idx, page));
            }
        }
        None
    }

    /// Collect all entries in `[start, end]` (inclusive), in ascending order.
    ///
    /// Equivalent to iterating `xas_for_each()` over the range.
    ///
    /// Ref: Linux `xas_for_each()` pattern — `lib/xarray.c`
    pub fn xa_for_each_range(&self, start: u64, end: u64) -> Vec<(u64, NonNull<Page>)> {
        self.inner
            .lock()
            .entries
            .range(start..=end)
            .map(|(&k, &v)| (k, v))
            .collect()
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.inner.lock().entries.len()
    }

    /// True if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().entries.is_empty()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::mm::page::Page;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    fn make_page(tag: u8) -> NonNull<Page> {
        let mut p = Box::new(Page::new());
        p.private = tag as usize;
        NonNull::new(Box::into_raw(p)).unwrap()
    }

    unsafe fn free_page(p: NonNull<Page>) {
        drop(unsafe { Box::from_raw(p.as_ptr()) });
    }

    // ── xarray_insert_and_load_roundtrip ─────────────────────────────────────

    #[test]
    fn xarray_insert_and_load_roundtrip() {
        let xa = XArray::new();
        let p0 = make_page(1);
        let p1 = make_page(2);

        assert!(xa.xa_load(0).is_none());
        assert!(xa.xa_store(0, p0).is_none()); // no previous entry
        assert!(xa.xa_store(1, p1).is_none());

        assert_eq!(xa.xa_load(0), Some(p0));
        assert_eq!(xa.xa_load(1), Some(p1));
        assert!(xa.xa_load(2).is_none());
        assert_eq!(xa.len(), 2);

        unsafe {
            free_page(p0);
            free_page(p1);
        }
    }

    // ── xarray_erase_clears_marks ─────────────────────────────────────────────

    #[test]
    fn xarray_erase_clears_marks() {
        let xa = XArray::new();
        let p = make_page(42);

        xa.xa_store(5, p);
        xa.xa_set_mark(5, XaMark::Dirty);
        assert!(xa.xa_get_mark(5, XaMark::Dirty));

        let removed = xa.xa_erase(5);
        assert_eq!(removed, Some(p));

        // Mark must be cleared after erase
        assert!(!xa.xa_get_mark(5, XaMark::Dirty));
        assert!(xa.xa_load(5).is_none());
        assert!(xa.is_empty());

        unsafe { free_page(p) };
    }

    // ── xarray_mark_set_get_clear ─────────────────────────────────────────────

    #[test]
    fn xarray_mark_set_get_clear() {
        let xa = XArray::new();
        let p = make_page(7);
        xa.xa_store(10, p);

        // Initially no marks
        assert!(!xa.xa_get_mark(10, XaMark::Dirty));
        assert!(!xa.xa_get_mark(10, XaMark::Writeback));
        assert!(!xa.xa_get_mark(10, XaMark::ToWrite));

        xa.xa_set_mark(10, XaMark::Dirty);
        assert!(xa.xa_get_mark(10, XaMark::Dirty));
        assert!(!xa.xa_get_mark(10, XaMark::Writeback)); // other marks unaffected

        xa.xa_set_mark(10, XaMark::Writeback);
        assert!(xa.xa_get_mark(10, XaMark::Writeback));

        xa.xa_clear_mark(10, XaMark::Dirty);
        assert!(!xa.xa_get_mark(10, XaMark::Dirty));
        assert!(xa.xa_get_mark(10, XaMark::Writeback)); // unaffected

        // xa_set_mark on missing index is a no-op
        xa.xa_set_mark(99, XaMark::Dirty);
        assert!(!xa.xa_get_mark(99, XaMark::Dirty));

        unsafe { free_page(p) };
    }

    // ── xarray_find_first_marked ──────────────────────────────────────────────

    #[test]
    fn xarray_find_first_marked() {
        let xa = XArray::new();
        let p2 = make_page(1);
        let p5 = make_page(2);
        let p8 = make_page(3);

        xa.xa_store(2, p2);
        xa.xa_store(5, p5);
        xa.xa_store(8, p8);

        xa.xa_set_mark(2, XaMark::Dirty);
        xa.xa_set_mark(8, XaMark::Dirty);

        // find in [0,10] — first dirty is index 2
        let found = xa.xa_find(0, 10, XaMark::Dirty);
        assert_eq!(found, Some((2, p2)));

        // find in [3,10] — first dirty is index 8
        let found = xa.xa_find(3, 10, XaMark::Dirty);
        assert_eq!(found, Some((8, p8)));

        // find in [6,7] — no dirty pages
        let found = xa.xa_find(6, 7, XaMark::Dirty);
        assert!(found.is_none());

        unsafe {
            free_page(p2);
            free_page(p5);
            free_page(p8);
        }
    }

    // ── xarray_xa_for_each_range_collects_correct_entries ────────────────────

    #[test]
    fn xarray_xa_for_each_range_collects_correct_entries() {
        let xa = XArray::new();
        let pages: Vec<NonNull<Page>> = (0u8..5).map(|i| make_page(i)).collect();

        for (i, &p) in pages.iter().enumerate() {
            xa.xa_store(i as u64, p);
        }

        // Full range
        let all = xa.xa_for_each_range(0, 4);
        assert_eq!(all.len(), 5);
        for (i, &(idx, page)) in all.iter().enumerate() {
            assert_eq!(idx, i as u64);
            assert_eq!(page, pages[i]);
        }

        // Sub-range [1,3]
        let sub = xa.xa_for_each_range(1, 3);
        assert_eq!(sub.len(), 3);
        assert_eq!(sub[0].0, 1);
        assert_eq!(sub[2].0, 3);

        // Empty range [6,10]
        let empty = xa.xa_for_each_range(6, 10);
        assert!(empty.is_empty());

        for p in pages {
            unsafe { free_page(p) };
        }
    }
}
