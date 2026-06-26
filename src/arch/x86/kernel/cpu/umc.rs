//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/umc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/umc.c
//! UMC (United Microelectronics Corp.) CPU vendor.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/umc.c

// UMC parts cloned the 486/U5S design. Linux includes a minimal vendor
// hook that just registers the vendor name. We model the vendor
// classifier predicate.

pub const UMC_VENDOR_STRING: [u8; 12] = *b"UMC UMC UMC ";

pub const fn is_umc(vendor_bytes: [u8; 12]) -> bool {
    let mut i = 0;
    while i < 12 {
        if vendor_bytes[i] != UMC_VENDOR_STRING[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umc_vendor_string_recognized() {
        assert!(is_umc(*b"UMC UMC UMC "));
        assert!(!is_umc(*b"GenuineIntel"));
    }
}
