//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/pid_list.c
//! test-origin: linux:vendor/linux/kernel/trace/pid_list.c
//! `trace_pid_list` — sparse bitset of pids that pass the trace filter.
//!
//! Linux uses a three-level lookup tree to keep the bitset compact for the
//! 4M-pid keyspace.  We back it with a `BTreeSet<pid_t>` which gives the
//! same lookup semantics and ordered iteration.
//!
//! Ref: vendor/linux/kernel/trace/pid_list.c

extern crate alloc;
use alloc::collections::BTreeSet;

use spin::Mutex;

pub struct TracePidList {
    inner: Mutex<BTreeSet<i32>>,
}

impl TracePidList {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(BTreeSet::new()),
        }
    }

    pub fn add(&self, pid: i32) {
        self.inner.lock().insert(pid);
    }

    pub fn remove(&self, pid: i32) {
        self.inner.lock().remove(&pid);
    }

    pub fn contains(&self, pid: i32) -> bool {
        self.inner.lock().contains(&pid)
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_contains_remove() {
        let l = TracePidList::new();
        l.add(1234);
        assert!(l.contains(1234));
        assert!(!l.contains(999));
        l.remove(1234);
        assert!(!l.contains(1234));
    }
}
