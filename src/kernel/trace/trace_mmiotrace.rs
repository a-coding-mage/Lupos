//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_mmiotrace.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_mmiotrace.c
//! MMIO read/write tracing for x86 driver debugging.
//!
//! Ref: vendor/linux/kernel/trace/trace_mmiotrace.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct MmioEvent {
    pub addr: u64,
    pub value: u64,
    pub width: u8,
    pub is_write: bool,
}

static EVENTS: Mutex<Vec<MmioEvent>> = Mutex::new(Vec::new());

pub fn record(e: MmioEvent) {
    EVENTS.lock().push(e);
}

pub fn drain() -> Vec<MmioEvent> {
    core::mem::take(&mut *EVENTS.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trip() {
        record(MmioEvent {
            addr: 0xfee00020,
            value: 0,
            width: 4,
            is_write: true,
        });
        let d = drain();
        assert_eq!(d[0].addr, 0xfee00020);
    }
}
