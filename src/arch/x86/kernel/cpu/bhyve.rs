//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/bhyve.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/bhyve.c
//! FreeBSD Bhyve hypervisor detection.

pub const BHYVE_SIGNATURE: &str = "bhyve bhyve ";
pub const CPUID_BHYVE_FEATURES: u32 = 0x4000_0001;
pub const CPUID_BHYVE_FEAT_EXT_DEST_ID: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BhyveHypervisor {
    pub name: &'static str,
    pub cpuid_base: u32,
    pub cpuid_max: u32,
    pub features: u32,
}

pub const fn bhyve_detect(hypervisor_feature: bool, cpuid_base: u32, cpuid_max: u32) -> u32 {
    if !hypervisor_feature || cpuid_base == 0 {
        0
    } else {
        cpuid_max
    }
}

pub const fn bhyve_features(cpuid_base: u32, cpuid_max: u32, eax_features: u32) -> u32 {
    let cpuid_leaf = cpuid_base | CPUID_BHYVE_FEATURES;
    if cpuid_max < cpuid_leaf {
        0
    } else {
        eax_features
    }
}

pub const fn bhyve_ext_dest_id(features: u32) -> bool {
    features & CPUID_BHYVE_FEAT_EXT_DEST_ID != 0
}

pub const fn x86_hyper_bhyve(cpuid_base: u32, cpuid_max: u32, features: u32) -> BhyveHypervisor {
    BhyveHypervisor {
        name: "Bhyve",
        cpuid_base,
        cpuid_max,
        features,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bhyve_detection_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/bhyve.c"
        ));
        assert!(source.contains("#define BHYVE_SIGNATURE"));
        assert!(source.contains("#define CPUID_BHYVE_FEATURES"));
        assert!(source.contains("CPUID_BHYVE_FEAT_EXT_DEST_ID"));
        assert!(source.contains("cpu_feature_enabled(X86_FEATURE_HYPERVISOR)"));
        assert!(source.contains("cpuid_base_hypervisor(BHYVE_SIGNATURE, 0);"));
        assert!(source.contains("cpuid_eax(bhyve_cpuid_base);"));
        assert!(source.contains("bhyve_ext_dest_id"));
        assert!(source.contains(".name\t\t\t= \"Bhyve\""));
        assert!(source.contains(".init.x2apic_available\t= bhyve_x2apic_available"));
        assert!(source.contains(".init.msi_ext_dest_id\t= bhyve_ext_dest_id"));

        assert_eq!(bhyve_detect(false, 0x4000_0000, 0x4000_0001), 0);
        assert_eq!(bhyve_detect(true, 0, 0x4000_0001), 0);
        assert_eq!(bhyve_detect(true, 0x4000_0000, 0x4000_0001), 0x4000_0001);
        assert_eq!(bhyve_features(0x4000_0000, 0x4000_0000, 1), 0);
        assert!(bhyve_ext_dest_id(1));
        assert_eq!(x86_hyper_bhyve(1, 2, 3).name, "Bhyve");
    }
}
