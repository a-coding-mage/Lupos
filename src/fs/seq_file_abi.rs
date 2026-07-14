//! linux-parity: partial
//! linux-source: vendor/linux/fs/seq_file.c
//! Minimal seq-file ABI exports for Linux-built modules.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("seq_open", linux_seq_open as usize, false);
    export_symbol_once("seq_read", linux_seq_read as usize, false);
    export_symbol_once("seq_read_iter", linux_seq_read_iter as usize, false);
    export_symbol_once("seq_lseek", linux_seq_lseek as usize, false);
    export_symbol_once("seq_release", linux_seq_release as usize, false);
    export_symbol_once("seq_printf", linux_seq_printf as usize, false);
    export_symbol_once("seq_putc", linux_seq_putc as usize, false);
    export_symbol_once("__seq_puts", linux___seq_puts as usize, false);
    export_symbol_once("seq_write", linux_seq_write as usize, false);
    export_symbol_once(
        "seq_put_decimal_ull",
        linux_seq_put_decimal_ull as usize,
        false,
    );
    export_symbol_once(
        "seq_put_decimal_ll",
        linux_seq_put_decimal_ll as usize,
        false,
    );
    export_symbol_once("single_open", linux_single_open as usize, false);
    export_symbol_once("single_open_size", linux_single_open_size as usize, false);
    export_symbol_once("single_release", linux_single_release as usize, false);
    export_symbol_once(
        "seq_release_private",
        linux_seq_release_private as usize,
        false,
    );
}

/// `seq_open` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_open(_file: *mut c_void, _op: *const c_void) -> i32 {
    0
}

/// `seq_read` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_read(
    _file: *mut c_void,
    _buf: *mut c_char,
    _size: usize,
    ppos: *mut i64,
) -> isize {
    if !ppos.is_null() {
        unsafe {
            *ppos = (*ppos).max(0);
        }
    }
    0
}

/// `seq_read_iter` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_read_iter(_iocb: *mut c_void, _iter: *mut c_void) -> isize {
    0
}

/// `seq_lseek` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_lseek(_file: *mut c_void, offset: i64, whence: i32) -> i64 {
    match whence {
        0 | 1 if offset >= 0 => offset,
        _ => -(EINVAL as i64),
    }
}

/// `seq_release` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_release(_inode: *mut c_void, _file: *mut c_void) -> i32 {
    0
}

/// `seq_release_private` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_release_private(inode: *mut c_void, file: *mut c_void) -> i32 {
    unsafe { linux_seq_release(inode, file) }
}

/// `seq_printf` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_printf(_m: *mut c_void, _fmt: *const c_char) -> i32 {
    0
}

/// `seq_putc` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_putc(_m: *mut c_void, _c: c_char) {}

/// `__seq_puts` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux___seq_puts(_m: *mut c_void, _s: *const c_char) {}

/// `seq_write` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_write(
    _seq: *mut c_void,
    _data: *const c_void,
    _len: usize,
) -> i32 {
    0
}

/// `seq_put_decimal_ull` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_put_decimal_ull(
    _m: *mut c_void,
    _delimiter: *const c_char,
    _num: u64,
) {
}

/// `seq_put_decimal_ll` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_seq_put_decimal_ll(
    _m: *mut c_void,
    _delimiter: *const c_char,
    _num: i64,
) {
}

/// `single_open` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_single_open(
    _file: *mut c_void,
    _show: *const c_void,
    _data: *mut c_void,
) -> i32 {
    0
}

/// `single_open_size` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_single_open_size(
    file: *mut c_void,
    show: *const c_void,
    data: *mut c_void,
    _size: usize,
) -> i32 {
    unsafe { linux_single_open(file, show, data) }
}

/// `single_release` - `vendor/linux/fs/seq_file.c`.
pub unsafe extern "C" fn linux_single_release(inode: *mut c_void, file: *mut c_void) -> i32 {
    unsafe { linux_seq_release(inode, file) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_file_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("seq_release"),
            Some(linux_seq_release as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("single_open_size"),
            Some(linux_single_open_size as usize)
        );
    }
}
