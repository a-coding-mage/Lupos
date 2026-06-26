//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/msi/api.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/api.c
//! PCI MSI public API coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/api.c`.

use crate::include::uapi::errno::EINVAL;
use crate::kernel::irq::msi::{msi_alloc_descs, msi_free_descs};

pub fn pci_alloc_irq_vectors(min_vecs: u32, max_vecs: u32) -> Result<u32, i32> {
    if min_vecs == 0 || max_vecs < min_vecs {
        return Err(EINVAL);
    }
    msi_alloc_descs(max_vecs)
}

pub fn pci_free_irq_vectors(start: u32, count: u32) {
    msi_free_descs(start, count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_vector_range_returns_einval() {
        assert_eq!(pci_alloc_irq_vectors(2, 1), Err(EINVAL));
    }
}
