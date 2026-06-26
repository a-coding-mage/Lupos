//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/utils.c
//! test-origin: linux:vendor/linux/lib/crypto/utils.c
//! Crypto bytewise XOR utility.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CRYPTO_UTILS_MODULE_DESCRIPTION: &str = "Crypto library utility functions";
pub const CRYPTO_XOR_EXPORT_SYMBOL: &str = "__crypto_xor";

const HAVE_EFFICIENT_UNALIGNED_ACCESS: bool =
    cfg!(any(target_arch = "x86", target_arch = "x86_64"));
const CONFIG_64BIT: bool = cfg!(target_pointer_width = "64");

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(CRYPTO_XOR_EXPORT_SYMBOL, __crypto_xor as usize, true);
}

pub fn crypto_xor_cpy(dst: &mut [u8], src1: &[u8], src2: &[u8]) {
    assert!(dst.len() <= src1.len());
    assert!(dst.len() <= src2.len());
    assert!(dst.len() <= u32::MAX as usize);
    unsafe {
        __crypto_xor(
            dst.as_mut_ptr(),
            src1.as_ptr(),
            src2.as_ptr(),
            dst.len() as u32,
        );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CryptoXorTrace {
    pub relalign: usize,
    pub prefix_bytes: usize,
    pub words64: usize,
    pub words32: usize,
    pub words16: usize,
    pub trailing_bytes: usize,
}

pub fn crypto_xor_trace(
    mut dst_addr: usize,
    src1_addr: usize,
    src2_addr: usize,
    mut len: usize,
    efficient_unaligned: bool,
    config_64bit: bool,
) -> CryptoXorTrace {
    let mut trace = CryptoXorTrace::default();

    if !efficient_unaligned {
        let size = core::mem::size_of::<usize>();
        let d = ((dst_addr ^ src1_addr) | (dst_addr ^ src2_addr)) & (size - 1);
        trace.relalign = if d == 0 {
            size
        } else {
            1usize << d.trailing_zeros()
        };

        while (dst_addr & (trace.relalign - 1)) != 0 && len > 0 {
            trace.prefix_bytes += 1;
            dst_addr += 1;
            len -= 1;
        }
    }

    if config_64bit && len >= 8 && (trace.relalign & 7) == 0 {
        trace.words64 = len / 8;
        let bytes = trace.words64 * 8;
        len -= bytes;
    }

    if len >= 4 && (trace.relalign & 3) == 0 {
        trace.words32 = len / 4;
        let bytes = trace.words32 * 4;
        len -= bytes;
    }

    if len >= 2 && (trace.relalign & 1) == 0 {
        trace.words16 = len / 2;
        let bytes = trace.words16 * 2;
        len -= bytes;
    }

    trace.trailing_bytes = len;
    trace
}

unsafe fn xor_bytes(dst: &mut *mut u8, src1: &mut *const u8, src2: &mut *const u8, count: usize) {
    for _ in 0..count {
        unsafe {
            **dst = **src1 ^ **src2;
            *dst = (*dst).add(1);
            *src1 = (*src1).add(1);
            *src2 = (*src2).add(1);
        }
    }
}

unsafe fn xor_words64(dst: &mut *mut u8, src1: &mut *const u8, src2: &mut *const u8, count: usize) {
    for _ in 0..count {
        unsafe {
            let word = core::ptr::read_unaligned(*src1 as *const u64)
                ^ core::ptr::read_unaligned(*src2 as *const u64);
            core::ptr::write_unaligned(*dst as *mut u64, word);
            *dst = (*dst).add(8);
            *src1 = (*src1).add(8);
            *src2 = (*src2).add(8);
        }
    }
}

unsafe fn xor_words32(dst: &mut *mut u8, src1: &mut *const u8, src2: &mut *const u8, count: usize) {
    for _ in 0..count {
        unsafe {
            let word = core::ptr::read_unaligned(*src1 as *const u32)
                ^ core::ptr::read_unaligned(*src2 as *const u32);
            core::ptr::write_unaligned(*dst as *mut u32, word);
            *dst = (*dst).add(4);
            *src1 = (*src1).add(4);
            *src2 = (*src2).add(4);
        }
    }
}

unsafe fn xor_words16(dst: &mut *mut u8, src1: &mut *const u8, src2: &mut *const u8, count: usize) {
    for _ in 0..count {
        unsafe {
            let word = core::ptr::read_unaligned(*src1 as *const u16)
                ^ core::ptr::read_unaligned(*src2 as *const u16);
            core::ptr::write_unaligned(*dst as *mut u16, word);
            *dst = (*dst).add(2);
            *src1 = (*src1).add(2);
            *src2 = (*src2).add(2);
        }
    }
}

pub unsafe extern "C" fn __crypto_xor(
    mut dst: *mut u8,
    mut src1: *const u8,
    mut src2: *const u8,
    len: u32,
) {
    if len == 0 {
        return;
    }
    if dst.is_null() || src1.is_null() || src2.is_null() {
        return;
    }

    let trace = crypto_xor_trace(
        dst as usize,
        src1 as usize,
        src2 as usize,
        len as usize,
        HAVE_EFFICIENT_UNALIGNED_ACCESS,
        CONFIG_64BIT,
    );

    unsafe {
        xor_bytes(&mut dst, &mut src1, &mut src2, trace.prefix_bytes);
        xor_words64(&mut dst, &mut src1, &mut src2, trace.words64);
        xor_words32(&mut dst, &mut src1, &mut src2, trace.words32);
        xor_words16(&mut dst, &mut src1, &mut src2, trace.words16);
        xor_bytes(&mut dst, &mut src1, &mut src2, trace.trailing_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_xor_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/utils.c"
        ));
        assert!(source.contains(
            "void __crypto_xor(u8 *dst, const u8 *src1, const u8 *src2, unsigned int len)"
        ));
        assert!(source.contains("int relalign = 0;"));
        assert!(source.contains("*dst++ = *src1++ ^ *src2++;"));
        assert!(source.contains("while (IS_ENABLED(CONFIG_64BIT) && len >= 8 && !(relalign & 7))"));
        assert!(source.contains("while (len >= 4 && !(relalign & 3))"));
        assert!(source.contains("while (len >= 2 && !(relalign & 1))"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__crypto_xor);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Crypto library utility functions\")"));

        assert_eq!(CRYPTO_XOR_EXPORT_SYMBOL, "__crypto_xor");
        assert_eq!(
            CRYPTO_UTILS_MODULE_DESCRIPTION,
            "Crypto library utility functions"
        );

        assert_eq!(
            crypto_xor_trace(0x1000, 0x2000, 0x3000, 19, true, true),
            CryptoXorTrace {
                relalign: 0,
                prefix_bytes: 0,
                words64: 2,
                words32: 0,
                words16: 1,
                trailing_bytes: 1,
            }
        );
        assert_eq!(
            crypto_xor_trace(0x1003, 0x2003, 0x3003, 19, false, true),
            CryptoXorTrace {
                relalign: core::mem::size_of::<usize>(),
                prefix_bytes: 5,
                words64: 1,
                words32: 1,
                words16: 1,
                trailing_bytes: 0,
            }
        );
        assert_eq!(
            crypto_xor_trace(0x1000, 0x2001, 0x3001, 9, false, true),
            CryptoXorTrace {
                relalign: 1,
                prefix_bytes: 0,
                words64: 0,
                words32: 0,
                words16: 0,
                trailing_bytes: 9,
            }
        );

        let mut dst = [0u8; 4];
        crypto_xor_cpy(
            &mut dst,
            &[0xff, 0x00, 0xaa, 0x55],
            &[0x0f, 0x0f, 0xff, 0xff],
        );
        assert_eq!(dst, [0xf0, 0x0f, 0x55, 0xaa]);

        let src1: [u8; 23] = core::array::from_fn(|i| (i as u8).wrapping_mul(3));
        let src2: [u8; 23] = core::array::from_fn(|i| 0xf0u8.wrapping_sub(i as u8));
        let mut unaligned = [0u8; 25];
        unsafe {
            __crypto_xor(
                unaligned.as_mut_ptr().add(1),
                src1.as_ptr().add(1),
                src2.as_ptr().add(1),
                22,
            )
        };
        for i in 0..22 {
            assert_eq!(unaligned[i + 1], src1[i + 1] ^ src2[i + 1]);
        }

        let mut alias = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mask = [0xffu8; 8];
        unsafe { __crypto_xor(alias.as_mut_ptr(), alias.as_ptr(), mask.as_ptr(), 8) };
        assert_eq!(alias, [254, 253, 252, 251, 250, 249, 248, 247]);

        let mut raw = [0u8; 3];
        unsafe { __crypto_xor(raw.as_mut_ptr(), b"abc".as_ptr(), [1u8, 2, 3].as_ptr(), 3) };
        assert_eq!(raw, [b'a' ^ 1, b'b' ^ 2, b'c' ^ 3]);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__crypto_xor"),
            Some(__crypto_xor as usize)
        );
    }
}
