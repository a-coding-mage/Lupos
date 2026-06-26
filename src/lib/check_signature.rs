//! linux-parity: complete
//! linux-source: vendor/linux/lib/check_signature.c
//! test-origin: linux:vendor/linux/lib/check_signature.c
//! BIOS signature comparison helper.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("check_signature", check_signature as usize, false);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn check_signature(
    io_addr: *const u8,
    signature: *const u8,
    length: i32,
) -> i32 {
    if io_addr.is_null() || signature.is_null() || length < 0 {
        return 0;
    }
    let mut index = 0isize;
    while index < length as isize {
        let io_byte = unsafe { core::ptr::read_volatile(io_addr.offset(index)) };
        let sig_byte = unsafe { *signature.offset(index) };
        if io_byte != sig_byte {
            return 0;
        }
        index += 1;
    }
    1
}

pub fn check_signature_bytes(io: &[u8], signature: &[u8]) -> i32 {
    if signature.len() > io.len() {
        return 0;
    }
    let mut index = 0usize;
    while index < signature.len() {
        if io[index] != signature[index] {
            return 0;
        }
        index += 1;
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_signature_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/check_signature.c"
        ));
        assert!(source.contains("#include <linux/io.h>"));
        assert!(source.contains("readb(io_addr) != *signature"));
        assert!(source.contains("io_addr++;"));
        assert!(source.contains("signature++;"));
        assert!(source.contains("EXPORT_SYMBOL(check_signature);"));
        assert_eq!(check_signature_bytes(b"BIOS123", b"BIOS"), 1);
        assert_eq!(check_signature_bytes(b"BIOS123", b"BOOS"), 0);
        assert_eq!(check_signature_bytes(b"BIO", b"BIOS"), 0);
    }

    #[test]
    fn check_signature_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("check_signature"),
            Some(check_signature as usize)
        );
    }
}
