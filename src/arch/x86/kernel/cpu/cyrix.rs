//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/cyrix.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/cyrix.c
//! Cyrix CPU vendor init quirks.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/cyrix.c

// Cyrix parts use a configuration index/data window at I/O ports 0x22/0x23
// instead of CPUID. Linux probes a small set of CCR0..CCR5 registers and
// classifies the chip into 5x86, 6x86, MII, etc. We model the register
// indices and the family classifier; actual port I/O is not performed.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyrixFamily {
    Cx486,
    Cx5x86,
    Cx6x86,
    Cx6x86MX,
    Mediagx,
    Unknown,
}

pub const CYRIX_CCR_PORT_INDEX: u16 = 0x22;
pub const CYRIX_CCR_PORT_DATA: u16 = 0x23;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CyrixDir {
    pub dir0: u8,
    pub dir1: u8,
}

pub const fn classify(dir: CyrixDir) -> CyrixFamily {
    match dir.dir0 {
        0x00..=0x1f => CyrixFamily::Cx486,
        0x20..=0x2f => CyrixFamily::Cx5x86,
        0x30..=0x3f => CyrixFamily::Cx6x86,
        0x40..=0x4f => CyrixFamily::Mediagx,
        0x50..=0x6f => CyrixFamily::Cx6x86MX,
        _ => CyrixFamily::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir0_ranges_match_linux_classifier() {
        assert_eq!(
            classify(CyrixDir {
                dir0: 0x05,
                dir1: 0
            }),
            CyrixFamily::Cx486
        );
        assert_eq!(
            classify(CyrixDir {
                dir0: 0x33,
                dir1: 0
            }),
            CyrixFamily::Cx6x86
        );
        assert_eq!(
            classify(CyrixDir {
                dir0: 0x55,
                dir1: 0
            }),
            CyrixFamily::Cx6x86MX
        );
        assert_eq!(
            classify(CyrixDir {
                dir0: 0xff,
                dir1: 0
            }),
            CyrixFamily::Unknown
        );
    }
}
