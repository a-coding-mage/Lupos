//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/kmsan_shadow.c
//! test-origin: linux:vendor/linux/arch/x86/mm/kmsan_shadow.c
//! KMSAN shadow address helpers.
//!
//! Mirrors the disabled x86 KMSAN shadow policy from
//! `vendor/linux/arch/x86/mm/kmsan_shadow.c`. Lupos does not enable KMSAN, but
//! the address transform remains useful for compile-time parity checks.

use crate::include::uapi::errno::ENODEV;

pub const KMSAN_SHADOW_SCALE_SHIFT: u64 = 3;
pub const KMSAN_SHADOW_OFFSET: u64 = 0xdfff_9000_0000_0000;

pub const fn kmsan_enabled() -> bool {
    false
}

pub const fn kmsan_mem_to_shadow(addr: u64) -> u64 {
    (addr >> KMSAN_SHADOW_SCALE_SHIFT).wrapping_add(KMSAN_SHADOW_OFFSET)
}

pub const fn kmsan_init_shadow() -> Result<(), i32> {
    Err(ENODEV)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_transform_scales_by_eight() {
        assert_eq!(
            kmsan_mem_to_shadow(0x80),
            KMSAN_SHADOW_OFFSET + (0x80 >> KMSAN_SHADOW_SCALE_SHIFT)
        );
    }

    #[test]
    fn kmsan_init_fails_closed() {
        assert_eq!(kmsan_init_shadow(), Err(ENODEV));
    }
}
