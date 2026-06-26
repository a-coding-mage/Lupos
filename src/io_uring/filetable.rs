//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/filetable.c
//! test-origin: linux:vendor/linux/io_uring/filetable.c
//! Fixed-file table — bitmap-backed slot allocator for `IORING_REGISTER_FILES`.
//!
//! Ref: vendor/linux/io_uring/filetable.c
//! Ref: vendor/linux/io_uring/filetable.h

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::fs::types::FileRef;

/// `IORING_FILE_INDEX_ALLOC` — caller wants the table to pick a free slot.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:141
pub const IORING_FILE_INDEX_ALLOC: u32 = u32::MAX;

/// `struct io_file_table` — Linux uses one `unsigned long *` bitmap plus a
/// parallel `struct io_rsrc_node *` array.  Lupos collapses them into a
/// single `Vec<Option<FileRef>>` with a packed bitmap for fast scan.
///
/// Ref: vendor/linux/io_uring/filetable.h::io_file_table
pub struct IoFileTable {
    /// `data.nodes` — one slot per registered fd.  `None` is an empty slot.
    pub slots: Vec<Option<FileRef>>,
    /// `bitmap` — `nr_bits = slots.len()`.  Bit set = slot occupied.
    bitmap: Vec<u64>,
    /// `alloc_hint` — last bit scan position.
    alloc_hint: u32,
    /// `file_alloc_start` — first slot eligible for auto-allocation.
    pub alloc_start: u32,
    /// `file_alloc_end` — one past the last slot eligible for auto-allocation.
    pub alloc_end: u32,
}

impl IoFileTable {
    /// Mirrors `io_alloc_file_tables`.  Returns `None` if `nr_files` is zero.
    pub fn new(nr_files: u32) -> Option<Self> {
        if nr_files == 0 {
            return None;
        }
        let n = nr_files as usize;
        Some(Self {
            slots: (0..n).map(|_| None).collect(),
            bitmap: vec![0u64; (n + 63) / 64],
            alloc_hint: 0,
            alloc_start: 0,
            alloc_end: nr_files,
        })
    }

    pub fn nr(&self) -> u32 {
        self.slots.len() as u32
    }

    /// `io_file_bitmap_set` — mark `index` occupied.
    pub fn bitmap_set(&mut self, index: u32) {
        let i = index as usize;
        self.bitmap[i / 64] |= 1u64 << (i % 64);
    }

    /// `io_file_bitmap_clear` — mark `index` free.
    pub fn bitmap_clear(&mut self, index: u32) {
        let i = index as usize;
        self.bitmap[i / 64] &= !(1u64 << (i % 64));
    }

    /// `io_file_bitmap_get` test helper — is bit `i` set?
    pub fn bitmap_is_set(&self, index: u32) -> bool {
        let i = index as usize;
        (self.bitmap[i / 64] >> (i % 64)) & 1 != 0
    }

    /// `io_file_table_set_alloc_range`.
    pub fn set_alloc_range(&mut self, off: u32, len: u32) {
        self.alloc_start = off;
        self.alloc_end = off + len;
        if self.alloc_hint < off || self.alloc_hint >= off + len {
            self.alloc_hint = off;
        }
    }

    /// `io_file_bitmap_get` — find first clear bit in `[alloc_hint, alloc_end)`
    /// then optionally wrap back to `alloc_start`.  Returns `-ENFILE` (-23) on
    /// exhaustion to match Linux.
    pub fn bitmap_get_free(&mut self) -> Result<u32, i32> {
        let mut nr = self.alloc_end;
        if self.alloc_hint < self.alloc_start || self.alloc_hint >= self.alloc_end {
            self.alloc_hint = self.alloc_start;
        }
        loop {
            // find_next_zero_bit(bitmap, nr, alloc_hint)
            let mut idx = self.alloc_hint;
            while idx < nr && self.bitmap_is_set(idx) {
                idx += 1;
            }
            if idx != nr {
                self.alloc_hint = idx + 1;
                return Ok(idx);
            }
            if self.alloc_hint == self.alloc_start {
                return Err(-23); // -ENFILE
            }
            nr = self.alloc_hint;
            self.alloc_hint = self.alloc_start;
        }
    }

    /// `io_install_fixed_file` (partial — without io_uring-fops detection,
    /// which we layer on top).  Returns `-EINVAL` for out-of-range slots.
    pub fn install_fixed(&mut self, slot: u32, file: FileRef) -> Result<(), i32> {
        if (slot as usize) >= self.slots.len() {
            return Err(-22); // -EINVAL
        }
        self.slots[slot as usize] = Some(file);
        self.bitmap_set(slot);
        Ok(())
    }

    /// `io_fixed_fd_remove` — drop file at `offset`.  Returns `-EBADF` if
    /// the slot was already empty.
    pub fn fixed_fd_remove(&mut self, offset: u32) -> Result<(), i32> {
        if (offset as usize) >= self.slots.len() {
            return Err(-22); // -EINVAL
        }
        if self.slots[offset as usize].is_none() {
            return Err(-9); // -EBADF
        }
        self.slots[offset as usize] = None;
        self.bitmap_clear(offset);
        Ok(())
    }

    /// `__io_fixed_fd_install` — accepts either an auto-alloc request
    /// (`IORING_FILE_INDEX_ALLOC`) or a 1-based explicit slot.
    pub fn fixed_fd_install(&mut self, file: FileRef, file_slot: u32) -> Result<u32, i32> {
        let alloc_slot = file_slot == IORING_FILE_INDEX_ALLOC;
        let slot = if alloc_slot {
            self.bitmap_get_free()?
        } else {
            // Linux uses 1-based file_slot here (see filetable.c:99).
            if file_slot == 0 {
                return Err(-22); // -EINVAL
            }
            file_slot - 1
        };
        self.install_fixed(slot, file)?;
        Ok(slot)
    }

    /// `io_rsrc_node_lookup` over the file slot — `None` if empty.
    pub fn lookup(&self, slot: u32) -> Option<&FileRef> {
        self.slots.get(slot as usize).and_then(|s| s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_file() -> FileRef {
        use crate::fs::dcache::d_alloc;
        use crate::fs::file::alloc_file;
        use crate::fs::ops::NOOP_FILE_OPS;

        let dentry = d_alloc("test");
        alloc_file(dentry, 0, 0, &NOOP_FILE_OPS)
    }

    #[test]
    fn ioring_file_index_alloc_matches_linux() {
        // Mirrors `#define IORING_FILE_INDEX_ALLOC (~0U)`.
        assert_eq!(IORING_FILE_INDEX_ALLOC, u32::MAX);
    }

    #[test]
    fn new_zero_size_returns_none() {
        assert!(IoFileTable::new(0).is_none());
    }

    #[test]
    fn bitmap_set_clear_round_trip() {
        let mut t = IoFileTable::new(128).unwrap();
        t.bitmap_set(5);
        assert!(t.bitmap_is_set(5));
        t.bitmap_clear(5);
        assert!(!t.bitmap_is_set(5));
    }

    #[test]
    fn bitmap_get_free_returns_lowest_clear_bit() {
        let mut t = IoFileTable::new(8).unwrap();
        t.bitmap_set(0);
        t.bitmap_set(1);
        let idx = t.bitmap_get_free().unwrap();
        // First free bit is 2.
        assert_eq!(idx, 2);
    }

    #[test]
    fn bitmap_get_free_exhausted_returns_enfile() {
        let mut t = IoFileTable::new(4).unwrap();
        for i in 0..4 {
            t.bitmap_set(i);
        }
        // -ENFILE per filetable.c:23,40.
        assert_eq!(t.bitmap_get_free().unwrap_err(), -23);
    }

    #[test]
    fn install_fixed_rejects_out_of_range_slot() {
        let mut t = IoFileTable::new(4).unwrap();
        assert_eq!(t.install_fixed(4, dummy_file()).unwrap_err(), -22);
    }

    #[test]
    fn install_then_remove_clears_bitmap() {
        let mut t = IoFileTable::new(4).unwrap();
        t.install_fixed(2, dummy_file()).unwrap();
        assert!(t.bitmap_is_set(2));
        t.fixed_fd_remove(2).unwrap();
        assert!(!t.bitmap_is_set(2));
    }

    #[test]
    fn remove_empty_slot_returns_ebadf() {
        let mut t = IoFileTable::new(4).unwrap();
        // -EBADF per filetable.c:137.
        assert_eq!(t.fixed_fd_remove(2).unwrap_err(), -9);
    }

    #[test]
    fn fixed_fd_install_alloc_picks_first_free() {
        let mut t = IoFileTable::new(4).unwrap();
        let slot = t
            .fixed_fd_install(dummy_file(), IORING_FILE_INDEX_ALLOC)
            .unwrap();
        assert_eq!(slot, 0);
        let slot2 = t
            .fixed_fd_install(dummy_file(), IORING_FILE_INDEX_ALLOC)
            .unwrap();
        assert_eq!(slot2, 1);
    }

    #[test]
    fn fixed_fd_install_explicit_slot_is_one_based() {
        // Linux file_slot is 1-based when not IORING_FILE_INDEX_ALLOC (see
        // filetable.c:99).
        let mut t = IoFileTable::new(4).unwrap();
        let slot = t.fixed_fd_install(dummy_file(), 3).unwrap();
        assert_eq!(slot, 2);
    }

    #[test]
    fn set_alloc_range_clamps_hint() {
        let mut t = IoFileTable::new(16).unwrap();
        t.alloc_hint = 12;
        t.set_alloc_range(2, 4);
        assert_eq!(t.alloc_start, 2);
        assert_eq!(t.alloc_end, 6);
        assert_eq!(t.alloc_hint, 2);
    }
}
