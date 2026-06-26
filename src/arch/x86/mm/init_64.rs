//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/init_64.c
//! test-origin: linux:vendor/linux/arch/x86/mm/init_64.c
//! x86_64 memory initialization policy.
//!
//! Mirrors mask construction and NX command-line policy from
//! `vendor/linux/arch/x86/mm/init_64.c`.

use crate::arch::x86::mm::paging::{
    _PAGE_NX, _PAGE_PKEY_BIT0, _PAGE_PKEY_BIT1, _PAGE_PKEY_BIT2, _PAGE_PKEY_BIT3,
};
use crate::include::uapi::errno::EINVAL;

pub const DEFAULT_PHYSICAL_MASK_SHIFT: u8 = 52;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nonx32Mode {
    Default,
    Enabled,
    Disabled,
}

pub const fn physical_mask(physical_bits: u8) -> Result<u64, i32> {
    if physical_bits == 0 || physical_bits > 52 {
        return Err(EINVAL);
    }
    Ok((1u64 << physical_bits) - 1)
}

pub const fn supported_pte_mask(nx: bool, pkeys: bool) -> u64 {
    let mut mask = u64::MAX;
    if !nx {
        mask &= !_PAGE_NX;
    }
    if !pkeys {
        mask &= !(_PAGE_PKEY_BIT0 | _PAGE_PKEY_BIT1 | _PAGE_PKEY_BIT2 | _PAGE_PKEY_BIT3);
    }
    mask
}

pub fn nonx32_setup(arg: &str) -> Result<Nonx32Mode, i32> {
    match arg.as_bytes() {
        b"on" | b"1" => Ok(Nonx32Mode::Enabled),
        b"off" | b"0" => Ok(Nonx32Mode::Disabled),
        b"" => Ok(Nonx32Mode::Default),
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_mask_caps_at_x86_64_limit() {
        assert_eq!(physical_mask(52), Ok(0x000f_ffff_ffff_ffff));
        assert_eq!(physical_mask(53), Err(EINVAL));
    }

    #[test]
    fn unsupported_bits_are_cleared_from_pte_mask() {
        let mask = supported_pte_mask(false, false);
        assert_eq!(mask & _PAGE_NX, 0);
        assert_eq!(mask & _PAGE_PKEY_BIT0, 0);
    }
}
