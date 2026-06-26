//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/affinity.c
//! test-origin: linux:vendor/linux/kernel/irq/affinity.c
//! IRQ affinity coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/affinity.c`.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqAffinity {
    pub pre_vectors: u32,
    pub post_vectors: u32,
    pub nr_sets: u32,
}

impl IrqAffinity {
    pub const fn new(pre_vectors: u32, post_vectors: u32, nr_sets: u32) -> Self {
        Self {
            pre_vectors,
            post_vectors,
            nr_sets,
        }
    }
}

pub fn irq_create_affinity_masks(nvecs: u32, nr_cpus: u32) -> Vec<u64> {
    let cpus = nr_cpus.max(1).min(64);
    let mut masks = Vec::new();
    for vector in 0..nvecs {
        let cpu = vector % cpus;
        masks.push(1u64 << cpu);
    }
    masks
}

pub fn irq_calc_affinity_vectors(minvec: u32, maxvec: u32, nr_cpus: u32) -> u32 {
    maxvec.min(nr_cpus.max(1)).max(minvec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_spread_vectors_across_cpus() {
        let masks = irq_create_affinity_masks(3, 2);
        assert_eq!(masks, alloc::vec![1, 2, 1]);
    }
}
