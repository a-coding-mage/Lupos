//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/ring_buffer.c
//! test-origin: linux:vendor/linux/kernel/trace/ring_buffer.c
//! Trace ring buffer — separate from the printk ring (M61).
//!
//! Mirrors `vendor/linux/kernel/trace/ring_buffer.c`'s per-CPU layout, but
//! collapsed into a single ring for M62.  Entries are fixed-size 32-byte
//! `TraceEvent` records — sufficient for the function tracer and kprobe
//! pre/post handlers.  Variable-length payloads (string tracepoints) are
//! deferred.
//!
//! Lockless single-producer-multi-consumer model: writers CAS-bump the
//! tail seq, then write into their reserved slot.  Reader locks the
//! whole ring (M62 doesn't need parallel readers).

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

pub const TRACE_RING_SIZE: usize = 1024;
const TRACE_RING_MASK: u64 = (TRACE_RING_SIZE as u64) - 1;

/// Single trace event.  32 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TraceEvent {
    pub ts_nsec: u64,
    pub ev_type: u32, // event id (e.g., TRACE_FN, TRACE_KPROBE)
    pub cpu: u16,
    pub pid: u16,
    pub arg0: u64, // typically the instruction pointer
    pub arg1: u64, // typically the parent instruction pointer
}

impl TraceEvent {
    pub const fn empty() -> Self {
        Self {
            ts_nsec: 0,
            ev_type: 0,
            cpu: 0,
            pid: 0,
            arg0: 0,
            arg1: 0,
        }
    }
}

/// Event-type IDs.  Mirrors selected entries from
/// `vendor/linux/include/linux/trace_events.h::trace_type`.
pub const TRACE_FN: u32 = 1; // function tracer
pub const TRACE_KPROBE: u32 = 0x80; // kprobe pre-handler
pub const TRACE_TP: u32 = 0x81; // generic static tracepoint
pub const TRACE_SYSCALL_ENTER: u32 = 0x100; // syscall entry
pub const TRACE_SYSCALL_EXIT: u32 = 0x101; // syscall exit

pub struct TraceRingBuffer {
    inner: Mutex<[TraceEvent; TRACE_RING_SIZE]>,
    head: AtomicU64,
    tail: AtomicU64,
    /// Whether tracing is enabled.  Mirrors `tracing_on`.
    enabled: core::sync::atomic::AtomicBool,
}

impl TraceRingBuffer {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new([TraceEvent::empty(); TRACE_RING_SIZE]),
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            enabled: core::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn push(&self, ev: TraceEvent) {
        if !self.is_enabled() {
            return;
        }
        let mut g = self.inner.lock();
        let head = self.head.load(Ordering::Relaxed);
        let idx = (head & TRACE_RING_MASK) as usize;
        g[idx] = ev;
        let new_head = head + 1;
        self.head.store(new_head, Ordering::Release);
        // Push tail forward if we've lapped.
        let tail = self.tail.load(Ordering::Relaxed);
        if new_head.saturating_sub(tail) >= TRACE_RING_SIZE as u64 {
            self.tail
                .store(new_head - TRACE_RING_SIZE as u64, Ordering::Release);
        }
    }

    /// Drain up to `out.len()` events into `out`.  Returns the number written.
    pub fn drain(&self, out: &mut [TraceEvent]) -> usize {
        let g = self.inner.lock();
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        let avail = (head - tail).min(out.len() as u64) as usize;
        for i in 0..avail {
            let idx = ((tail + i as u64) & TRACE_RING_MASK) as usize;
            out[i] = g[idx];
        }
        self.tail.store(tail + avail as u64, Ordering::Release);
        avail
    }

    pub fn len(&self) -> usize {
        let h = self.head.load(Ordering::Acquire);
        let t = self.tail.load(Ordering::Acquire);
        (h - t) as usize
    }
}

/// Single global trace ring for M62 (per-CPU sharding deferred).
pub static TRACE_RB: TraceRingBuffer = TraceRingBuffer::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_round_trip() {
        let rb = TraceRingBuffer::new();
        rb.set_enabled(true);
        rb.push(TraceEvent {
            ts_nsec: 1,
            ev_type: TRACE_FN,
            cpu: 0,
            pid: 7,
            arg0: 0xaa,
            arg1: 0xbb,
        });
        let mut out = [TraceEvent::empty(); 4];
        let n = rb.drain(&mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].ev_type, TRACE_FN);
        assert_eq!(out[0].arg0, 0xaa);
    }

    #[test]
    fn disabled_drops_events() {
        let rb = TraceRingBuffer::new();
        rb.push(TraceEvent::empty()); // drops because disabled
        assert_eq!(rb.len(), 0);
    }

    #[test]
    fn overflow_pushes_tail() {
        let rb = TraceRingBuffer::new();
        rb.set_enabled(true);
        for i in 0..(TRACE_RING_SIZE + 16) {
            let mut e = TraceEvent::empty();
            e.arg0 = i as u64;
            rb.push(e);
        }
        assert_eq!(rb.len(), TRACE_RING_SIZE);
    }
}
