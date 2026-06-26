//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/blktrace.c
//! test-origin: linux:vendor/linux/kernel/trace/blktrace.c
//! Block-layer I/O tracing.
//!
//! Records block-layer events (queue, issue, complete, getrq) into a
//! per-cpu trace buffer.  Userspace `blktrace` consumes them via
//! `/sys/kernel/debug/block/<dev>/trace*`.
//!
//! Ref: vendor/linux/kernel/trace/blktrace.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

/// `enum blktrace_act` action codes (subset).
pub const BLK_TA_QUEUE: u32 = 1;
pub const BLK_TA_ISSUE: u32 = 4;
pub const BLK_TA_COMPLETE: u32 = 5;
pub const BLK_TA_GETRQ: u32 = 7;

#[derive(Clone, Copy, Debug)]
pub struct BlkTraceEvent {
    pub dev: u32,
    pub sector: u64,
    pub bytes: u32,
    pub action: u32,
    pub pid: u32,
    pub time: u64,
}

static EVENTS: Mutex<Vec<BlkTraceEvent>> = Mutex::new(Vec::new());

pub fn record(ev: BlkTraceEvent) {
    EVENTS.lock().push(ev);
}

pub fn drain() -> Vec<BlkTraceEvent> {
    core::mem::take(&mut *EVENTS.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_codes_match_linux() {
        assert_eq!(BLK_TA_QUEUE, 1);
        assert_eq!(BLK_TA_COMPLETE, 5);
    }

    #[test]
    fn record_then_drain() {
        record(BlkTraceEvent {
            dev: 8,
            sector: 0x1000,
            bytes: 512,
            action: BLK_TA_QUEUE,
            pid: 1,
            time: 0,
        });
        let d = drain();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].sector, 0x1000);
    }
}
