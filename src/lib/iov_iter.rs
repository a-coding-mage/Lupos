//! linux-parity: partial
//! linux-source: vendor/linux/lib/iov_iter.c
//! Minimal iov_iter ABI exports used by Linux-built modules.

use core::ffi::c_void;

use crate::arch::x86::kernel::uaccess::{copy_from_user, copy_to_user};
use crate::include::uapi::errno::{EFAULT, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};

const ITER_UBUF: u8 = 0;
const ITER_IOVEC: u8 = 1;
const ITER_KVEC: u8 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinuxIoVec {
    base: *mut u8,
    len: usize,
}

#[repr(C)]
pub(crate) struct LinuxIovIter {
    iter_type: u8,
    nofault: bool,
    data_source: bool,
    _pad: [u8; 5],
    iov_offset: usize,
    ptr: *mut c_void,
    count: usize,
    nr_segs: usize,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("import_ubuf", linux_import_ubuf as usize, true);
    export_symbol_once("iov_iter_init", linux_iov_iter_init as usize, false);
    export_symbol_once("iov_iter_kvec", linux_iov_iter_kvec as usize, false);
    export_symbol_once("iov_iter_advance", linux_iov_iter_advance as usize, false);
    export_symbol_once("iov_iter_revert", linux_iov_iter_revert as usize, false);
    export_symbol_once(
        "iov_iter_single_seg_count",
        linux_iov_iter_single_seg_count as usize,
        false,
    );
    export_symbol_once(
        "iov_iter_get_pages_alloc2",
        linux_iov_iter_get_pages_alloc2 as usize,
        false,
    );
    export_symbol_once("_copy_to_iter", linux_copy_to_iter as usize, false);
    export_symbol_once("_copy_from_iter", linux_copy_from_iter as usize, false);
    export_symbol_once(
        "_copy_from_iter_nocache",
        linux_copy_from_iter as usize,
        false,
    );
}

unsafe fn iter_current_buffer(iter: &LinuxIovIter) -> Option<(*mut u8, usize, bool)> {
    match iter.iter_type {
        ITER_UBUF => Some((
            unsafe { iter.ptr.cast::<u8>().add(iter.iov_offset) },
            iter.count,
            true,
        )),
        ITER_IOVEC | ITER_KVEC if iter.nr_segs == 1 && !iter.ptr.is_null() => {
            let iov = unsafe { iter.ptr.cast::<LinuxIoVec>().read() };
            if iter.iov_offset > iov.len {
                None
            } else {
                Some((
                    unsafe { iov.base.add(iter.iov_offset) },
                    iov.len.saturating_sub(iter.iov_offset).min(iter.count),
                    iter.iter_type == ITER_IOVEC,
                ))
            }
        }
        _ => None,
    }
}

unsafe fn advance_iter(iter: &mut LinuxIovIter, copied: usize) {
    iter.iov_offset = iter.iov_offset.saturating_add(copied);
    iter.count = iter.count.saturating_sub(copied);
}

unsafe fn revert_iter(iter: &mut LinuxIovIter, unroll: usize) {
    let moved = iter.iov_offset.min(unroll);
    iter.iov_offset -= moved;
    iter.count = iter.count.saturating_add(moved);
}

/// `import_ubuf` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_import_ubuf(
    direction: i32,
    buf: *mut c_void,
    len: usize,
    iter: *mut LinuxIovIter,
) -> i32 {
    if iter.is_null() || direction & !1 != 0 {
        return -EINVAL;
    }
    if buf.is_null() && len != 0 {
        return -EFAULT;
    }
    unsafe {
        iter.write(LinuxIovIter {
            iter_type: ITER_UBUF,
            nofault: false,
            data_source: direction != 0,
            _pad: [0; 5],
            iov_offset: 0,
            ptr: buf,
            count: len,
            nr_segs: 1,
        });
    }
    0
}

/// `iov_iter_init` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_init(
    iter: *mut LinuxIovIter,
    direction: u32,
    iov: *const LinuxIoVec,
    nr_segs: usize,
    count: usize,
) {
    if iter.is_null() {
        return;
    }
    unsafe {
        iter.write(LinuxIovIter {
            iter_type: ITER_IOVEC,
            nofault: false,
            data_source: direction != 0,
            _pad: [0; 5],
            iov_offset: 0,
            ptr: iov.cast_mut().cast::<c_void>(),
            count,
            nr_segs,
        });
    }
}

/// `iov_iter_kvec` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_kvec(
    iter: *mut LinuxIovIter,
    direction: u32,
    kvec: *const LinuxIoVec,
    nr_segs: usize,
    count: usize,
) {
    if iter.is_null() {
        return;
    }
    unsafe {
        iter.write(LinuxIovIter {
            iter_type: ITER_KVEC,
            nofault: false,
            data_source: direction != 0,
            _pad: [0; 5],
            iov_offset: 0,
            ptr: kvec.cast_mut().cast::<c_void>(),
            count,
            nr_segs,
        });
    }
}

/// `iov_iter_advance` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_advance(iter: *mut LinuxIovIter, bytes: usize) {
    if iter.is_null() {
        return;
    }
    unsafe { advance_iter(&mut *iter, bytes) };
}

/// `iov_iter_revert` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_revert(iter: *mut LinuxIovIter, bytes: usize) {
    if iter.is_null() {
        return;
    }
    unsafe { revert_iter(&mut *iter, bytes) };
}

/// `iov_iter_single_seg_count` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_single_seg_count(iter: *const LinuxIovIter) -> usize {
    if iter.is_null() {
        return 0;
    }
    let iter_ref = unsafe { &*iter };
    unsafe { iter_current_buffer(iter_ref) }
        .map(|(_, available, _)| available)
        .unwrap_or(0)
}

/// `iov_iter_get_pages_alloc2` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_iov_iter_get_pages_alloc2(
    iter: *mut LinuxIovIter,
    pages: *mut *mut *mut c_void,
    maxsize: usize,
    start: *mut usize,
) -> isize {
    if iter.is_null() || pages.is_null() || start.is_null() || maxsize == 0 {
        return -EFAULT as isize;
    }
    unsafe {
        pages.write(core::ptr::null_mut());
        start.write(0);
    }
    -EFAULT as isize
}

/// `_copy_to_iter` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_copy_to_iter(
    addr: *const c_void,
    bytes: usize,
    iter: *mut LinuxIovIter,
) -> usize {
    if addr.is_null() || iter.is_null() || bytes == 0 {
        return 0;
    }
    let iter_ref = unsafe { &mut *iter };
    let Some((dst, available, user_backed)) = (unsafe { iter_current_buffer(iter_ref) }) else {
        return 0;
    };
    let requested = bytes.min(available);
    let copied = if user_backed {
        let not_copied = unsafe { copy_to_user(dst, addr.cast::<u8>(), requested) };
        requested.saturating_sub(not_copied)
    } else {
        unsafe { core::ptr::copy_nonoverlapping(addr.cast::<u8>(), dst, requested) };
        requested
    };
    unsafe { advance_iter(iter_ref, copied) };
    copied
}

/// `_copy_from_iter` - `vendor/linux/lib/iov_iter.c`.
pub unsafe extern "C" fn linux_copy_from_iter(
    addr: *mut c_void,
    bytes: usize,
    iter: *mut LinuxIovIter,
) -> usize {
    if addr.is_null() || iter.is_null() || bytes == 0 {
        return 0;
    }
    let iter_ref = unsafe { &mut *iter };
    let Some((src, available, user_backed)) = (unsafe { iter_current_buffer(iter_ref) }) else {
        return 0;
    };
    let requested = bytes.min(available);
    let copied = if user_backed {
        let not_copied = unsafe { copy_from_user(addr.cast::<u8>(), src.cast_const(), requested) };
        requested.saturating_sub(not_copied)
    } else {
        unsafe { core::ptr::copy_nonoverlapping(src.cast_const(), addr.cast::<u8>(), requested) };
        requested
    };
    unsafe { advance_iter(iter_ref, copied) };
    copied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iov_iter_layout_matches_staged_linux() {
        assert_eq!(core::mem::size_of::<LinuxIovIter>(), 40);
        assert_eq!(core::mem::offset_of!(LinuxIovIter, iov_offset), 8);
        assert_eq!(core::mem::offset_of!(LinuxIovIter, ptr), 16);
        assert_eq!(core::mem::offset_of!(LinuxIovIter, count), 24);
        assert_eq!(core::mem::offset_of!(LinuxIovIter, nr_segs), 32);
    }

    #[test]
    fn iov_iter_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("_copy_to_iter"),
            Some(linux_copy_to_iter as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("import_ubuf"),
            Some(linux_import_ubuf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("iov_iter_kvec"),
            Some(linux_iov_iter_kvec as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("iov_iter_advance"),
            Some(linux_iov_iter_advance as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("iov_iter_revert"),
            Some(linux_iov_iter_revert as usize)
        );
    }

    #[test]
    fn iov_iter_advance_revert_and_single_seg_count_track_count() {
        let mut data = [0u8; 8];
        let iov = LinuxIoVec {
            base: data.as_mut_ptr(),
            len: data.len(),
        };
        let mut iter = LinuxIovIter {
            iter_type: ITER_KVEC,
            nofault: false,
            data_source: false,
            _pad: [0; 5],
            iov_offset: 0,
            ptr: (&iov as *const LinuxIoVec).cast_mut().cast(),
            count: data.len(),
            nr_segs: 1,
        };

        unsafe {
            assert_eq!(linux_iov_iter_single_seg_count(&iter), 8);
            linux_iov_iter_advance(&mut iter, 3);
            assert_eq!(linux_iov_iter_single_seg_count(&iter), 5);
            linux_iov_iter_revert(&mut iter, 2);
            assert_eq!(linux_iov_iter_single_seg_count(&iter), 7);
        }
    }

    #[test]
    fn iov_iter_get_pages_alloc2_fails_closed_without_page_extraction() {
        let mut iter = LinuxIovIter {
            iter_type: ITER_KVEC,
            nofault: false,
            data_source: false,
            _pad: [0; 5],
            iov_offset: 0,
            ptr: core::ptr::null_mut(),
            count: 0,
            nr_segs: 0,
        };
        let mut pages = core::ptr::null_mut();
        let mut start = 99usize;
        let ret =
            unsafe { linux_iov_iter_get_pages_alloc2(&mut iter, &mut pages, 4096, &mut start) };
        assert_eq!(ret, -EFAULT as isize);
        assert!(pages.is_null());
        assert_eq!(start, 0);
    }
}
