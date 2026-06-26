//! linux-parity: complete
//! linux-source: vendor/linux/lib/iomap_copy.c
//! test-origin: linux:vendor/linux/lib/iomap_copy.c
//! Raw MMIO copy helpers.

use core::ffi::c_void;
use core::ptr::{read_volatile, write_volatile};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__iowrite32_copy", __iowrite32_copy as usize, true);
    export_symbol_once("__ioread32_copy", __ioread32_copy as usize, true);
    export_symbol_once("__iowrite64_copy", __iowrite64_copy as usize, true);
}

pub unsafe extern "C" fn __iowrite32_copy(to: *mut c_void, from: *const c_void, count: usize) {
    if to.is_null() || from.is_null() {
        return;
    }
    let mut dst = to.cast::<u32>();
    let mut src = from.cast::<u32>();
    let end = unsafe { src.add(count) };
    while src < end {
        unsafe {
            write_volatile(dst, read_volatile(src));
            src = src.add(1);
            dst = dst.add(1);
        }
    }
}

pub unsafe extern "C" fn __ioread32_copy(to: *mut c_void, from: *const c_void, count: usize) {
    if to.is_null() || from.is_null() {
        return;
    }
    let mut dst = to.cast::<u32>();
    let mut src = from.cast::<u32>();
    let end = unsafe { src.add(count) };
    while src < end {
        unsafe {
            write_volatile(dst, read_volatile(src));
            src = src.add(1);
            dst = dst.add(1);
        }
    }
}

pub unsafe extern "C" fn __iowrite64_copy(to: *mut c_void, from: *const c_void, count: usize) {
    if to.is_null() || from.is_null() {
        return;
    }
    let mut dst = to.cast::<u64>();
    let mut src = from.cast::<u64>();
    let end = unsafe { src.add(count) };
    while src < end {
        unsafe {
            write_volatile(dst, read_volatile(src));
            src = src.add(1);
            dst = dst.add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iomap_copy_uses_raw_32_and_64_bit_units() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/iomap_copy.c"
        ));
        assert!(source.contains("__iowrite32_copy"));
        assert!(source.contains("while (src < end)"));
        assert!(source.contains("__raw_writel(*src++, dst++);"));
        assert!(source.contains("*dst++ = __raw_readl(src++);"));
        assert!(source.contains("__iowrite64_copy"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__iowrite64_copy);"));

        let input32 = [1u32, 2, 3, 4];
        let mut mmio32 = [0u32; 4];
        let mut out32 = [0u32; 4];
        unsafe {
            __iowrite32_copy(mmio32.as_mut_ptr().cast(), input32.as_ptr().cast(), 4);
            __ioread32_copy(out32.as_mut_ptr().cast(), mmio32.as_ptr().cast(), 4);
        }
        assert_eq!(out32, input32);

        let input64 = [0x1122_3344_5566_7788u64, 0x99aa_bbcc_ddee_ff00];
        let mut mmio64 = [0u64; 2];
        unsafe {
            __iowrite64_copy(mmio64.as_mut_ptr().cast(), input64.as_ptr().cast(), 2);
        }
        assert_eq!(mmio64, input64);
    }
}
