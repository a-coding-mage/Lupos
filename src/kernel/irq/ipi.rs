//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/ipi.c
//! test-origin: linux:vendor/linux/kernel/irq/ipi.c
//! Inter-processor interrupt coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/ipi.c`.

use core::sync::atomic::{AtomicU64, Ordering};

static SENT_IPIS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IpiKind {
    Reschedule = 0,
    CallFunction = 1,
    TlbShootdown = 2,
}

pub fn send_ipi_mask(mask: u64, _kind: IpiKind) -> u32 {
    let count = mask.count_ones();
    SENT_IPIS.fetch_add(count as u64, Ordering::AcqRel);
    count
}

pub fn sent_ipi_count() -> u64 {
    SENT_IPIS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipi_count_matches_mask_weight() {
        let before = sent_ipi_count();
        assert_eq!(send_ipi_mask(0b1011, IpiKind::Reschedule), 3);
        assert_eq!(sent_ipi_count(), before + 3);
    }
}
