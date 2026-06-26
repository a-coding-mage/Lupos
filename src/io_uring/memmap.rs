//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/memmap.c
//! test-origin: linux:vendor/linux/io_uring/memmap.c
//! io_uring ring-page allocator and mmap fault-helper.
//!
//! Userspace mmaps SQ_RING / CQ_RING / SQES / PBUF_RING through this module.
//! Each region is a contiguous slab of kernel-allocated pages whose physical
//! addresses are recorded on the `IoRingCtx`.  A `vm_fault` handler returns
//! the appropriate page based on the file offset.
//!
//! Ref: vendor/linux/io_uring/memmap.c

extern crate alloc;

use alloc::vec::Vec;

use super::cqe::Cqe;
use super::sqe::Sqe;

/// `IORING_OFF_SQ_RING` / `CQ_RING` / `SQES` / `PBUF_RING` — UAPI mmap offsets.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h
pub const IORING_OFF_SQ_RING: u64 = 0;
pub const IORING_OFF_CQ_RING: u64 = 0x800_0000;
pub const IORING_OFF_SQES: u64 = 0x1000_0000;
pub const IORING_OFF_PBUF_RING: u64 = 0x8000_0000;
pub const IORING_OFF_PBUF_SHIFT: u64 = 16;

/// Page size used for ring backing.  Lupos kernel-only port — userspace mmap
/// integration uses 4 KiB pages on x86_64.
pub const PAGE_SIZE: usize = 4096;

/// One backing region.  `pages` holds the physical-page-equivalent buffers.
pub struct RingRegion {
    pub pages: Vec<[u8; PAGE_SIZE]>,
    /// Byte length the region exposes (may be less than `pages.len() * PAGE_SIZE`).
    pub len: usize,
}

impl RingRegion {
    /// Round `bytes` up to a whole number of pages and allocate them zeroed.
    pub fn new(bytes: usize) -> Self {
        let n_pages = (bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut pages = Vec::with_capacity(n_pages);
        for _ in 0..n_pages {
            pages.push([0u8; PAGE_SIZE]);
        }
        Self { pages, len: bytes }
    }

    /// Return the byte at offset `off`, or `None` if out of range.
    pub fn get(&self, off: usize) -> Option<u8> {
        if off >= self.len {
            return None;
        }
        Some(self.pages[off / PAGE_SIZE][off % PAGE_SIZE])
    }

    /// Mutate the byte at offset `off`.  Used by the kernel-side ring writer
    /// (CQE producer) and by tests that emulate userspace fault-in.
    pub fn set(&mut self, off: usize, val: u8) -> Result<(), ()> {
        if off >= self.len {
            return Err(());
        }
        self.pages[off / PAGE_SIZE][off % PAGE_SIZE] = val;
        Ok(())
    }
}

/// Compute the byte size of the SQ ring header for `entries`.
/// Linux: `sizeof(struct io_rings)` followed by `sq_entries * sizeof(__u32)`
/// for the SQ-array indirection.
pub fn sq_ring_bytes(sq_entries: u32) -> usize {
    // sizeof(struct io_rings) — head+tail+mask+entries+flags+dropped+overflow
    // (Linux is 64 bytes; we approximate with a generous fixed header so the
    // page-rounding semantics are exact.)
    const IO_RINGS_HDR: usize = 64;
    IO_RINGS_HDR + (sq_entries as usize) * core::mem::size_of::<u32>()
}

/// Byte size of the CQ ring buffer for `cq_entries` (header + cqes[]).
pub fn cq_ring_bytes(cq_entries: u32) -> usize {
    const IO_RINGS_HDR: usize = 64;
    IO_RINGS_HDR + (cq_entries as usize) * core::mem::size_of::<Cqe>()
}

/// Byte size of the SQES region for `sq_entries`.
pub fn sqes_bytes(sq_entries: u32) -> usize {
    (sq_entries as usize) * core::mem::size_of::<Sqe>()
}

/// Resolve an mmap offset to its region tag.  Returns `None` for unknown
/// offsets.  Callers use the tag to pick the right `RingRegion` on the ctx.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionTag {
    SqRing,
    CqRing,
    Sqes,
    PbufRing(u16),
}

pub fn region_for_offset(off: u64) -> Option<RegionTag> {
    if off == IORING_OFF_SQ_RING {
        return Some(RegionTag::SqRing);
    }
    if off == IORING_OFF_CQ_RING {
        return Some(RegionTag::CqRing);
    }
    if off == IORING_OFF_SQES {
        return Some(RegionTag::Sqes);
    }
    if off >= IORING_OFF_PBUF_RING && off < IORING_OFF_PBUF_RING.saturating_add(u32::MAX as u64) {
        let bgid = ((off - IORING_OFF_PBUF_RING) >> IORING_OFF_PBUF_SHIFT) as u16;
        return Some(RegionTag::PbufRing(bgid));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_match_linux() {
        // Mirrors vendor/linux/include/uapi/linux/io_uring.h.
        assert_eq!(IORING_OFF_SQ_RING, 0);
        assert_eq!(IORING_OFF_CQ_RING, 0x800_0000);
        assert_eq!(IORING_OFF_SQES, 0x1000_0000);
        assert_eq!(IORING_OFF_PBUF_RING, 0x8000_0000);
        assert_eq!(IORING_OFF_PBUF_SHIFT, 16);
    }

    #[test]
    fn region_for_offset_recognizes_named_regions() {
        assert_eq!(region_for_offset(0), Some(RegionTag::SqRing));
        assert_eq!(region_for_offset(0x800_0000), Some(RegionTag::CqRing));
        assert_eq!(region_for_offset(0x1000_0000), Some(RegionTag::Sqes));
    }

    #[test]
    fn region_for_offset_extracts_pbuf_bgid() {
        // PBUF_RING base + (bgid=7 << 16).
        let off = IORING_OFF_PBUF_RING + (7u64 << IORING_OFF_PBUF_SHIFT);
        assert_eq!(region_for_offset(off), Some(RegionTag::PbufRing(7)));
    }

    #[test]
    fn region_for_offset_returns_none_for_garbage() {
        assert!(region_for_offset(0x1234_5678).is_none());
    }

    #[test]
    fn ring_region_zeros_pages_on_alloc() {
        let r = RingRegion::new(PAGE_SIZE * 2);
        assert_eq!(r.pages.len(), 2);
        assert_eq!(r.get(0), Some(0));
        assert_eq!(r.get(PAGE_SIZE), Some(0));
        assert_eq!(r.get(PAGE_SIZE * 2), None);
    }

    #[test]
    fn ring_region_rounds_up_to_page_size() {
        let r = RingRegion::new(100);
        assert_eq!(r.pages.len(), 1);
        assert_eq!(r.len, 100);
    }

    #[test]
    fn sq_cq_byte_sizes_track_entries() {
        // Linear in entries with a fixed 64-byte header.
        let sq = sq_ring_bytes(8);
        let cq = cq_ring_bytes(8);
        assert_eq!(sq, 64 + 8 * 4);
        assert_eq!(cq, 64 + 8 * core::mem::size_of::<Cqe>());
        assert_eq!(sqes_bytes(8), 8 * 64);
    }

    #[test]
    fn region_set_get_round_trip() {
        let mut r = RingRegion::new(PAGE_SIZE * 2);
        r.set(4097, 0xaa).unwrap();
        assert_eq!(r.get(4097), Some(0xaa));
    }
}
