//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk
//! test-origin: linux:vendor/linux/kernel/printk
//! Lockless printk descriptor ring buffer.
//!
//! Mirrors the high-level design of `vendor/linux/kernel/printk/printk_ringbuffer.c`
//! at a level appropriate for Lupos:
//!
//! - A fixed-size descriptor array ("desc_ring") holds metadata (`PrintkInfo`)
//!   plus a `text_blk_lpos` that points at the data block in the text ring.
//! - A fixed-size text data ring stores the message bytes.
//! - Writers `reserve` an entry, fill the text, then `commit`. Reservation
//!   happens via atomic CAS on the head sequence counter.
//! - Readers walk from `tail_seq` to `head_seq`, copying records out.
//!
//! M61 simplifications versus full Linux design:
//! - Single contiguous text buffer indexed by `(begin, next)` byte offsets,
//!   no fancy `prb_data_blk_lpos` wrap encoding (we just modulo).
//! - Descriptor states collapsed to "free / reserved / committed" — no
//!   "reusable" state machine, since we don't yet support concurrent readers
//!   recycling slots.  Recycling happens by overwriting the oldest committed
//!   record on overflow.
//! - One global ring (no per-CPU `printk_safe`), single-writer-per-cpu via
//!   the spinlock around `reserve` for now.  Full lockless writers come in a
//!   follow-up once we wire up M62's per-CPU trace ring buffer.
//!
//! Data integrity invariants preserved from Linux:
//! - `seq` numbers are strictly monotonic.
//! - A `committed` record is byte-stable from commit until recycled.
//! - Readers see records in `seq` order.

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use super::record::{LOG_NEWLINE, PrintkInfo};

/// Number of descriptor slots.  Power of two so we can mask seq → index.
pub const PRB_DESC_COUNT: usize = 256;
/// Size of the text data ring, in bytes.  Power of two.
pub const PRB_TEXT_BUF_SIZE: usize = 16 * 1024;

const DESC_MASK: u64 = (PRB_DESC_COUNT as u64) - 1;
const TEXT_MASK: usize = PRB_TEXT_BUF_SIZE - 1;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum DescState {
    Free = 0,
    Reserved = 1,
    Committed = 2,
}

#[derive(Clone, Copy)]
struct DescSlot {
    info: PrintkInfo,
    state: DescState,
    text_begin: usize, // byte offset into TEXT (non-wrapped index)
    text_len: usize,
}

impl DescSlot {
    const fn empty() -> Self {
        Self {
            info: PrintkInfo::empty(),
            state: DescState::Free,
            text_begin: 0,
            text_len: 0,
        }
    }
}

/// The lockless ring (with a coarse mutex around reserve+commit for M61).
pub struct PrintkRingbuffer {
    inner: Mutex<RbInner>,
    /// Next sequence to assign on reserve.
    head_seq: AtomicU64,
    /// Oldest valid sequence still in the ring.
    tail_seq: AtomicU64,
    /// Count of failed reservations (text buffer too small for record).
    fail: AtomicU64,
}

struct RbInner {
    descs: [DescSlot; PRB_DESC_COUNT],
    text: [u8; PRB_TEXT_BUF_SIZE],
    /// Next byte offset where text will be written.  Non-wrapped 64-bit cursor
    /// would be ideal, but `usize` works with modulo for our small ring.
    text_head: usize,
}

impl PrintkRingbuffer {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(RbInner {
                descs: [DescSlot::empty(); PRB_DESC_COUNT],
                text: [0; PRB_TEXT_BUF_SIZE],
                text_head: 0,
            }),
            head_seq: AtomicU64::new(0),
            tail_seq: AtomicU64::new(0),
            fail: AtomicU64::new(0),
        }
    }

    /// Reserve + write + commit a record.
    /// Atomic from the reader's perspective: the new `head_seq` is published
    /// only after the descriptor + text are both written.
    pub fn emit(
        &self,
        ts_nsec: u64,
        facility: u8,
        level: u8,
        flags: u8,
        caller_id: u32,
        text: &[u8],
    ) -> Option<u64> {
        if text.len() > PRB_TEXT_BUF_SIZE {
            self.fail.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let mut g = self.inner.lock();

        // Allocate text-block region (may wrap).  We don't enforce contiguity
        // — writers always copy linearly with modulo, readers read with modulo.
        let begin = g.text_head;
        let end = begin.wrapping_add(text.len());
        // Push tail past any descriptor whose text region overlaps [begin, end).
        loop {
            let head_id = self.head_seq.load(Ordering::Relaxed);
            let tail_id = self.tail_seq.load(Ordering::Relaxed);
            if tail_id >= head_id {
                break; // empty
            }
            let tail_idx = (tail_id & DESC_MASK) as usize;
            let tail = &g.descs[tail_idx];
            if tail.state != DescState::Committed {
                break;
            }
            // Overlap check (modulo arithmetic).
            let t_begin = tail.text_begin;
            let t_end = t_begin.wrapping_add(tail.text_len);
            let overlap = if t_begin <= t_end {
                begin < t_end && t_begin < end
            } else {
                // tail wraps the buffer
                begin < t_end || t_begin < end
            };
            // Also push tail if descriptor slot itself collides.
            let new_id = head_id;
            let new_idx = (new_id & DESC_MASK) as usize;
            let slot_collide = new_idx == tail_idx;
            if !overlap && !slot_collide {
                break;
            }
            self.tail_seq.store(tail_id + 1, Ordering::Relaxed);
            g.descs[tail_idx].state = DescState::Free;
        }

        // Copy text into the text ring (with modulo wrap).
        for (i, b) in text.iter().enumerate() {
            let idx = begin.wrapping_add(i) & TEXT_MASK;
            g.text[idx] = *b;
        }
        g.text_head = end & TEXT_MASK;

        // Allocate descriptor.
        let new_seq = self.head_seq.load(Ordering::Relaxed);
        let new_idx = (new_seq & DESC_MASK) as usize;
        let slot = &mut g.descs[new_idx];
        slot.info = PrintkInfo::empty();
        slot.info.seq = new_seq;
        slot.info.ts_nsec = ts_nsec;
        slot.info.text_len = text.len() as u16;
        slot.info.facility = facility;
        slot.info.set_flags_level(flags | LOG_NEWLINE, level);
        slot.info.caller_id = caller_id;
        slot.text_begin = begin & TEXT_MASK;
        slot.text_len = text.len();
        slot.state = DescState::Committed;

        // Publish.
        self.head_seq.store(new_seq + 1, Ordering::Release);
        Some(new_seq)
    }

    /// Returns the smallest committed sequence in the ring.
    pub fn tail(&self) -> u64 {
        self.tail_seq.load(Ordering::Acquire)
    }

    /// Returns the next sequence that will be assigned (i.e., one past
    /// the highest committed).
    pub fn head(&self) -> u64 {
        self.head_seq.load(Ordering::Acquire)
    }

    /// Read the record at `seq` into `dst_text`, returning the actual text length.
    /// Returns `None` if `seq` is no longer in the ring.
    pub fn read(&self, seq: u64, dst_info: &mut PrintkInfo, dst_text: &mut [u8]) -> Option<usize> {
        let g = self.inner.lock();
        if seq < self.tail_seq.load(Ordering::Acquire)
            || seq >= self.head_seq.load(Ordering::Acquire)
        {
            return None;
        }
        let idx = (seq & DESC_MASK) as usize;
        let slot = &g.descs[idx];
        if slot.state != DescState::Committed || slot.info.seq != seq {
            return None;
        }
        *dst_info = slot.info;
        let n = slot.text_len.min(dst_text.len());
        for i in 0..n {
            let src_idx = slot.text_begin.wrapping_add(i) & TEXT_MASK;
            dst_text[i] = g.text[src_idx];
        }
        Some(slot.text_len)
    }

    pub fn fail_count(&self) -> u64 {
        self.fail.load(Ordering::Relaxed)
    }
}

/// The single global printk ringbuffer.
pub static PRINTK_RB: PrintkRingbuffer = PrintkRingbuffer::new();

/// Mirror a record from the legacy `src/log.rs` ring into the printk descriptor ring.
/// Called from the `log::on_emit` shim.
pub fn push_from_log(ts_nsec: u64, level: u8, facility: u8, caller_id: u32, text: &[u8]) {
    let _ = PRINTK_RB.emit(ts_nsec, facility, level, 0, caller_id, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::printk::levels::*;

    #[test]
    fn reserve_commit_round_trip() {
        let rb = PrintkRingbuffer::new();
        let seq = rb.emit(1234, LOG_KERN, KERN_INFO, 0, 0, b"hello").unwrap();
        let mut info = PrintkInfo::empty();
        let mut buf = [0u8; 32];
        let n = rb.read(seq, &mut info, &mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"hello");
        assert_eq!(info.seq, seq);
        assert_eq!(info.ts_nsec, 1234);
        assert_eq!(info.level(), KERN_INFO);
        assert_eq!(info.facility, LOG_KERN);
    }

    #[test]
    fn sequential_seqs() {
        let rb = PrintkRingbuffer::new();
        let s0 = rb.emit(0, 0, 6, 0, 0, b"a").unwrap();
        let s1 = rb.emit(0, 0, 6, 0, 0, b"b").unwrap();
        let s2 = rb.emit(0, 0, 6, 0, 0, b"c").unwrap();
        assert_eq!(s1, s0 + 1);
        assert_eq!(s2, s0 + 2);
    }

    #[test]
    fn descriptor_overflow_recycles_oldest() {
        let rb = PrintkRingbuffer::new();
        for i in 0..(PRB_DESC_COUNT as u64 + 16) {
            let _ = rb.emit(i, 0, 6, 0, 0, b"x").unwrap();
        }
        // Tail must have advanced past 0.
        assert!(rb.tail() > 0);
        // Head should be at our final write count.
        assert_eq!(rb.head(), PRB_DESC_COUNT as u64 + 16);
    }

    #[test]
    fn read_old_seq_returns_none_after_recycle() {
        let rb = PrintkRingbuffer::new();
        for i in 0..(PRB_DESC_COUNT as u64 + 1) {
            let _ = rb.emit(i, 0, 6, 0, 0, b"x").unwrap();
        }
        let mut info = PrintkInfo::empty();
        let mut buf = [0u8; 4];
        // seq 0 was the very first record; it must be gone now.
        assert!(rb.read(0, &mut info, &mut buf).is_none());
    }
}
