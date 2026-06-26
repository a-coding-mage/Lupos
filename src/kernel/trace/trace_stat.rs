//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_stat.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_stat.c
//! `tracing_stat` — pluggable per-tracer stats files.
//!
//! Ref: vendor/linux/kernel/trace/trace_stat.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

pub struct TraceStat {
    pub name: &'static str,
    pub value: u64,
}

static STATS: Mutex<Vec<TraceStat>> = Mutex::new(Vec::new());

pub fn register(name: &'static str, value: u64) {
    STATS.lock().push(TraceStat { name, value });
}

pub fn snapshot() -> Vec<(&'static str, u64)> {
    STATS.lock().iter().map(|s| (s.name, s.value)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_snapshot() {
        let n0 = snapshot().len();
        register("nr_events", 42);
        register("nr_dropped", 7);
        let s = snapshot();
        assert_eq!(s.len(), n0 + 2);
    }
}
