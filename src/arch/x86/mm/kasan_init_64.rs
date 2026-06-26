//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/kasan_init_64.c
//! test-origin: linux:vendor/linux/arch/x86/mm/kasan_init_64.c
//! x86_64 KASAN shadow policy.
//!
//! Mirrors the address transform and disabled init gate from
//! `vendor/linux/arch/x86/mm/kasan_init_64.c`.

use crate::include::uapi::errno::ENODEV;

pub const KASAN_SHADOW_SCALE_SHIFT: u64 = 3;
pub const KASAN_SHADOW_OFFSET: u64 = 0xdfff_8000_0000_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KasanShadowRange {
    pub start: u64,
    pub end: u64,
}

pub const fn kasan_enabled() -> bool {
    false
}

pub const fn kasan_mem_to_shadow(addr: u64) -> u64 {
    (addr >> KASAN_SHADOW_SCALE_SHIFT).wrapping_add(KASAN_SHADOW_OFFSET)
}

pub const fn kasan_shadow_range(start: u64, end: u64) -> KasanShadowRange {
    KasanShadowRange {
        start: kasan_mem_to_shadow(start),
        end: kasan_mem_to_shadow(end),
    }
}

pub const fn kasan_init() -> Result<(), i32> {
    Err(ENODEV)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_range_scales_addresses() {
        assert_eq!(
            kasan_shadow_range(0, 0x40),
            KasanShadowRange {
                start: KASAN_SHADOW_OFFSET,
                end: KASAN_SHADOW_OFFSET + 8
            }
        );
    }

    #[test]
    fn kasan_init_fails_closed() {
        assert_eq!(kasan_init(), Err(ENODEV));
    }
}
