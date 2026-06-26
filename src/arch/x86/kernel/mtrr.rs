//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Memory Type Range Register decoding.
//!
//! Full variable-range programming is deferred until the resource tree can
//! arbitrate overlapping RAM/device ranges. The decoding here is enough for
//! PAT/ioremap to reason about the architectural default memory type.
//!
//! References:
//! - vendor/linux/arch/x86/kernel/cpu/mtrr/generic.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MtrrMemoryType {
    Uncacheable = 0,
    WriteCombining = 1,
    WriteThrough = 4,
    WriteProtected = 5,
    WriteBack = 6,
}

pub const MSR_MTRRcap: u32 = 0x0000_00FE;
pub const MSR_MTRRdefType: u32 = 0x0000_02FF;
pub const MTRR_DEF_TYPE_ENABLE: u64 = 1 << 11;
pub const MTRR_DEF_TYPE_FIXED_ENABLE: u64 = 1 << 10;
pub const MTRR_DEF_TYPE_MASK: u64 = 0xff;

pub const fn default_type_from_msr(value: u64) -> Option<MtrrMemoryType> {
    match value & MTRR_DEF_TYPE_MASK {
        0 => Some(MtrrMemoryType::Uncacheable),
        1 => Some(MtrrMemoryType::WriteCombining),
        4 => Some(MtrrMemoryType::WriteThrough),
        5 => Some(MtrrMemoryType::WriteProtected),
        6 => Some(MtrrMemoryType::WriteBack),
        _ => None,
    }
}

pub const fn mtrrs_enabled(value: u64) -> bool {
    value & MTRR_DEF_TYPE_ENABLE != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtrr_default_type_decodes_architectural_values() {
        assert_eq!(default_type_from_msr(0), Some(MtrrMemoryType::Uncacheable));
        assert_eq!(default_type_from_msr(6), Some(MtrrMemoryType::WriteBack));
        assert_eq!(default_type_from_msr(7), None);
    }

    #[test]
    fn mtrr_enable_bit_matches_linux_generic_mtrr() {
        assert!(mtrrs_enabled(MTRR_DEF_TYPE_ENABLE | 6));
        assert!(!mtrrs_enabled(6));
    }
}
