//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/topology_ext.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/topology_ext.c
//! Extended-topology CPUID (0x1F) decoder for Intel.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/topology_ext.c

// CPUID(0x1F) is the v2 successor to CPUID(0xB). It adds module, tile,
// and die level types and a unique 32-bit x2APIC ID in EDX. We model the
// extraction of the x2APIC ID and the shift-table assembly.

use super::topology_common::{TopologyLevel, classify_level};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct ExtTopologyLevel {
    pub level_type: u8,
    pub shift: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct ExtTopology {
    pub x2apic_id: u32,
    pub shifts: [ExtTopologyLevel; 6],
}

pub fn record_level(ext: &mut ExtTopology, index: usize, eax: u32, ecx: u32) {
    if index >= ext.shifts.len() {
        return;
    }
    ext.shifts[index] = ExtTopologyLevel {
        level_type: classify_level_byte(ecx),
        shift: (eax & 0x1f) as u8,
    };
}

const fn classify_level_byte(ecx: u32) -> u8 {
    match classify_level(ecx) {
        TopologyLevel::Smt => 1,
        TopologyLevel::Core => 2,
        TopologyLevel::Module => 3,
        TopologyLevel::Tile => 4,
        TopologyLevel::Die => 5,
        TopologyLevel::Package => 6,
        TopologyLevel::Invalid => 0,
        TopologyLevel::Other(b) => b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_level_stores_level_type_and_shift() {
        let mut ext = ExtTopology::default();
        record_level(&mut ext, 0, 1, (1 << 8) | 0);
        assert_eq!(ext.shifts[0].level_type, 1);
        assert_eq!(ext.shifts[0].shift, 1);
    }
}
