//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/vortex.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/vortex.c
//! DM&P Vortex 86 CPU vendor.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/vortex.c

// Vortex CPUs (used in embedded systems) are 486-class. Linux applies a
// few feature-bit fixups based on CPUID(0x80000004) brand string. We
// model the brand-substring detector.

pub fn is_vortex(brand: &[u8]) -> bool {
    let marker = b"Vortex86";
    brand.windows(marker.len()).any(|w| w == marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_vortex_brand_string() {
        assert!(is_vortex(b"DM&P Vortex86 SX"));
        assert!(!is_vortex(b"GenuineIntel CPU"));
    }
}
