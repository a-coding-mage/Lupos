//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/alloc_cache.c
//! test-origin: linux:vendor/linux/io_uring/alloc_cache.c
//! Per-ring slab cache for short-lived io_uring objects (poll table entries,
//! async_data, etc.).  LIFO free-list with a hard size cap.
//!
//! Ref: vendor/linux/io_uring/alloc_cache.c
//! Ref: vendor/linux/io_uring/alloc_cache.h

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// Linux: `#define IO_ALLOC_CACHE_MAX 128`.
pub const IO_ALLOC_CACHE_MAX: u32 = 128;

/// `struct io_alloc_cache` — LIFO free-list bounded by `max_cached`.
///
/// In Linux the cache stores raw `void *` so any object type can be cached.
/// We expose a typed wrapper so callers don't have to cast.  Behavioral parity
/// (LIFO order, bounded size, `init_clear` zeroing) is preserved.
pub struct AllocCache<T> {
    entries: Vec<Box<T>>,
    max_cached: u32,
    /// Number of leading bytes to zero on `get()` to match `init_bytes`.
    init_clear: u32,
    _phantom: PhantomData<T>,
}

impl<T: Default> AllocCache<T> {
    /// Mirrors `io_alloc_cache_init`.  Returns `Err(())` if `max_nr` is zero
    /// (matches Linux's "returns true on failure" but with a Rust shape).
    pub fn init(max_nr: u32, init_bytes: u32) -> Result<Self, ()> {
        if max_nr == 0 {
            return Err(());
        }
        Ok(Self {
            entries: Vec::with_capacity(max_nr as usize),
            max_cached: max_nr,
            init_clear: init_bytes,
            _phantom: PhantomData,
        })
    }

    /// Mirrors `io_alloc_cache_put`.  Returns `true` if the object was
    /// cached, `false` if the cache was full (caller drops the object).
    pub fn put(&mut self, obj: Box<T>) -> bool {
        if (self.entries.len() as u32) < self.max_cached {
            self.entries.push(obj);
            true
        } else {
            false
        }
    }

    /// Mirrors `io_alloc_cache_get`.  LIFO — the last `put()` is the next
    /// `get()`.  Returns `None` when empty.
    pub fn get(&mut self) -> Option<Box<T>> {
        let mut obj = self.entries.pop()?;
        if self.init_clear > 0 {
            // Mirror `memset(entry, 0, cache->init_clear)`.  Bounded to
            // `init_clear` bytes so we match Linux even when T is larger.
            let clear = (self.init_clear as usize).min(core::mem::size_of::<T>());
            unsafe {
                core::ptr::write_bytes(obj.as_mut() as *mut T as *mut u8, 0, clear);
            }
        }
        Some(obj)
    }

    /// Mirrors `io_cache_alloc`.  Tries the free-list first, then heap-alloc.
    pub fn alloc(&mut self) -> Box<T> {
        self.get().unwrap_or_else(|| Box::new(T::default()))
    }

    /// Mirrors `io_cache_free`.  Caches if room, else drops.
    pub fn free(&mut self, obj: Box<T>) {
        let _ = self.put(obj);
    }

    /// Mirrors `io_alloc_cache_free` (drains the cache).
    pub fn drain(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> u32 {
        self.max_cached
    }
}

impl<T> Drop for AllocCache<T> {
    fn drop(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, PartialEq, Eq)]
    struct Item {
        x: u32,
        y: u32,
    }

    #[test]
    fn io_alloc_cache_max_matches_linux() {
        // Mirrors `#define IO_ALLOC_CACHE_MAX 128` in alloc_cache.h.
        assert_eq!(IO_ALLOC_CACHE_MAX, 128);
    }

    #[test]
    fn init_returns_err_for_zero_capacity() {
        // Linux returns `true` (failure) when kvmalloc_array(0, ...) fails;
        // we surface that as Err(()).
        let r: Result<AllocCache<Item>, ()> = AllocCache::init(0, 0);
        assert!(r.is_err());
    }

    #[test]
    fn put_and_get_are_lifo() {
        let mut c: AllocCache<Item> = AllocCache::init(4, 0).unwrap();
        c.put(Box::new(Item { x: 1, y: 1 }));
        c.put(Box::new(Item { x: 2, y: 2 }));
        c.put(Box::new(Item { x: 3, y: 3 }));
        assert_eq!(c.get().unwrap().x, 3);
        assert_eq!(c.get().unwrap().x, 2);
        assert_eq!(c.get().unwrap().x, 1);
        assert!(c.get().is_none());
    }

    #[test]
    fn put_rejects_when_full() {
        let mut c: AllocCache<Item> = AllocCache::init(2, 0).unwrap();
        assert!(c.put(Box::new(Item::default())));
        assert!(c.put(Box::new(Item::default())));
        // Third put exceeds max_cached — must return false (caller frees).
        assert!(!c.put(Box::new(Item::default())));
    }

    #[test]
    fn alloc_falls_back_to_heap_when_empty() {
        let mut c: AllocCache<Item> = AllocCache::init(4, 0).unwrap();
        let obj = c.alloc();
        // Empty cache produces a fresh default; not a panic.
        assert_eq!(*obj, Item::default());
    }

    #[test]
    fn init_clear_zeroes_leading_bytes_on_get() {
        let mut c: AllocCache<Item> = AllocCache::init(2, 4).unwrap();
        c.put(Box::new(Item {
            x: 0xdead,
            y: 0xbeef,
        }));
        let got = c.get().unwrap();
        // First 4 bytes (= x) zeroed; y untouched.
        assert_eq!(got.x, 0);
        assert_eq!(got.y, 0xbeef);
    }

    #[test]
    fn drain_clears_cache() {
        let mut c: AllocCache<Item> = AllocCache::init(4, 0).unwrap();
        c.put(Box::new(Item::default()));
        c.put(Box::new(Item::default()));
        c.drain();
        assert!(c.is_empty());
    }
}
