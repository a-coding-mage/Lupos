//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pkeys.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pkeys.c
//! x86 protection-key policy.
//!
//! Mirrors the user-visible gates from `vendor/linux/arch/x86/mm/pkeys.c`.
//! Lupos keeps pkeys disabled until CR4.PKE/PKRU context switching is present,
//! but exposes Linux-compatible PKRU bit construction and fail-closed hooks.

use crate::arch::x86::kernel::cpu::CpuFeatures;
use crate::include::uapi::errno::EOPNOTSUPP;

pub const ARCH_DEFAULT_PKEY: i32 = 0;
pub const PKRU_AD_BIT: u32 = 0x1;
pub const PKRU_WD_BIT: u32 = 0x2;

pub const fn pkru_ad_mask(pkey: u8) -> u32 {
    PKRU_AD_BIT << (pkey as u32 * 2)
}

pub const fn pkru_wd_mask(pkey: u8) -> u32 {
    PKRU_WD_BIT << (pkey as u32 * 2)
}

pub const fn cpu_has_pku(features: CpuFeatures) -> bool {
    features.leaf7_ecx & (1 << 3) != 0
}

pub const fn protection_keys_enabled(_features: CpuFeatures) -> bool {
    false
}

pub const fn execute_only_pkey() -> Result<i32, i32> {
    Err(EOPNOTSUPP)
}

pub const fn arch_override_mprotect_pkey(requested_pkey: i32) -> Result<i32, i32> {
    if requested_pkey == -1 {
        Ok(ARCH_DEFAULT_PKEY)
    } else {
        Err(EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::kernel::cpuid::CpuidResult;

    const ZERO_CPUID: CpuidResult = CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    #[test]
    fn pkru_masks_use_two_bits_per_key() {
        assert_eq!(pkru_ad_mask(0), 0x1);
        assert_eq!(pkru_wd_mask(1), 0x8);
    }

    #[test]
    fn pkeys_fail_closed_even_if_cpu_advertises_pku() {
        let features = CpuFeatures::from_cpuid(
            ZERO_CPUID,
            CpuidResult {
                ecx: 1 << 3,
                ..ZERO_CPUID
            },
            ZERO_CPUID,
        );
        assert!(cpu_has_pku(features));
        assert!(!protection_keys_enabled(features));
        assert_eq!(execute_only_pkey(), Err(EOPNOTSUPP));
    }
}
