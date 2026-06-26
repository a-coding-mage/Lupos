//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_printk.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_printk.c
//! `trace_printk()` — printk-shaped emit into the trace ring buffer (not the
//! printk ring).  Useful for debugging without touching dmesg.
//!
//! Ref: vendor/linux/kernel/trace/trace_printk.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

static SINK: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn emit_str(s: &str) {
    SINK.lock().push(s.into());
}

pub fn drain() -> Vec<String> {
    core::mem::take(&mut *SINK.lock())
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn emit_then_drain() {
        emit_str("hello");
        emit_str("world");
        let d = drain();
        assert_eq!(d, alloc::vec!["hello".to_string(), "world".to_string()]);
    }
}
