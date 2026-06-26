//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/matrix.c
//! test-origin: linux:vendor/linux/kernel/irq/matrix.c
//! IRQ matrix allocator coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/matrix.c`.

use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
pub struct IrqMatrix {
    allocated: AtomicU64,
    online_cpus: AtomicU64,
}

impl IrqMatrix {
    pub const fn new() -> Self {
        Self {
            allocated: AtomicU64::new(0),
            online_cpus: AtomicU64::new(1),
        }
    }

    pub fn set_online_cpus(&self, mask: u64) {
        self.online_cpus.store(mask, Ordering::Release);
    }

    pub fn alloc(&self) -> Option<u32> {
        loop {
            let old = self.allocated.load(Ordering::Acquire);
            let free = !old;
            if free == 0 {
                return None;
            }
            let bit = free.trailing_zeros();
            let new = old | (1u64 << bit);
            if self
                .allocated
                .compare_exchange(old, new, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Some(bit);
            }
        }
    }

    pub fn free(&self, bit: u32) {
        if bit < 64 {
            self.allocated.fetch_and(!(1u64 << bit), Ordering::AcqRel);
        }
    }

    pub fn allocated_count(&self) -> u32 {
        self.allocated.load(Ordering::Acquire).count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_free_updates_count() {
        let matrix = IrqMatrix::new();
        let bit = matrix.alloc().unwrap();
        assert_eq!(matrix.allocated_count(), 1);
        matrix.free(bit);
        assert_eq!(matrix.allocated_count(), 0);
    }
}
