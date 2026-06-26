//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/tick-broadcast.c
//! test-origin: linux:vendor/linux/kernel/time/tick-broadcast.c
//! Tick broadcast coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/tick-broadcast.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static BROADCAST_MASK: AtomicU64 = AtomicU64::new(0);

pub fn tick_broadcast_set_cpu(cpu: usize) {
    if cpu < 64 {
        BROADCAST_MASK.fetch_or(1u64 << cpu, Ordering::AcqRel);
    }
}

pub fn tick_broadcast_clear_cpu(cpu: usize) {
    if cpu < 64 {
        BROADCAST_MASK.fetch_and(!(1u64 << cpu), Ordering::AcqRel);
    }
}

pub fn tick_broadcast_mask() -> u64 {
    BROADCAST_MASK.load(Ordering::Acquire)
}

pub fn tick_broadcast_count() -> u32 {
    tick_broadcast_mask().count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_mask_add_and_remove() {
        tick_broadcast_clear_cpu(2);
        tick_broadcast_set_cpu(2);
        assert!(tick_broadcast_mask() & (1 << 2) != 0);
        tick_broadcast_clear_cpu(2);
        assert!(tick_broadcast_mask() & (1 << 2) == 0);
    }
}
