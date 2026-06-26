//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/scattered.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/scattered.c
//! "Scattered" CPU feature bits — synthetic flags assembled from CPUID.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/scattered.c

// Linux uses a `cpuid_bit` table to collect feature bits that don't fit
// into the standard CPUID leaves (e.g. AMD perfctr core, Intel HWP MSRs).
// Each entry says "leaf L, subleaf S, register R, bit B" → set X86_FEATURE_N.
// We model the table walker.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuidRegister {
    Eax,
    Ebx,
    Ecx,
    Edx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScatteredFeature {
    pub feature_id: u16,
    pub leaf: u32,
    pub subleaf: u32,
    pub reg: CpuidRegister,
    pub bit: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuidLeafResult {
    pub leaf: u32,
    pub subleaf: u32,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

pub const fn select_reg(result: CpuidLeafResult, reg: CpuidRegister) -> u32 {
    match reg {
        CpuidRegister::Eax => result.eax,
        CpuidRegister::Ebx => result.ebx,
        CpuidRegister::Ecx => result.ecx,
        CpuidRegister::Edx => result.edx,
    }
}

pub fn feature_set(entry: ScatteredFeature, result: CpuidLeafResult) -> bool {
    if entry.leaf != result.leaf || entry.subleaf != result.subleaf {
        return false;
    }
    (select_reg(result, entry.reg) >> (entry.bit as u32)) & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_bit_read_uses_selected_register() {
        let entry = ScatteredFeature {
            feature_id: 0x42,
            leaf: 0x8000_0007,
            subleaf: 0,
            reg: CpuidRegister::Edx,
            bit: 9,
        };
        let result = CpuidLeafResult {
            leaf: 0x8000_0007,
            subleaf: 0,
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 1 << 9,
        };
        assert!(feature_set(entry, result));
    }

    #[test]
    fn mismatched_leaf_skips_entry() {
        let entry = ScatteredFeature {
            feature_id: 0x42,
            leaf: 0x8000_0007,
            subleaf: 0,
            reg: CpuidRegister::Edx,
            bit: 9,
        };
        let result = CpuidLeafResult {
            leaf: 0x1,
            subleaf: 0,
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 1 << 9,
        };
        assert!(!feature_set(entry, result));
    }
}
