//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/memneq.c
//! test-origin: linux:vendor/linux/lib/crypto/memneq.c
//! Constant-time memory inequality test.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__crypto_memneq", __crypto_memneq_raw as usize, false);
}

fn crypto_memneq_generic(a: &[u8], b: &[u8]) -> usize {
    let mut neq = 0usize;
    for i in 0..a.len() {
        neq |= (a[i] ^ b[i]) as usize;
        core::hint::black_box(neq);
    }
    neq
}

fn crypto_memneq_16(a: &[u8], b: &[u8]) -> usize {
    let mut neq = 0usize;
    neq |= (a[0] ^ b[0]) as usize;
    core::hint::black_box(neq);
    neq |= (a[1] ^ b[1]) as usize;
    core::hint::black_box(neq);
    neq |= (a[2] ^ b[2]) as usize;
    core::hint::black_box(neq);
    neq |= (a[3] ^ b[3]) as usize;
    core::hint::black_box(neq);
    neq |= (a[4] ^ b[4]) as usize;
    core::hint::black_box(neq);
    neq |= (a[5] ^ b[5]) as usize;
    core::hint::black_box(neq);
    neq |= (a[6] ^ b[6]) as usize;
    core::hint::black_box(neq);
    neq |= (a[7] ^ b[7]) as usize;
    core::hint::black_box(neq);
    neq |= (a[8] ^ b[8]) as usize;
    core::hint::black_box(neq);
    neq |= (a[9] ^ b[9]) as usize;
    core::hint::black_box(neq);
    neq |= (a[10] ^ b[10]) as usize;
    core::hint::black_box(neq);
    neq |= (a[11] ^ b[11]) as usize;
    core::hint::black_box(neq);
    neq |= (a[12] ^ b[12]) as usize;
    core::hint::black_box(neq);
    neq |= (a[13] ^ b[13]) as usize;
    core::hint::black_box(neq);
    neq |= (a[14] ^ b[14]) as usize;
    core::hint::black_box(neq);
    neq |= (a[15] ^ b[15]) as usize;
    core::hint::black_box(neq);
    neq
}

pub fn __crypto_memneq(a: &[u8], b: &[u8]) -> usize {
    assert!(b.len() >= a.len());
    match a.len() {
        16 => crypto_memneq_16(a, b),
        _ => crypto_memneq_generic(a, b),
    }
}

pub fn crypto_memneq(a: &[u8], b: &[u8]) -> i32 {
    i32::from(__crypto_memneq(a, b) != 0)
}

pub unsafe extern "C" fn __crypto_memneq_raw(a: *const u8, b: *const u8, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    if a.is_null() || b.is_null() {
        return 1;
    }
    let a = unsafe { core::slice::from_raw_parts(a, size) };
    let b = unsafe { core::slice::from_raw_parts(b, size) };
    __crypto_memneq(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memneq_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/memneq.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/utils.h"
        ));
        assert!(
            source.contains("__crypto_memneq_generic(const void *a, const void *b, size_t size)")
        );
        assert!(source.contains("__crypto_memneq_16(const void *a, const void *b)"));
        assert!(source.contains("switch (size)"));
        assert!(source.contains("case 16:"));
        assert!(source.contains("return __crypto_memneq_16(a, b);"));
        assert!(source.contains("EXPORT_SYMBOL(__crypto_memneq);"));
        assert!(header.contains("return __crypto_memneq(a, b, size) != 0UL ? 1 : 0;"));

        assert_eq!(crypto_memneq(b"same", b"same"), 0);
        assert_eq!(crypto_memneq(b"same", b"some"), 1);
        assert_eq!(__crypto_memneq(&[0u8; 16], &[0u8; 16]), 0);
        let mut b = [0u8; 16];
        b[15] = 1;
        assert_ne!(__crypto_memneq(&[0u8; 16], &b), 0);

        unsafe {
            assert_eq!(__crypto_memneq_raw(b"abc".as_ptr(), b"abc".as_ptr(), 3), 0);
            assert_ne!(__crypto_memneq_raw(b"abc".as_ptr(), b"abd".as_ptr(), 3), 0);
        }
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__crypto_memneq"),
            Some(__crypto_memneq_raw as usize)
        );
    }
}
