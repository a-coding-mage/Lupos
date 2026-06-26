//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/crash_dump_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/crash_dump_64.c
//! 64-bit crash dump old-memory copying helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/crash_dump_64.c

use crate::arch::x86::mm::paging::{PAGE_SHIFT, PAGE_SIZE};
use crate::include::uapi::errno::{EFAULT, EINVAL};

pub fn copy_oldmem_page(
    memory: &[u8],
    pfn: u64,
    csize: usize,
    offset: usize,
    out: &mut [u8],
) -> Result<usize, i32> {
    copy_oldmem_page_encrypted(memory, pfn, csize, offset, out, false)
}

pub fn copy_oldmem_page_encrypted(
    memory: &[u8],
    pfn: u64,
    csize: usize,
    offset: usize,
    out: &mut [u8],
    _encrypted: bool,
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

pub fn elfcorehdr_read(memory: &[u8], addr: u64, out: &mut [u8]) -> Result<usize, i32> {
    let start = addr as usize;
    let end = start.checked_add(out.len()).ok_or(EFAULT)?;
    if end > memory.len() {
        return Err(EFAULT);
    }
    out.copy_from_slice(&memory[start..end]);
    Ok(out.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_oldmem_page_handles_plain_and_encrypted_paths() {
        let mut memory = [0u8; 8192];
        memory[4096 + 7] = 0xaa;
        let mut out = [0u8; 1];
        assert_eq!(
            copy_oldmem_page_encrypted(&memory, 1, 1, 7, &mut out, true),
            Ok(1)
        );
        assert_eq!(out[0], 0xaa);
    }

    #[test]
    fn copy_oldmem_page_rejects_bad_offset_and_bounds() {
        let memory = [0u8; 4096];
        let mut out = [0u8; 2];
        assert_eq!(
            copy_oldmem_page(&memory, 0, 1, PAGE_SIZE as usize, &mut out),
            Err(EINVAL)
        );
        assert_eq!(copy_oldmem_page(&memory, 1, 1, 0, &mut out), Err(EFAULT));
    }

    #[test]
    fn elfcorehdr_read_copies_exact_requested_range() {
        let memory = [1u8, 2, 3, 4];
        let mut out = [0u8; 2];
        assert_eq!(elfcorehdr_read(&memory, 1, &mut out), Ok(2));
        assert_eq!(out, [2, 3]);
    }
}
