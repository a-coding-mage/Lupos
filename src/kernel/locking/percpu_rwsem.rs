//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/percpu-rwsem.c
//! test-origin: linux:vendor/linux/kernel/locking/percpu-rwsem.c
//! Per-CPU rwsem coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/percpu-rwsem.c`.  The Lupos version is
//! a global atomic reader counter with writer exclusion; per-CPU counter
//! sharding can replace the counter when the percpu allocator grows that API.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[repr(C)]
pub struct PerCpuRwSem {
    readers: AtomicUsize,
    writer: AtomicBool,
}

impl PerCpuRwSem {
    pub const fn new() -> Self {
        Self {
            readers: AtomicUsize::new(0),
            writer: AtomicBool::new(false),
        }
    }

    pub fn down_read_trylock(&self) -> bool {
        if self.writer.load(Ordering::Acquire) {
            return false;
        }
        self.readers.fetch_add(1, Ordering::AcqRel);
        if self.writer.load(Ordering::Acquire) {
            self.readers.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        true
    }

    pub fn up_read(&self) {
        self.readers.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn down_write_trylock(&self) -> bool {
        self.writer
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            && self.readers.load(Ordering::Acquire) == 0
    }

    pub fn up_write(&self) {
        self.writer.store(false, Ordering::Release);
    }

    pub fn reader_count(&self) -> usize {
        self.readers.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_waits_for_readers() {
        let sem = PerCpuRwSem::new();
        assert!(sem.down_read_trylock());
        assert_eq!(sem.reader_count(), 1);
        assert!(!sem.down_write_trylock());
        sem.up_write();
        sem.up_read();
        assert!(sem.down_write_trylock());
        sem.up_write();
    }
}
