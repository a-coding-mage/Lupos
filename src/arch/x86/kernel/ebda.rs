//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ebda.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ebda.c
//! BIOS/EBDA reservation helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/ebda.c

pub const BIOS_RAM_SIZE_KB_PTR: u32 = 0x413;
pub const BIOS_START_MIN: u32 = 0x20000;
pub const BIOS_START_MAX: u32 = 0x9f000;
pub const ONE_MB: u32 = 0x100000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BiosRegion {
    pub start: u32,
    pub size: u32,
}

pub const fn reserve_bios_region_start(
    bios_kb: u16,
    ebda_start: u32,
    legacy_reserve: bool,
) -> Option<u32> {
    if !legacy_reserve {
        return None;
    }
    let mut bios_start = (bios_kb as u32) << 10;
    if bios_start < BIOS_START_MIN || bios_start > BIOS_START_MAX {
        bios_start = BIOS_START_MAX;
    }
    if ebda_start >= BIOS_START_MIN && ebda_start < bios_start {
        bios_start = ebda_start;
    }
    Some(bios_start)
}

pub const fn reserved_bios_region(
    bios_kb: u16,
    ebda_start: u32,
    legacy_reserve: bool,
) -> Option<BiosRegion> {
    match reserve_bios_region_start(bios_kb, ebda_start, legacy_reserve) {
        Some(start) => Some(BiosRegion {
            start,
            size: ONE_MB - start,
        }),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conventional_memory_size_selects_bios_region_start() {
        assert_eq!(
            reserve_bios_region_start(640, 0, true),
            Some(BIOS_START_MAX)
        );
        assert_eq!(reserve_bios_region_start(512, 0, true), Some(512 * 1024));
    }

    #[test]
    fn bogus_bios_size_is_clamped_to_640k() {
        assert_eq!(reserve_bios_region_start(32, 0, true), Some(BIOS_START_MAX));
        assert_eq!(
            reserve_bios_region_start(1024, 0, true),
            Some(BIOS_START_MAX)
        );
    }

    #[test]
    fn sane_ebda_below_bios_start_extends_reserved_region() {
        assert_eq!(reserve_bios_region_start(640, 0x80000, true), Some(0x80000));
        assert_eq!(
            reserved_bios_region(640, 0x80000, true),
            Some(BiosRegion {
                start: 0x80000,
                size: 0x80000
            })
        );
    }

    #[test]
    fn legacy_platform_hook_can_disable_reservation() {
        assert_eq!(reserved_bios_region(640, 0x80000, false), None);
    }
}
