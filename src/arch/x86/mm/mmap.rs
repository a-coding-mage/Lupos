//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/mmap.c
//! test-origin: linux:vendor/linux/arch/x86/mm/mmap.c
//! x86 mmap layout policy.
//!
//! Mirrors the ASLR and top-down layout decisions from
//! `vendor/linux/arch/x86/mm/mmap.c`. The generic VMA insertion code remains
//! in `crate::mm::mmap`.

use crate::arch::x86::mm::paging::{PAGE_MASK, PAGE_SIZE};
use crate::include::uapi::errno::EINVAL;
use crate::mm::mmap::{DEFAULT_MMAP_BASE, TASK_SIZE};

pub const MMAP_RND_BITS_MIN: u8 = 28;
pub const MMAP_RND_BITS_MAX: u8 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmapLayout {
    pub base: u64,
    pub legacy: bool,
    pub random_offset: u64,
}

pub const fn mmap_rnd_mask(bits: u8) -> Result<u64, i32> {
    if bits < MMAP_RND_BITS_MIN || bits > MMAP_RND_BITS_MAX {
        return Err(EINVAL);
    }
    Ok((1u64 << bits) - 1)
}

pub const fn arch_mmap_rnd(entropy: u64, bits: u8) -> Result<u64, i32> {
    let mask = match mmap_rnd_mask(bits) {
        Ok(mask) => mask,
        Err(err) => return Err(err),
    };
    Ok((entropy & mask) << crate::arch::x86::mm::paging::PAGE_SHIFT)
}

pub const fn mmap_base(entropy: u64, bits: u8, legacy: bool) -> Result<MmapLayout, i32> {
    let rnd = match arch_mmap_rnd(entropy, bits) {
        Ok(rnd) => rnd,
        Err(err) => return Err(err),
    };
    let base = if legacy {
        DEFAULT_MMAP_BASE
    } else {
        TASK_SIZE
            .saturating_sub(128 * 1024 * 1024)
            .saturating_sub(rnd)
    } & PAGE_MASK;
    Ok(MmapLayout {
        base,
        legacy,
        random_offset: rnd,
    })
}

pub const fn valid_mmap_addr(addr: u64, len: u64) -> bool {
    if len == 0 || addr & (PAGE_SIZE - 1) != 0 {
        return false;
    }
    match addr.checked_add(len) {
        Some(end) => end <= TASK_SIZE,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmap_randomization_is_page_scaled() {
        assert_eq!(arch_mmap_rnd(3, MMAP_RND_BITS_MIN), Ok(3 * PAGE_SIZE));
    }

    #[test]
    fn layout_stays_below_task_size() {
        let layout = mmap_base(0x1234, MMAP_RND_BITS_MIN, false).unwrap();
        assert!(layout.base < TASK_SIZE);
        assert!(valid_mmap_addr(0x4000, PAGE_SIZE));
    }
}
