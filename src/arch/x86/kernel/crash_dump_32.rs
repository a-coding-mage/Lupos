//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/crash_dump_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/crash_dump_32.c
//! 32-bit crash dump old-memory copying helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/crash_dump_32.c

use crate::arch::x86::mm::paging::{PAGE_SHIFT, PAGE_SIZE};
use crate::include::uapi::errno::{EFAULT, EINVAL};

pub const NONPAE_MAX_PFN: u64 = 0x000f_ffff;

pub const fn is_crashed_pfn_valid(pfn: u64, pae_enabled: bool) -> bool {
    pae_enabled || pfn <= NONPAE_MAX_PFN
}

pub fn copy_oldmem_page(
    memory: &[u8],
    pfn: u64,
    csize: usize,
    offset: usize,
    out: &mut [u8],
) -> Result<usize, i32> {
    if csize == 0 {
        return Ok(0);
    }
    if offset as u64 >= PAGE_SIZE || csize > out.len() {
        return Err(EINVAL);
    }
    let start = ((pfn << PAGE_SHIFT) as usize)
        .checked_add(offset)
        .ok_or(EFAULT)?;
    let end = start.checked_add(csize).ok_or(EFAULT)?;
    if end > memory.len() {
        return Err(EFAULT);
    }
    out[..csize].copy_from_slice(&memory[start..end]);
    Ok(csize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_pae_rejects_pfn_that_overflows_32bit_physical_address() {
        assert!(is_crashed_pfn_valid(NONPAE_MAX_PFN, false));
        assert!(!is_crashed_pfn_valid(NONPAE_MAX_PFN + 1, false));
        assert!(is_crashed_pfn_valid(NONPAE_MAX_PFN + 1, true));
    }

    #[test]
    fn copy_oldmem_page_copies_from_pfn_plus_offset() {
        let memory = [0x11u8; 8192];
        let mut out = [0u8; 4];
        assert_eq!(copy_oldmem_page(&memory, 1, 4, 8, &mut out), Ok(4));
        assert_eq!(out, [0x11; 4]);
    }

    #[test]
    fn copy_oldmem_page_validates_bounds() {
        let memory = [0u8; 4096];
        let mut out = [0u8; 4];
        assert_eq!(
            copy_oldmem_page(&memory, 0, 4, PAGE_SIZE as usize, &mut out),
            Err(EINVAL)
        );
        assert_eq!(copy_oldmem_page(&memory, 1, 4, 0, &mut out), Err(EFAULT));
    }
}
