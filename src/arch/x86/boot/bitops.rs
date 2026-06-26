//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/bitops.h
//! test-origin: linux:vendor/linux/arch/x86/boot/bitops.h
//! Very simple bitops for the real-mode boot code.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/bitops.h
//!
//! The setup stub defines `_LINUX_BITOPS_H` to inhibit the full kernel
//! `<linux/bitops.h>` and supplies these three tiny helpers. Linux treats
//! the address as a `const u32 *` and operates with the x86 bit-string
//! instructions:
//!   * `constant_test_bit` — pure C: `(1UL << (nr & 31)) & p[nr >> 5]`.
//!   * `variable_test_bit` — `btl %nr, *p` reading the carry flag.
//!   * `set_bit`           — `btsl %nr, *addr`.
//!
//! `btl`/`btsl` index a bit string laid out as little-endian 32-bit words:
//! word = `nr >> 5`, bit-in-word = `nr & 31`. That is exactly the C
//! `constant_test_bit` formula, so the asm and the C paths are
//! behaviourally identical. We therefore translate everything to safe Rust
//! over `&[u32]` / `&mut [u32]` with the same word/mask arithmetic.

/// Word index for bit `nr` — Linux `nr >> 5`.
#[inline]
const fn word_index(nr: i32) -> usize {
    (nr >> 5) as usize
}

/// Bit mask within a word for bit `nr` — Linux `1UL << (nr & 31)`.
#[inline]
const fn word_mask(nr: i32) -> u32 {
    1u32 << (nr & 31)
}

/// `constant_test_bit(nr, addr)` — the compile-time-constant path.
///
/// Mirrors bitops.h lines 20-24: `((1UL << (nr & 31)) & p[nr >> 5]) != 0`.
#[inline]
pub fn constant_test_bit(nr: i32, addr: &[u32]) -> bool {
    (word_mask(nr) & addr[word_index(nr)]) != 0
}

/// `variable_test_bit(nr, addr)` — the run-time path.
///
/// Linux emits `btl %nr, *p` and returns the carry flag. `btl` selects bit
/// `nr` of the bit string at `p` (word `nr>>5`, bit `nr&31`) into CF, which
/// is exactly what `constant_test_bit` computes. So the safe Rust body is
/// identical; the only reason Linux keeps two functions is to let the
/// compiler pick a cheaper encoding when `nr` is a literal.
#[inline]
pub fn variable_test_bit(nr: i32, addr: &[u32]) -> bool {
    (word_mask(nr) & addr[word_index(nr)]) != 0
}

/// `test_bit(nr, addr)` — Linux's macro picks `constant_test_bit` when
/// `nr` is a compile-time constant, otherwise `variable_test_bit`. Both
/// compute the same value, so the Rust seam exposes one function.
#[inline]
pub fn test_bit(nr: i32, addr: &[u32]) -> bool {
    constant_test_bit(nr, addr)
}

/// `set_bit(nr, addr)` — set bit `nr` in the bit string at `addr`.
///
/// Linux emits `btsl %nr, *(u32*)addr`. `btsl` sets word `nr>>5`, bit
/// `nr&31`; we OR in the mask, which is the identical effect.
#[inline]
pub fn set_bit(nr: i32, addr: &mut [u32]) {
    addr[word_index(nr)] |= word_mask(nr);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bit_sets_correct_word_and_mask() {
        let mut bits = [0u32; 4];
        set_bit(0, &mut bits);
        assert_eq!(bits[0], 1);
        set_bit(31, &mut bits);
        assert_eq!(bits[0], 0x8000_0001);
    }

    #[test]
    fn set_bit_crosses_word_boundary() {
        // Bit 32 is bit 0 of the second word; bit 63 is bit 31 of word 1.
        let mut bits = [0u32; 4];
        set_bit(32, &mut bits);
        assert_eq!(bits[1], 0x0000_0001);
        set_bit(63, &mut bits);
        assert_eq!(bits[1], 0x8000_0001);
        // The first word stayed untouched.
        assert_eq!(bits[0], 0);
    }

    #[test]
    fn test_bit_reads_what_set_bit_wrote() {
        let mut bits = [0u32; 4];
        assert!(!test_bit(70, &bits));
        set_bit(70, &mut bits); // word 2, bit 6
        assert!(test_bit(70, &bits));
        assert!(constant_test_bit(70, &bits));
        assert!(variable_test_bit(70, &bits));
        // Neighboring bits remain clear.
        assert!(!test_bit(69, &bits));
        assert!(!test_bit(71, &bits));
    }

    #[test]
    fn constant_and_variable_paths_agree() {
        let mut bits = [0u32; 4];
        set_bit(5, &mut bits);
        set_bit(33, &mut bits);
        set_bit(96, &mut bits);
        for nr in 0..128 {
            assert_eq!(
                constant_test_bit(nr, &bits),
                variable_test_bit(nr, &bits),
                "paths disagree at bit {nr}"
            );
        }
    }
}
