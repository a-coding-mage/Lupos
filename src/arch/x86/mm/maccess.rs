//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/maccess.c
//! test-origin: linux:vendor/linux/arch/x86/mm/maccess.c
//! Machine-check-safe kernel memory access helpers.
//!
//! Mirrors the no-fault copy surface from `vendor/linux/arch/x86/mm/maccess.c`.
//! The host-testable implementation validates null pointers and copies byte
//! ranges without taking ownership of either side.

use crate::include::uapi::errno::{EFAULT, EINVAL};

pub unsafe fn copy_from_kernel_nofault(
    dst: *mut u8,
    src: *const u8,
    len: usize,
) -> Result<(), i32> {
    if len == 0 {
        return Ok(());
    }
    if dst.is_null() || src.is_null() {
        return Err(EFAULT);
    }
    if (dst as usize).checked_add(len).is_none() || (src as usize).checked_add(len).is_none() {
        return Err(EINVAL);
    }
    unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    Ok(())
}

pub unsafe fn copy_to_kernel_nofault(dst: *mut u8, src: *const u8, len: usize) -> Result<(), i32> {
    unsafe { copy_from_kernel_nofault(dst, src, len) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_from_kernel_nofault_copies_bytes() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        unsafe { copy_from_kernel_nofault(dst.as_mut_ptr(), src.as_ptr(), src.len()).unwrap() };
        assert_eq!(dst, src);
    }

    #[test]
    fn copy_from_kernel_nofault_rejects_null() {
        let mut dst = [0u8; 4];
        assert_eq!(
            unsafe { copy_from_kernel_nofault(dst.as_mut_ptr(), core::ptr::null(), 4) },
            Err(EFAULT)
        );
    }
}
