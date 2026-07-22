//! linux-parity: partial
//! linux-source: vendor/linux/lib/hexdump.c
//! test-origin: linux:vendor/linux/lib/hexdump.c
//! Hexdump ABI exports used by Linux-built modules.

use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "hex_dump_to_buffer",
        linux_hex_dump_to_buffer as usize,
        false,
    );
    export_symbol_once("print_hex_dump", linux_print_hex_dump as usize, false);
}

/// `hex_dump_to_buffer` - `vendor/linux/lib/hexdump.c`.
pub unsafe extern "C" fn linux_hex_dump_to_buffer(
    _buf: *const c_void,
    _len: usize,
    _rowsize: i32,
    _groupsize: i32,
    linebuf: *mut c_char,
    linebuflen: usize,
    _ascii: bool,
) -> i32 {
    if !linebuf.is_null() && linebuflen != 0 {
        unsafe {
            *linebuf = 0;
        }
    }
    0
}

/// `print_hex_dump` - `vendor/linux/lib/hexdump.c`.
pub unsafe extern "C" fn linux_print_hex_dump(
    _level: *const c_char,
    _prefix: *const c_char,
    _prefix_type: i32,
    _rowsize: i32,
    _groupsize: i32,
    _buf: *const c_void,
    _len: usize,
    _ascii: bool,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexdump_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("print_hex_dump"),
            Some(linux_print_hex_dump as usize)
        );
    }
}
