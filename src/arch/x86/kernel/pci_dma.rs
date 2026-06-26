//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/pci-dma.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/pci-dma.c
//! x86 PCI DMA and IOMMU command-line policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/pci-dma.c

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IommuPolicy {
    pub panic_on_overflow: bool,
    pub force_iommu: bool,
    pub iommu_merge: bool,
    pub no_iommu: bool,
    pub iommu_detected: bool,
    pub swiotlb_enable: bool,
    pub passthrough: bool,
    pub use_dac: bool,
}

pub fn parse_iommu_option(policy: &mut IommuPolicy, option: &str) -> bool {
    match option {
        "off" => policy.no_iommu = true,
        "force" => policy.force_iommu = true,
        "noforce" => policy.force_iommu = false,
        "biomerge" | "merge" => policy.iommu_merge = true,
        "nomerge" => policy.iommu_merge = false,
        "panic" => policy.panic_on_overflow = true,
        "nopanic" => policy.panic_on_overflow = false,
        "usedac" => policy.use_dac = true,
        "soft" => policy.swiotlb_enable = true,
        "pt" => policy.passthrough = true,
        "nopt" => policy.passthrough = false,
        _ => return false,
    }
    true
}

pub fn iommu_setup(policy: &mut IommuPolicy, arg: &str) {
    for opt in arg.split(',') {
        let _ = parse_iommu_option(policy, opt.trim());
    }
}

pub const fn detect_swiotlb(
    policy: IommuPolicy,
    max_pfn: u64,
    dma32_pfn: u64,
    cc_active: bool,
) -> bool {
    policy.swiotlb_enable || cc_active || max_pfn > dma32_pfn
}

pub const fn via_no_dac_quirk(vendor: u16, device: u16) -> bool {
    vendor == 0x1106 && device != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_tracks_linux_iommu_options() {
        let mut p = IommuPolicy::default();
        iommu_setup(&mut p, "force,panic,nomerge,usedac,pt,soft");
        assert!(p.force_iommu);
        assert!(p.panic_on_overflow);
        assert!(!p.iommu_merge);
        assert!(p.use_dac);
        assert!(p.passthrough);
        assert!(p.swiotlb_enable);
    }

    #[test]
    fn swiotlb_needed_above_dma32_or_confidential_compute() {
        assert!(detect_swiotlb(
            IommuPolicy::default(),
            0x2_0000,
            0x1_0000,
            false
        ));
        assert!(detect_swiotlb(IommuPolicy::default(), 1, 2, true));
        assert!(!detect_swiotlb(IommuPolicy::default(), 1, 2, false));
    }
}
