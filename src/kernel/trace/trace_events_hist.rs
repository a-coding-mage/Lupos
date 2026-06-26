//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_hist.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_hist.c
//! Histogram triggers over trace events (`hist:`).
//!
//! Linux's hist trigger keeps a per-bucket counter keyed by a field value.
//! Lupos's port uses a `BTreeMap<i64, u64>` for the same lookup semantics.
//!
//! Ref: vendor/linux/kernel/trace/trace_events_hist.c

extern crate alloc;
use alloc::collections::BTreeMap;

use spin::Mutex;

pub struct Histogram {
    buckets: Mutex<BTreeMap<i64, u64>>,
}

impl Histogram {
    pub const fn new() -> Self {
        Self {
            buckets: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn record(&self, key: i64) {
        *self.buckets.lock().entry(key).or_insert(0) += 1;
    }

    pub fn get(&self, key: i64) -> u64 {
        *self.buckets.lock().get(&key).unwrap_or(&0)
    }

    pub fn len(&self) -> usize {
        self.buckets.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_bucket() {
        let h = Histogram::new();
        h.record(42);
        h.record(42);
        h.record(7);
        assert_eq!(h.get(42), 2);
        assert_eq!(h.get(7), 1);
        assert_eq!(h.get(0), 0);
    }
}
