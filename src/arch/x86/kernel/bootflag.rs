//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/bootflag.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/bootflag.c
//! Simple Boot Flag CMOS value helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/bootflag.c

pub const SBF_RESERVED: u8 = 0x78;
pub const SBF_PNPOS: u8 = 1 << 0;
pub const SBF_BOOTING: u8 = 1 << 1;
pub const SBF_DIAG: u8 = 1 << 2;
pub const SBF_PARITY: u8 = 1 << 7;

pub const fn parity8(mut value: u8) -> bool {
    value ^= value >> 4;
    ((0x6996u16 >> (value & 0x0f)) & 1) != 0
}

pub const fn sbf_value_valid(value: u8) -> bool {
    (value & SBF_RESERVED) == 0 && parity8(value)
}

pub const fn sbf_next_value(current: u8, isapnp: bool) -> u8 {
    let mut value = current & !SBF_RESERVED;
    value &= !SBF_BOOTING;
    value &= !SBF_DIAG;
    if isapnp {
        value |= SBF_PNPOS;
    }
    if !parity8(value) {
        value ^= SBF_PARITY;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity8_reports_odd_parity_like_linux_bitops() {
        assert!(!parity8(0));
        assert!(parity8(1));
        assert!(!parity8(3));
    }

    #[test]
    fn sbf_valid_rejects_reserved_bits_and_even_parity() {
        assert!(sbf_value_valid(SBF_PNPOS));
        assert!(!sbf_value_valid(SBF_RESERVED));
        assert!(!sbf_value_valid(0));
    }

    #[test]
    fn next_value_clears_boot_state_and_fixes_parity() {
        let value = sbf_next_value(SBF_BOOTING | SBF_DIAG | SBF_RESERVED, true);
        assert_eq!(value & (SBF_BOOTING | SBF_DIAG | SBF_RESERVED), 0);
        assert!(value & SBF_PNPOS != 0);
        assert!(parity8(value));
    }
}
