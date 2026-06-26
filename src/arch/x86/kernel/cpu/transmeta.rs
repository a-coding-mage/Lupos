//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/transmeta.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/transmeta.c
//! Transmeta Crusoe / Efficeon CPU vendor init.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/transmeta.c

// Transmeta parts expose an additional CPUID 0x8086_0001 leaf carrying
// the code-morphing software (CMS) revision in EBX. `transmeta.c`
// surfaces this in /proc/cpuinfo. We model the leaf decoder.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransmetaCmsInfo {
    pub revision: u32,
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
}

pub const fn parse_8086_0001(eax: u32, ebx: u32) -> TransmetaCmsInfo {
    TransmetaCmsInfo {
        revision: ebx,
        family: (eax >> 8) & 0xf,
        model: (eax >> 4) & 0xf,
        stepping: eax & 0xf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cms_info_decodes_eax_and_ebx() {
        let info = parse_8086_0001(0x0000_0123, 0xdead_beef);
        assert_eq!(info.family, 1);
        assert_eq!(info.model, 2);
        assert_eq!(info.stepping, 3);
        assert_eq!(info.revision, 0xdead_beef);
    }
}
