//! linux-parity: partial
//! linux-source: vendor/linux/drivers/iommu/iommu.c
//! test-origin: linux:vendor/linux/drivers/iommu/iommu.c
//! IOMMU framework — M55.
//!
//! Mirrors `include/linux/iommu.h` and `drivers/iommu/iommu.c`.
//!
//! For M55 we implement the **passthrough** domain only: `iommu_map`
//! records the mapping metadata but does not program real hardware.
//! A real IOMMU driver (Intel VT-d / AMD-Vi) is a deferred milestone.
//!
//! References:
//!   - `include/linux/iommu.h:223`      — `struct iommu_domain`
//!   - `drivers/iommu/iommu.c:2680`    — `iommu_map`
//!   - `drivers/iommu/iommu.c:2788`    — `iommu_unmap`
//!   - `drivers/iommu/iommu.c:2171`    — `iommu_attach_device`

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, EINVAL};

/// IOMMU domain type — mirrors `enum iommu_domain_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IommuDomainType {
    /// Device addresses = physical addresses (no translation).
    Passthrough,
    /// Unmanaged DMA — used for user-space DMA remapping.
    Dma,
}

/// `struct iommu_domain` — `include/linux/iommu.h:223`.
pub struct IommuDomain {
    pub domain_type: IommuDomainType,
    /// Mapping table: IOVA → (paddr, size).
    mappings: Mutex<BTreeMap<u64, (u64, usize)>>,
}

impl IommuDomain {
    /// `iommu_domain_alloc` — allocate a new domain of the given type.
    pub fn alloc(domain_type: IommuDomainType) -> Arc<Self> {
        Arc::new(Self {
            domain_type,
            mappings: Mutex::new(BTreeMap::new()),
        })
    }
}

/// `iommu_map` — `drivers/iommu/iommu.c:2680`.
///
/// Maps `paddr..paddr+size` at IOVA `iova` in `domain`.
/// On the passthrough domain this is a no-op (IOVA == PA), but we still
/// record the mapping so `iommu_unmap` can validate the argument.
pub fn iommu_map(domain: &Arc<IommuDomain>, iova: u64, paddr: u64, size: usize) -> Result<(), i32> {
    if size == 0 || (iova & 0xFFF != 0) || (paddr & 0xFFF != 0) {
        return Err(EINVAL);
    }
    let mut g = domain.mappings.lock();
    if g.contains_key(&iova) {
        return Err(EEXIST);
    }
    g.insert(iova, (paddr, size));
    Ok(())
}

/// `iommu_unmap` — `drivers/iommu/iommu.c:2788`.
///
/// Removes the mapping at `iova` in `domain`.
pub fn iommu_unmap(domain: &Arc<IommuDomain>, iova: u64, size: usize) -> usize {
    let mut g = domain.mappings.lock();
    if let Some((_, mapped_size)) = g.remove(&iova) {
        if mapped_size == size { mapped_size } else { 0 }
    } else {
        0
    }
}

/// `iommu_attach_device` — `drivers/iommu/iommu.c:2171`.
///
/// Associates a device with a domain.  On passthrough we accept any device.
pub fn iommu_attach_device(_domain: &Arc<IommuDomain>, _dev_name: &str) -> Result<(), i32> {
    Ok(())
}

/// Return the number of active mappings in `domain`.
pub fn iommu_mapping_count(domain: &Arc<IommuDomain>) -> usize {
    domain.mappings.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passthrough() -> Arc<IommuDomain> {
        IommuDomain::alloc(IommuDomainType::Passthrough)
    }

    #[test]
    fn map_and_unmap_roundtrip() {
        let dom = passthrough();
        iommu_map(&dom, 0x1000, 0x2000, 0x1000).unwrap();
        assert_eq!(iommu_mapping_count(&dom), 1);
        let unmapped = iommu_unmap(&dom, 0x1000, 0x1000);
        assert_eq!(unmapped, 0x1000);
        assert_eq!(iommu_mapping_count(&dom), 0);
    }

    #[test]
    fn duplicate_map_returns_eexist() {
        let dom = passthrough();
        iommu_map(&dom, 0x1000, 0x2000, 0x1000).unwrap();
        let r = iommu_map(&dom, 0x1000, 0x3000, 0x1000);
        assert_eq!(r, Err(EEXIST));
    }

    #[test]
    fn unaligned_iova_returns_einval() {
        let dom = passthrough();
        let r = iommu_map(&dom, 0x100, 0x2000, 0x1000); // not page-aligned
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn attach_device_ok() {
        let dom = passthrough();
        iommu_attach_device(&dom, "pci:0000:00:01.0").unwrap();
    }
}
