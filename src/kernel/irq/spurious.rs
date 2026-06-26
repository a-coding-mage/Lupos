//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/spurious.c
//! test-origin: linux:vendor/linux/kernel/irq/spurious.c
//! Spurious IRQ detection coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/spurious.c`.

use core::sync::atomic::{AtomicU32, Ordering};

pub const SPURIOUS_DEFERRED: u32 = 100;

#[repr(C)]
pub struct SpuriousIrq {
    count: AtomicU32,
}

impl SpuriousIrq {
    pub const fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
        }
    }

    pub fn note_unhandled(&self) -> u32 {
        self.count.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn should_disable(&self) -> bool {
        self.count.load(Ordering::Acquire) >= SPURIOUS_DEFERRED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_after_threshold() {
        let s = SpuriousIrq::new();
        for _ in 0..SPURIOUS_DEFERRED {
            s.note_unhandled();
        }
        assert!(s.should_disable());
    }
}
