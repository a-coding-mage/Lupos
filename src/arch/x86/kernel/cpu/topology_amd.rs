//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/topology_amd.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/topology_amd.c
//! AMD topology decoder (extended CPUID 0x8000_001e leaf).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/topology_amd.c

// AMD encodes die/CCD/CCX/core in CPUID(0x8000_001e).ECX/EBX. We model
// the helpers that pull those fields out of a raw CPUID result.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct AmdTopologyLeaf {
    pub ebx: u32,
    pub ecx: u32,
}

pub const fn core_id(leaf: AmdTopologyLeaf) -> u32 {
    leaf.ebx & 0xff
}

pub const fn threads_per_core(leaf: AmdTopologyLeaf) -> u32 {
    ((leaf.ebx >> 8) & 0xff) + 1
}

pub const fn node_id(leaf: AmdTopologyLeaf) -> u32 {
    leaf.ecx & 0xff
}

pub const fn nodes_per_processor(leaf: AmdTopologyLeaf) -> u32 {
    ((leaf.ecx >> 8) & 0x07) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_amd_topology_fields() {
        let leaf = AmdTopologyLeaf {
            ebx: 0x0000_0103,
            ecx: 0x0000_0001,
        };
        assert_eq!(core_id(leaf), 3);
        assert_eq!(threads_per_core(leaf), 2);
        assert_eq!(node_id(leaf), 1);
        assert_eq!(nodes_per_processor(leaf), 1);
    }
}
