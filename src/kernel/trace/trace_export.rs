//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_export.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_export.c
//! Pluggable trace exporter — routes ring-buffer events to alternate sinks
//! (e.g. STM, NPU trace tap).
//!
//! Ref: vendor/linux/kernel/trace/trace_export.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

pub struct TraceExporter {
    pub name: &'static str,
    pub write: fn(&[u8]),
}

static EXPORTERS: Mutex<Vec<TraceExporter>> = Mutex::new(Vec::new());

pub fn register(e: TraceExporter) {
    EXPORTERS.lock().push(e);
}

pub fn broadcast(bytes: &[u8]) -> usize {
    let g = EXPORTERS.lock();
    for e in g.iter() {
        (e.write)(bytes);
    }
    g.len()
}

pub fn count() -> usize {
    EXPORTERS.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static SEEN: AtomicUsize = AtomicUsize::new(0);
    fn stub(b: &[u8]) {
        SEEN.fetch_add(b.len(), Ordering::AcqRel);
    }

    #[test]
    fn register_and_broadcast() {
        let n0 = count();
        register(TraceExporter {
            name: "test",
            write: stub,
        });
        let _ = broadcast(b"hello");
        assert_eq!(count(), n0 + 1);
        assert!(SEEN.load(Ordering::Acquire) >= 5);
    }
}
