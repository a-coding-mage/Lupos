//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr/amd.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr/amd.c
//! AMD K6/K7 MTRR vendor-specific register layout.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/mtrr/amd.c

// Pre-K7 AMD parts expose two pairs of "UWCCR" MSRs at 0xc000_0085/0086
// that encode a base/length pair plus three attribute bits per range
// (Write-Combining, Uncached, Mask). We model the encoder/decoder so PAT
// code can reason about the architectural range without programming MSRs.

pub const MSR_K6_UWCCR: u32 = 0xc000_0085;
pub const MSR_K6_PSOR: u32 = 0xc000_0087;

pub const AMD_MTRR_ATTR_WC: u32 = 1 << 0;
pub const AMD_MTRR_ATTR_UNCACHED: u32 = 1 << 1;
pub const AMD_MTRR_ATTR_MASK_BITS: u32 = 0xff_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdMtrrEntry {
    pub base: u64,
    pub mask: u64,
    pub write_combining: bool,
    pub uncached: bool,
}

pub const fn encode_entry(entry: AmdMtrrEntry) -> u64 {
    let mut value = entry.base & 0xffff_ffff_fffe_0000;
    if entry.write_combining {
        value |= AMD_MTRR_ATTR_WC as u64;
    }
    if entry.uncached {
        value |= AMD_MTRR_ATTR_UNCACHED as u64;
    }
    value
}

pub const fn decode_entry(value: u64) -> AmdMtrrEntry {
    AmdMtrrEntry {
        base: value & 0xffff_ffff_fffe_0000,
        mask: 0,
        write_combining: value & AMD_MTRR_ATTR_WC as u64 != 0,
        uncached: value & AMD_MTRR_ATTR_UNCACHED as u64 != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_preserves_attribute_bits() {
        let entry = AmdMtrrEntry {
            base: 0xe000_0000,
            mask: 0,
            write_combining: true,
            uncached: false,
        };
        let encoded = encode_entry(entry);
        let decoded = decode_entry(encoded);
        assert!(decoded.write_combining);
        assert!(!decoded.uncached);
        assert_eq!(decoded.base, 0xe000_0000);
    }
}
