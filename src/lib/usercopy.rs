//! linux-parity: complete
//! linux-source: vendor/linux/lib/usercopy.c
//! test-origin: linux:vendor/linux/lib/usercopy.c
//! Out-of-line usercopy helpers and zeroed-buffer check.

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::EFAULT;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::fault_inject_usercopy::should_fail_usercopy;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("_copy_from_user", _copy_from_user as usize, false);
    export_symbol_once("_copy_to_user", _copy_to_user as usize, false);
    export_symbol_once("check_zeroed_user", check_zeroed_user as usize, false);
}

pub fn check_zeroed_user_bytes(bytes: &[u8]) -> i32 {
    check_zeroed_kernel_bytes(bytes)
}

pub unsafe extern "C" fn _copy_from_user(to: *mut u8, from: *const u8, n: usize) -> usize {
    let mut res = n;

    if should_fail_usercopy() {
        unsafe { core::ptr::write_bytes(to, 0, n) };
        return n;
    }
    if !uaccess::access_ok(from as u64, n as u64) {
        unsafe { core::ptr::write_bytes(to, 0, n) };
        return n;
    }

    res = unsafe { uaccess::copy_from_user(to, from, n) };
    if res == 0 {
        return 0;
    }

    unsafe { core::ptr::write_bytes(to.add(n - res), 0, res) };
    res
}

pub unsafe extern "C" fn _copy_to_user(to: *mut u8, from: *const u8, n: usize) -> usize {
    if should_fail_usercopy() {
        return n;
    }
    if uaccess::access_ok(to as u64, n as u64) {
        unsafe { uaccess::copy_to_user(to, from, n) }
    } else {
        n
    }
}

pub unsafe extern "C" fn check_zeroed_user(from: *const u8, size: usize) -> i32 {
    if size == 0 {
        return 1;
    }

    let align = (from as usize) % core::mem::size_of::<usize>();
    let from = (from as usize).wrapping_sub(align) as *const u8;
    let size = match size.checked_add(align) {
        Some(size) => size,
        None => return -EFAULT,
    };

    if !uaccess::access_ok(from as u64, size as u64) {
        return -EFAULT;
    }

    let mut val = match unsafe { get_user_ulong(from as *const usize) } {
        Ok(val) => val,
        Err(_) => return -EFAULT,
    };
    if align != 0 {
        val &= !aligned_byte_mask(align);
    }

    let mut cursor = from;
    let mut remaining = size;
    while remaining > core::mem::size_of::<usize>() {
        if val != 0 {
            return 0;
        }

        cursor = unsafe { cursor.add(core::mem::size_of::<usize>()) };
        remaining -= core::mem::size_of::<usize>();

        val = match unsafe { get_user_ulong(cursor as *const usize) } {
            Ok(val) => val,
            Err(_) => return -EFAULT,
        };
    }

    if remaining < core::mem::size_of::<usize>() {
        val &= aligned_byte_mask(remaining);
    }

    (val == 0) as i32
}

#[inline]
const fn aligned_byte_mask(n: usize) -> usize {
    if n == 0 {
        0
    } else if n >= core::mem::size_of::<usize>() {
        usize::MAX
    } else {
        (1usize << (8 * n)) - 1
    }
}

unsafe fn get_user_ulong(from: *const usize) -> Result<usize, i32> {
    if from.is_null() {
        return Err(-EFAULT);
    }
    #[cfg(target_pointer_width = "64")]
    {
        unsafe { uaccess::get_user_u64(from as *const u64).map(|val| val as usize) }
    }
    #[cfg(target_pointer_width = "32")]
    {
        unsafe { uaccess::get_user_u32(from as *const u32).map(|val| val as usize) }
    }
}

fn check_zeroed_kernel_bytes(bytes: &[u8]) -> i32 {
    if bytes.is_empty() {
        return 1;
    }

    let align = (bytes.as_ptr() as usize) % core::mem::size_of::<usize>();
    let mut word = 0usize;
    let mut offset = 0usize;

    while offset < bytes.len() {
        let byte = bytes[offset] as usize;
        let shift = 8 * ((align + offset) % core::mem::size_of::<usize>());
        word |= byte << shift;

        if (align + offset + 1) % core::mem::size_of::<usize>() == 0 {
            if word != 0 {
                return 0;
            }
            word = 0;
        }
        offset += 1;
    }

    if word == 0 { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usercopy_exports_and_zero_check_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/usercopy.c"
        ));
        assert!(source.contains("unsigned long _copy_from_user(void *to"));
        assert!(source.contains("return _inline_copy_from_user(to, from, n);"));
        assert!(source.contains("unsigned long _copy_to_user(void __user *to"));
        assert!(source.contains("int check_zeroed_user(const void __user *from, size_t size)"));
        assert!(source.contains("if (unlikely(size == 0))"));
        assert!(source.contains("from -= align;"));
        assert!(source.contains("unsafe_get_user(val, (unsigned long __user *) from, err_fault);"));
        assert!(source.contains("val &= ~aligned_byte_mask(align);"));
        assert!(source.contains("while (size > sizeof(unsigned long))"));
        assert!(source.contains("val &= aligned_byte_mask(size);"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("EXPORT_SYMBOL(check_zeroed_user);"));

        assert_eq!(check_zeroed_user_bytes(&[]), 1);
        assert_eq!(check_zeroed_user_bytes(&[0, 0, 0]), 1);
        assert_eq!(check_zeroed_user_bytes(&[0, 1, 0]), 0);
        assert_eq!(
            unsafe { check_zeroed_user((1u64 << 47) as *const u8, 4) },
            -EFAULT
        );

        let src = [1u8, 2, 3];
        let mut dst = [0u8; 3];
        assert_eq!(
            unsafe { _copy_from_user(dst.as_mut_ptr(), src.as_ptr(), src.len()) },
            0
        );
        assert_eq!(dst, src);
        assert_eq!(unsafe { check_zeroed_user(dst.as_ptr(), dst.len()) }, 0);

        let zeroed = [0u8; 3];
        assert_eq!(
            unsafe { check_zeroed_user(zeroed.as_ptr(), zeroed.len()) },
            1
        );
    }

    #[test]
    fn exported_usercopy_rejects_invalid_user_pointers() {
        let invalid = (1u64 << 47) as *const u8;
        let mut dst = [0u8; 16];

        assert_eq!(
            unsafe { _copy_from_user(dst.as_mut_ptr(), invalid, dst.len()) },
            dst.len()
        );
        assert_eq!(
            unsafe { _copy_to_user(invalid as *mut u8, dst.as_ptr(), dst.len()) },
            dst.len()
        );
        assert_eq!(unsafe { check_zeroed_user(invalid, dst.len()) }, -EFAULT);
    }

    #[test]
    fn copy_from_user_zeroes_uncopied_tail_on_fail_path() {
        crate::lib::fault_inject_usercopy::reset_for_test();

        let invalid = (1u64 << 47) as *const u8;
        let mut dst = [0xffu8; 8];
        assert_eq!(
            unsafe { _copy_from_user(dst.as_mut_ptr(), invalid, dst.len()) },
            dst.len()
        );
        assert_eq!(dst, [0; 8]);

        let mut dst = [0xffu8; 8];
        crate::lib::fault_inject_usercopy::setup_fail_usercopy("probability=100");
        assert_eq!(
            unsafe { _copy_from_user(dst.as_mut_ptr(), invalid, dst.len()) },
            dst.len()
        );
        assert_eq!(dst, [0; 8]);
        crate::lib::fault_inject_usercopy::reset_for_test();
    }

    #[test]
    fn copy_to_user_honors_fault_injection_without_touching_destination() {
        crate::lib::fault_inject_usercopy::reset_for_test();

        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        crate::lib::fault_inject_usercopy::setup_fail_usercopy("probability=100");
        assert_eq!(
            unsafe { _copy_to_user(dst.as_mut_ptr(), src.as_ptr(), src.len()) },
            src.len()
        );
        assert_eq!(dst, [0; 4]);
        crate::lib::fault_inject_usercopy::reset_for_test();
    }
}
