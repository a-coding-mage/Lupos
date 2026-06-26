//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/aperture_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/aperture_64.c
//! AMD64 GART aperture discovery policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/aperture_64.c

pub const GART_MIN_ADDR: u64 = 512 * 1024 * 1024;
pub const GART_MAX_ADDR: u64 = 1u64 << 32;
pub const GART_MIN_SIZE: u64 = 32 * 1024 * 1024;
pub const GART_MAX_SIZE: u64 = 2 * 1024 * 1024 * 1024;

pub const GARTEN: u32 = 1 << 0;
pub const AMD64_GARTAPERTURECTL: u16 = 0x90;
pub const AMD64_GARTAPERTUREBASE: u16 = 0x94;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Aperture {
    pub base: u64,
    pub size: u64,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GartE820Action {
    None,
    Reserve { base: u64, size: u64 },
    DisableAll,
    FixNeeded,
}

pub const fn aperture_size(order: u8) -> u64 {
    GART_MIN_SIZE << order
}

pub const fn allocate_aperture_size(order: u8) -> u64 {
    let clamped = if order > 5 { 5 } else { order };
    aperture_size(clamped)
}

pub const fn aperture_valid(base: u64, size: u64, min_size: u64) -> bool {
    base != 0
        && size >= min_size
        && size <= GART_MAX_SIZE
        && base >= GART_MIN_ADDR
        && base + size <= GART_MAX_ADDR
        && (base & (size - 1)) == 0
}

pub const fn decode_aperture(control: u32, base_reg: u32) -> Aperture {
    let order = ((control >> 1) & 0x1f) as u8;
    Aperture {
        base: (base_reg as u64) << 25,
        size: aperture_size(order),
        enabled: (control & GARTEN) != 0,
    }
}

pub fn choose_free_aperture(ranges: &[MemoryRange], size: u64) -> Option<u64> {
    for range in ranges {
        let start = align_up(range.start.max(GART_MIN_ADDR), size);
        if start
            .checked_add(size)
            .map_or(false, |end| end - 1 <= range.end)
            && start + size <= GART_MAX_ADDR
        {
            return Some(start);
        }
    }
    None
}

pub fn gart_e820_action(
    amd_gart_present: bool,
    early_pci_allowed: bool,
    current: Option<Aperture>,
    free_ranges: &[MemoryRange],
    min_size: u64,
) -> GartE820Action {
    if !amd_gart_present {
        return GartE820Action::None;
    }
    if !early_pci_allowed {
        return GartE820Action::FixNeeded;
    }
    if let Some(aperture) = current {
        if aperture.enabled && aperture_valid(aperture.base, aperture.size, min_size) {
            return GartE820Action::Reserve {
                base: aperture.base,
                size: aperture.size,
            };
        }
    }
    if let Some(base) = choose_free_aperture(free_ranges, min_size) {
        GartE820Action::Reserve {
            base,
            size: min_size,
        }
    } else {
        GartE820Action::DisableAll
    }
}

const fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        value
    } else {
        ((value + align - 1) / align) * align
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aperture_size_starts_at_linux_minimum() {
        assert_eq!(aperture_size(0), 32 * 1024 * 1024);
        assert_eq!(allocate_aperture_size(9), 1024 * 1024 * 1024);
    }

    #[test]
    fn valid_aperture_requires_low_4g_aligned_window() {
        assert!(aperture_valid(0x8000_0000, GART_MIN_SIZE, GART_MIN_SIZE));
        assert!(!aperture_valid(0x1000_0000, GART_MIN_SIZE, GART_MIN_SIZE));
        assert!(!aperture_valid(0x8100_0000, GART_MIN_SIZE, GART_MIN_SIZE));
    }

    #[test]
    fn decode_aperture_uses_control_order_and_base_register() {
        let aperture = decode_aperture(GARTEN | (2 << 1), 0x40);
        assert_eq!(
            aperture,
            Aperture {
                base: 0x8000_0000,
                size: 128 * 1024 * 1024,
                enabled: true
            }
        );
    }

    #[test]
    fn gart_e820_reserves_existing_or_free_window() {
        let current = Aperture {
            base: 0x8000_0000,
            size: GART_MIN_SIZE,
            enabled: true,
        };
        assert_eq!(
            gart_e820_action(true, true, Some(current), &[], GART_MIN_SIZE),
            GartE820Action::Reserve {
                base: 0x8000_0000,
                size: GART_MIN_SIZE
            }
        );
        assert_eq!(
            gart_e820_action(
                true,
                true,
                None,
                &[MemoryRange {
                    start: GART_MIN_ADDR,
                    end: GART_MIN_ADDR + GART_MIN_SIZE - 1
                }],
                GART_MIN_SIZE,
            ),
            GartE820Action::Reserve {
                base: GART_MIN_ADDR,
                size: GART_MIN_SIZE
            }
        );
    }
}
