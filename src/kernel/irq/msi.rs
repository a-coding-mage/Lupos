//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/msi.c
//! test-origin: linux:vendor/linux/kernel/irq/msi.c
//! MSI / MSI-X allocator (M37).
//!
//! Mirrors `vendor/linux/drivers/pci/msi/`.  M37 ships a linear pool of LAPIC
//! vectors above 0xC0; PCI integration arrives in M55.

use core::sync::atomic::{AtomicU32, Ordering};

/// MSI vector pool starts at 0xC0 (above the LAPIC IPI/Timer range).
const MSI_VECTOR_BASE: u32 = 0xC0;
const MSI_VECTOR_TOP: u32 = 0xFE; // leave 0xFE/0xFF for spurious + error

static NEXT_MSI: AtomicU32 = AtomicU32::new(MSI_VECTOR_BASE);

pub const ENXIO: i32 = 6;

/// `msi_alloc_descs(count)` — allocate `count` contiguous vectors.
pub fn msi_alloc_descs(count: u32) -> Result<u32, i32> {
    if count == 0 {
        return Err(ENXIO);
    }
    let start = NEXT_MSI.fetch_add(count, Ordering::AcqRel);
    if start.saturating_add(count) >= MSI_VECTOR_TOP {
        // Rollback.
        NEXT_MSI.fetch_sub(count, Ordering::AcqRel);
        return Err(ENXIO);
    }
    Ok(start)
}

/// `msi_free_descs(start, count)` — Lupos M37 doesn't recycle (linear pool).
/// PCI hot-unplug recycling lands in M55.
pub fn msi_free_descs(_start: u32, _count: u32) {}

/// Reset the pool — used by tests only.
#[doc(hidden)]
pub fn _reset_for_tests() {
    NEXT_MSI.store(MSI_VECTOR_BASE, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_returns_consecutive_vectors() {
        _reset_for_tests();
        let a = msi_alloc_descs(1).unwrap();
        let b = msi_alloc_descs(1).unwrap();
        assert_eq!(b, a + 1);
    }

    #[test]
    fn alloc_zero_returns_enxio() {
        assert_eq!(msi_alloc_descs(0), Err(ENXIO));
    }

    #[test]
    fn alloc_within_pool() {
        _reset_for_tests();
        let s = msi_alloc_descs(4).unwrap();
        assert!(s >= MSI_VECTOR_BASE);
    }
}
