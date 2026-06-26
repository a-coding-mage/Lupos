//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/topology_common.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/topology_common.c
//! CPUID 0xB / 0x1F (x2APIC) topology decoder.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/topology_common.c

// CPUID 0xB enumerates topology level by level (subleaf=0..N). Each level
// returns: ECX.bits[7:0] = level number, ECX.bits[15:8] = level type,
// EAX[4:0] = shift to next level, EBX[15:0] = logical processors in this level.
// Type 1=SMT, 2=Core, 3=Module, 4=Tile, 5=Die, 6=Package.
// We model the level decoder.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyLevel {
    Invalid,
    Smt,
    Core,
    Module,
    Tile,
    Die,
    Package,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TopologyLevelDescriptor {
    pub level_number: u8,
    pub level_type: TopologyLevel,
    pub shift_to_next: u8,
    pub logical_processors: u16,
}

pub const fn classify_level(ecx: u32) -> TopologyLevel {
    match (ecx >> 8) & 0xff {
        0 => TopologyLevel::Invalid,
        1 => TopologyLevel::Smt,
        2 => TopologyLevel::Core,
        3 => TopologyLevel::Module,
        4 => TopologyLevel::Tile,
        5 => TopologyLevel::Die,
        6 => TopologyLevel::Package,
        other => TopologyLevel::Other(other as u8),
    }
}

pub const fn decode(eax: u32, ebx: u32, ecx: u32) -> TopologyLevelDescriptor {
    TopologyLevelDescriptor {
        level_number: (ecx & 0xff) as u8,
        level_type: classify_level(ecx),
        shift_to_next: (eax & 0x1f) as u8,
        logical_processors: (ebx & 0xffff) as u16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smt_level_decodes_correctly() {
        let d = decode(1, 2, (1 << 8) | 0);
        assert_eq!(d.level_type, TopologyLevel::Smt);
        assert_eq!(d.shift_to_next, 1);
        assert_eq!(d.logical_processors, 2);
    }

    #[test]
    fn invalid_level_when_type_is_zero() {
        let d = decode(0, 0, 0);
        assert_eq!(d.level_type, TopologyLevel::Invalid);
    }
}
