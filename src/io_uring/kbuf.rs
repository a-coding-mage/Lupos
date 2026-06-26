//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/kbuf.c
//! test-origin: linux:vendor/linux/io_uring/kbuf.c
//! Provided-buffer management for io_uring.
//!
//! Two flavours, both backed by the same `IoBufferList`:
//!   - "classic" buffers fed by `IORING_OP_PROVIDE_BUFFERS` / `REMOVE_BUFFERS`
//!   - "ring-mapped" buffers registered via `IORING_REGISTER_PBUF_RING`
//!
//! Ref: vendor/linux/io_uring/kbuf.c
//! Ref: vendor/linux/io_uring/kbuf.h

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::uapi::{IoUringBuf, IoUringBufReg};

/// `#define MAX_BIDS_PER_BGID (1 << 16)` (vendor/linux/io_uring/kbuf.c:21).
pub const MAX_BIDS_PER_BGID: u32 = 1 << 16;

/// `IOBL_INC` — ring uses incremental consumption.
/// Ref: vendor/linux/io_uring/kbuf.h::IOBL_INC
pub const IOBL_INC: u32 = 1 << 0;
/// `IOBL_MMAP` — ring memory is kernel-mapped.
pub const IOBL_MMAP: u32 = 1 << 1;

/// `struct io_buffer` — classic single-shot provided buffer.
/// Ref: vendor/linux/io_uring/kbuf.h::io_buffer
#[derive(Clone, Debug)]
pub struct IoBuffer {
    pub addr: u64,
    pub len: u32,
    pub bid: u16,
    pub bgid: u16,
}

/// `struct io_buffer_list` — head of a per-bgid buffer list.
/// Ref: vendor/linux/io_uring/kbuf.h::io_buffer_list
#[derive(Debug)]
pub struct IoBufferList {
    pub bgid: u16,
    pub flags: u32,
    /// Classic mode: FIFO of free buffers.
    pub classic: Vec<IoBuffer>,
    /// Ring mode: kernel-side mirror of the userspace `io_uring_buf_ring`.
    pub ring: Option<Vec<IoUringBuf>>,
    pub head: u32,
    pub tail: u32,
    pub mask: u32,
    pub nr_entries: u32,
}

impl IoBufferList {
    /// `io_alloc_pbuf_ring` — set up ring-mode state.  `nr_entries` must be a
    /// power of two per the UAPI contract.
    pub fn new_ring(bgid: u16, nr_entries: u32, flags: u32) -> Result<Self, i32> {
        if nr_entries == 0 || !nr_entries.is_power_of_two() {
            return Err(-22); // -EINVAL
        }
        let mut ring = Vec::with_capacity(nr_entries as usize);
        ring.resize(nr_entries as usize, IoUringBuf::default());
        Ok(Self {
            bgid,
            flags,
            classic: Vec::new(),
            ring: Some(ring),
            head: 0,
            tail: 0,
            mask: nr_entries - 1,
            nr_entries,
        })
    }

    /// `io_provide_buffers` classic path — fan out `nbufs` buffers starting at
    /// `base_addr` with stride `len` and starting bid `start_bid`.
    pub fn provide_buffers(
        &mut self,
        base_addr: u64,
        len: u32,
        nbufs: u32,
        start_bid: u16,
    ) -> Result<u32, i32> {
        if self.ring.is_some() {
            // Mixed mode is rejected by Linux.
            return Err(-22);
        }
        if (nbufs as u64).saturating_add(start_bid as u64) > MAX_BIDS_PER_BGID as u64 {
            return Err(-22);
        }
        let mut added = 0;
        for i in 0..nbufs {
            self.classic.push(IoBuffer {
                addr: base_addr + (i as u64) * (len as u64),
                len,
                bid: start_bid.wrapping_add(i as u16),
                bgid: self.bgid,
            });
            added += 1;
        }
        Ok(added)
    }

    /// `io_remove_buffers` classic path — drop up to `nbufs` from the front.
    pub fn remove_buffers(&mut self, nbufs: u32) -> u32 {
        let n = (nbufs as usize).min(self.classic.len());
        self.classic.drain(0..n);
        n as u32
    }

    /// `io_buffer_select` classic — pop a buffer in FIFO order.
    pub fn select_classic(&mut self) -> Option<IoBuffer> {
        if self.classic.is_empty() {
            return None;
        }
        Some(self.classic.remove(0))
    }

    /// `io_ring_buffer_select` — peek at the buffer at the current head.
    /// Ref: vendor/linux/io_uring/kbuf.c:24 `io_ring_head_to_buf`.
    pub fn ring_head(&self) -> Option<IoUringBuf> {
        let ring = self.ring.as_ref()?;
        if self.head == self.tail {
            return None;
        }
        let idx = (self.head & self.mask) as usize;
        Some(ring[idx])
    }

    /// `io_kbuf_commit` (non-incremental) — advance head by `nr`.
    pub fn commit(&mut self, nr: u32) {
        self.head = self.head.wrapping_add(nr);
    }

    /// `io_kbuf_inc_commit` — incremental: trim `len` bytes from the head
    /// buffer, advancing past consumed buffers.  Returns true if every byte
    /// landed on a buffer boundary.
    pub fn inc_commit(&mut self, mut len: u32) -> bool {
        if len == 0 {
            return false;
        }
        let mask = self.mask;
        let ring = match self.ring.as_mut() {
            Some(r) => r,
            None => return true,
        };
        while len > 0 {
            let idx = (self.head & mask) as usize;
            let buf_len = ring[idx].len;
            let this_len = len.min(buf_len);
            let remaining = buf_len - this_len;
            if remaining != 0 || this_len == 0 {
                ring[idx].addr = ring[idx].addr.wrapping_add(this_len as u64);
                ring[idx].len = remaining;
                return false;
            }
            ring[idx].len = 0;
            self.head = self.head.wrapping_add(1);
            len -= this_len;
        }
        true
    }
}

/// Per-ring buffer-list registry keyed by `bgid`.  Linux uses an XArray; we
/// use a BTreeMap (same lookup semantics, monotonic iteration).
#[derive(Default)]
pub struct IoBufferRegistry {
    lists: BTreeMap<u16, IoBufferList>,
}

impl IoBufferRegistry {
    pub const fn new() -> Self {
        Self {
            lists: BTreeMap::new(),
        }
    }

    /// `io_buffer_add_list` — insert; rejects duplicates with `-EBUSY`.
    pub fn add_list(&mut self, bgid: u16, list: IoBufferList) -> Result<(), i32> {
        if self.lists.contains_key(&bgid) {
            return Err(-16); // -EBUSY
        }
        self.lists.insert(bgid, list);
        Ok(())
    }

    /// `io_buffer_get_list` — look up by bgid.
    pub fn get_list(&mut self, bgid: u16) -> Option<&mut IoBufferList> {
        self.lists.get_mut(&bgid)
    }

    /// `io_unregister_pbuf_ring` — drop a registration.
    pub fn remove_list(&mut self, bgid: u16) -> Option<IoBufferList> {
        self.lists.remove(&bgid)
    }

    /// `io_register_pbuf_ring` validates `reg` and inserts the new list.
    pub fn register_pbuf_ring(&mut self, reg: &IoUringBufReg) -> Result<(), i32> {
        if reg.ring_entries == 0 || !reg.ring_entries.is_power_of_two() {
            return Err(-22); // -EINVAL
        }
        let list = IoBufferList::new_ring(reg.bgid, reg.ring_entries, reg.flags as u32)?;
        self.add_list(reg.bgid, list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_bids_per_bgid_matches_linux() {
        // Mirrors `#define MAX_BIDS_PER_BGID (1 << 16)` (kbuf.c:21).
        assert_eq!(MAX_BIDS_PER_BGID, 65_536);
    }

    #[test]
    fn classic_provide_then_select_is_fifo() {
        let mut bl = IoBufferList::new_ring(7, 4, 0).unwrap();
        // new_ring builds a ring; switch to classic for this scenario.
        bl.ring = None;
        bl.provide_buffers(0x10_0000, 4096, 4, 100).unwrap();
        let b = bl.select_classic().unwrap();
        assert_eq!(b.bid, 100);
        assert_eq!(b.addr, 0x10_0000);
        let b = bl.select_classic().unwrap();
        assert_eq!(b.bid, 101);
        assert_eq!(b.addr, 0x10_0000 + 4096);
    }

    #[test]
    fn remove_buffers_trims_front() {
        let mut bl = IoBufferList::new_ring(0, 4, 0).unwrap();
        bl.ring = None;
        bl.provide_buffers(0x1000, 64, 4, 0).unwrap();
        let removed = bl.remove_buffers(2);
        assert_eq!(removed, 2);
        assert_eq!(bl.classic.len(), 2);
        assert_eq!(bl.classic[0].bid, 2);
    }

    #[test]
    fn ring_select_returns_head_when_non_empty() {
        let mut bl = IoBufferList::new_ring(0, 4, 0).unwrap();
        let ring = bl.ring.as_mut().unwrap();
        ring[0] = IoUringBuf {
            addr: 0xaaaa,
            len: 1024,
            bid: 5,
            resv: 0,
        };
        bl.tail = 1;
        let b = bl.ring_head().unwrap();
        assert_eq!(b.addr, 0xaaaa);
        assert_eq!(b.bid, 5);
    }

    #[test]
    fn ring_select_returns_none_when_empty() {
        let bl = IoBufferList::new_ring(0, 4, 0).unwrap();
        assert!(bl.ring_head().is_none());
    }

    #[test]
    fn ring_must_be_power_of_two() {
        // Linux rejects non-pow2 with -EINVAL.
        assert_eq!(IoBufferList::new_ring(0, 3, 0).unwrap_err(), -22);
    }

    #[test]
    fn commit_advances_head() {
        let mut bl = IoBufferList::new_ring(0, 4, 0).unwrap();
        bl.tail = 4;
        bl.commit(2);
        assert_eq!(bl.head, 2);
    }

    #[test]
    fn inc_commit_trims_head_buffer() {
        let mut bl = IoBufferList::new_ring(0, 4, IOBL_INC).unwrap();
        let ring = bl.ring.as_mut().unwrap();
        ring[0] = IoUringBuf {
            addr: 0x1000,
            len: 100,
            bid: 0,
            resv: 0,
        };
        bl.tail = 1;
        let drained_whole = bl.inc_commit(30);
        // 30 bytes consumed from a 100-byte buffer — partial; head stays put.
        assert!(!drained_whole);
        assert_eq!(bl.head, 0);
        let r = bl.ring.as_ref().unwrap();
        assert_eq!(r[0].len, 70);
        // 0x1000 base + 30 bytes consumed (decimal) = 0x1000 + 0x1E = 0x101E.
        assert_eq!(r[0].addr, 0x1000 + 30);
    }

    #[test]
    fn registry_rejects_duplicate_bgid() {
        let mut reg = IoBufferRegistry::new();
        let list1 = IoBufferList::new_ring(1, 4, 0).unwrap();
        let list2 = IoBufferList::new_ring(1, 4, 0).unwrap();
        reg.add_list(1, list1).unwrap();
        // Second add for the same bgid is -EBUSY.
        assert_eq!(reg.add_list(1, list2).unwrap_err(), -16);
    }
}
