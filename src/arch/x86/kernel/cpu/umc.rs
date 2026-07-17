//! test-origin: lupos-specific:legacy UMC CPU-vendor model removed from current Linux
//! UMC (United Microelectronics Corp.) CPU vendor.
//!
//! Retained for compatibility with old x86 CPUs; the current vendored Linux
//! tree no longer contains the former `arch/x86/kernel/cpu/umc.c` source.

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
