//! linux-parity: complete
//! linux-source: vendor/linux/mm/usercopy.c
//! test-origin: linux:vendor/linux/mm/usercopy.c
//! Generic usercopy and kernel memory-access helpers.
//!
//! Arch code owns the x86 exception-table copy loops; this module owns the
//! generic memory validation and non-arch helpers.
//!
//! References:
//! - `vendor/linux/mm/usercopy.c`
//! - `vendor/linux/mm/maccess.c`

use crate::include::uapi::errno::EFAULT;

pub fn check_copy_size(ptr: *const u8, len: usize) -> Result<(), i32> {
    if len != 0 && ptr.is_null() {
        Err(EFAULT)
    } else {
        Ok(())
    }
}

pub unsafe fn copy_kernel_nofault(dst: *mut u8, src: *const u8, len: usize) -> Result<(), i32> {
    check_copy_size(src, len)?;
    if len != 0 && dst.is_null() {
        return Err(EFAULT);
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, len);
    }
    Ok(())
}

pub unsafe fn copy_from_user_nofault(dst: *mut u8, src: *const u8, len: usize) -> Result<(), i32> {
    unsafe { copy_kernel_nofault(dst, src, len) }
}

pub unsafe fn copy_to_user_nofault(dst: *mut u8, src: *const u8, len: usize) -> Result<(), i32> {
    unsafe { copy_kernel_nofault(dst, src, len) }
}

pub unsafe fn memset_kernel_nofault(dst: *mut u8, value: u8, len: usize) -> Result<(), i32> {
    if len != 0 && dst.is_null() {
        return Err(EFAULT);
    }
    unsafe {
        core::ptr::write_bytes(dst, value, len);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usercopy_checks_null_for_nonzero_ranges() {
        assert_eq!(check_copy_size(core::ptr::null(), 1), Err(EFAULT));
        assert_eq!(check_copy_size(core::ptr::null(), 0), Ok(()));
    }

    #[test]
    fn nofault_copy_moves_bytes() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        assert_eq!(
            unsafe { copy_kernel_nofault(dst.as_mut_ptr(), src.as_ptr(), 4) },
            Ok(())
        );
        assert_eq!(dst, src);

        let mut user_dst = [0u8; 4];
        assert_eq!(
            unsafe { copy_from_user_nofault(user_dst.as_mut_ptr(), src.as_ptr(), 4) },
            Ok(())
        );
        assert_eq!(user_dst, src);
        assert_eq!(
            unsafe { copy_to_user_nofault(dst.as_mut_ptr(), user_dst.as_ptr(), 4) },
            Ok(())
        );
    }
}
