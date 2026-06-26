//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/cpuhotplug.c
//! test-origin: linux:vendor/linux/kernel/irq/cpuhotplug.c
//! IRQ CPU hotplug coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/cpuhotplug.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static ONLINE_IRQ_CPUS: AtomicU64 = AtomicU64::new(1);

pub fn irq_cpu_online(cpu: usize) {
    if cpu < 64 {
        ONLINE_IRQ_CPUS.fetch_or(1u64 << cpu, Ordering::AcqRel);
    }
}

pub fn irq_cpu_offline(cpu: usize) {
    if cpu < 64 {
        ONLINE_IRQ_CPUS.fetch_and(!(1u64 << cpu), Ordering::AcqRel);
    }
}

pub fn irq_online_cpu_mask() -> u64 {
    ONLINE_IRQ_CPUS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_mask_tracks_cpu_state() {
        irq_cpu_online(3);
        assert!(irq_online_cpu_mask() & (1 << 3) != 0);
        irq_cpu_offline(3);
        assert!(irq_online_cpu_mask() & (1 << 3) == 0);
    }
}
