//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/pseudo_lock.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/pseudo_lock.c
//! L2/L3 pseudo-locking via resctrl.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/resctrl/pseudo_lock.c

// Pseudo-locking pins a region of memory into a chosen cache way bitmap
// so other workloads cannot evict it. The driver validates the requested
// bitmap against the contiguous-bit constraint. We model the predicate.

pub const fn contiguous_bits(bitmap: u64) -> bool {
    if bitmap == 0 {
        return false;
    }
    let bits = bitmap;
    let trailing_zeros = bits.trailing_zeros();
    let shifted = bits >> trailing_zeros;
    let ones = (!shifted).trailing_zeros();
    shifted >> ones == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contiguous_bitmap_passes() {
        assert!(contiguous_bits(0b0011_1100));
        assert!(contiguous_bits(0b0000_0001));
    }

    #[test]
    fn non_contiguous_bitmap_fails() {
        assert!(!contiguous_bits(0b0011_0011));
        assert!(!contiguous_bits(0));
    }
}
