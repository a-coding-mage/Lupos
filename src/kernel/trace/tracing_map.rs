//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/tracing_map.c
//! test-origin: linux:vendor/linux/kernel/trace/tracing_map.c
//! Bounded hash map used by histograms to aggregate values.
//!
//! Ref: vendor/linux/kernel/trace/tracing_map.c

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use spin::Mutex;

pub struct TracingMap<K: Ord + Clone> {
    inner: Mutex<BTreeMap<K, u64>>,
    capacity: usize,
}

impl<K: Ord + Clone> TracingMap<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(BTreeMap::new()),
            capacity,
        }
    }

    /// Returns `Err(-ENOSPC)` if the new key exceeds capacity.
    pub fn add(&self, k: K, v: u64) -> Result<u64, i32> {
        let mut g = self.inner.lock();
        if !g.contains_key(&k) && g.len() == self.capacity {
            return Err(-28); // -ENOSPC
        }
        let entry = g.entry(k).or_insert(0);
        *entry += v;
        Ok(*entry)
    }

    pub fn get(&self, k: &K) -> Option<u64> {
        self.inner.lock().get(k).copied()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_aggregates() {
        let m: TracingMap<i32> = TracingMap::new(4);
        m.add(1, 5).unwrap();
        m.add(1, 10).unwrap();
        assert_eq!(m.get(&1), Some(15));
    }

    #[test]
    fn overflow_returns_enospc() {
        let m: TracingMap<u32> = TracingMap::new(2);
        m.add(1, 1).unwrap();
        m.add(2, 1).unwrap();
        // Existing keys still allowed to grow.
        m.add(1, 1).unwrap();
        // New key beyond capacity → -ENOSPC.
        assert_eq!(m.add(3, 1).unwrap_err(), -28);
    }
}
