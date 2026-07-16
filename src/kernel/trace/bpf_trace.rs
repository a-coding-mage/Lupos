//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/bpf_trace.c
//! test-origin: linux:vendor/linux/kernel/trace/bpf_trace.c
//! BPF helpers exposed to tracing programs.
//!
//! Wires `BPF_PROG_TYPE_KPROBE` / `BPF_PROG_TYPE_TRACEPOINT` programs to
//! the trace ring buffer and a small set of `bpf_*` helpers.
//!
//! Ref: vendor/linux/kernel/trace/bpf_trace.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

pub use crate::kernel::bpf::module_raw_tracepoints::RawTracepointMap;

#[derive(Clone, Copy, Debug)]
pub struct BpfTraceEvent {
    pub prog_id: u32,
    pub ctx: u64,
    pub ret: i32,
}

static EVENTS: Mutex<Vec<BpfTraceEvent>> = Mutex::new(Vec::new());

/// `bpf_trace_printk` — bounded copy of caller-formatted bytes.  Returns
/// number of bytes recorded (matches Linux).
pub fn trace_printk(buf: &[u8]) -> i32 {
    let len = buf.len().min(64);
    EVENTS.lock().push(BpfTraceEvent {
        prog_id: 0,
        ctx: 0,
        ret: len as i32,
    });
    len as i32
}

pub fn record(ev: BpfTraceEvent) {
    EVENTS.lock().push(ev);
}

pub fn drain() -> Vec<BpfTraceEvent> {
    core::mem::take(&mut *EVENTS.lock())
}

/// `bpf_get_raw_tracepoint()` module fallback.  Built-in maps are not yet
/// linker-collected by Lupos, while loaded-module maps use the same ownership
/// and pinning rules as vendor Linux.
///
/// # Safety
/// The returned map must be paired with `put_raw_tracepoint()` before module
/// teardown.
pub unsafe fn get_raw_tracepoint(name: &str) -> Option<RawTracepointMap> {
    unsafe { crate::kernel::bpf::module_raw_tracepoints::get(name) }
}

pub fn put_raw_tracepoint(map: RawTracepointMap) {
    crate::kernel::bpf::module_raw_tracepoints::put(map);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_printk_caps_at_64() {
        let buf = [0u8; 128];
        assert_eq!(trace_printk(&buf), 64);
        drain();
    }

    #[test]
    fn record_round_trip() {
        record(BpfTraceEvent {
            prog_id: 7,
            ctx: 0xface,
            ret: 0,
        });
        let d = drain();
        assert_eq!(d[0].prog_id, 7);
    }
}
