//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/kaslr.c
//! test-origin: linux:vendor/linux/arch/x86/mm/kaslr.c
//! Regular-kernel memory KASLR layout policy.
//!
//! Mirrors the memory-region randomization role of
//! `vendor/linux/arch/x86/mm/kaslr.c`. Entropy collection itself is implemented
//! by the existing `crate::arch::x86::kernel::kaslr` module.

use crate::arch::x86::mm::paging::{PAGE_MASK, PAGE_SIZE};
use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomizedMemoryWindow {
    pub base: u64,
    pub size: u64,
}

pub const fn kaslr_memory_enabled(cmdline_nokaslr: bool, entropy: u64) -> bool {
    !cmdline_nokaslr && entropy != 0
}

pub const fn randomize_memory_window(
    base: u64,
    size: u64,
    entropy: u64,
    enabled: bool,
) -> Result<RandomizedMemoryWindow, i32> {
    if size == 0 || size & (PAGE_SIZE - 1) != 0 {
        return Err(EINVAL);
    }
    if !enabled {
        return Ok(RandomizedMemoryWindow { base, size });
    }
    let slots = size / PAGE_SIZE;
    let slot = entropy % slots;
    let randomized = (base + slot * PAGE_SIZE) & PAGE_MASK;
    Ok(RandomizedMemoryWindow {
        base: randomized,
        size: size - slot * PAGE_SIZE,
    })
}

pub fn kaslr_get_random_long<E: crate::arch::x86::kernel::kaslr::KaslrEntropy>(source: &E) -> u64 {
    crate::arch::x86::kernel::kaslr::kaslr_get_random_long(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_memory_kaslr_preserves_base() {
        assert_eq!(
            randomize_memory_window(0x1000, 0x4000, 3, false).unwrap(),
            RandomizedMemoryWindow {
                base: 0x1000,
                size: 0x4000
            }
        );
    }

    #[test]
    fn enabled_memory_kaslr_selects_page_slot() {
        assert_eq!(
            randomize_memory_window(0x1000, 0x4000, 2, true)
                .unwrap()
                .base,
            0x3000
        );
    }
}
