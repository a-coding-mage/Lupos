//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/gf128mul.c
//! test-origin: linux:vendor/linux/lib/crypto/gf128mul.c
//! GF(2^128) multiplication helpers.

extern crate alloc;

use alloc::boxed::Box;

use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Be128 {
    pub a: u64,
    pub b: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Le128 {
    pub a: u64,
    pub b: u64,
}

impl Be128 {
    pub const fn new(a: u64, b: u64) -> Self {
        Self { a, b }
    }

    pub fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self {
            a: u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            b: u64::from_be_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
        }
    }

    pub fn to_be_bytes(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.a.to_be_bytes());
        out[8..].copy_from_slice(&self.b.to_be_bytes());
        out
    }
}

impl Le128 {
    pub const fn new(a: u64, b: u64) -> Self {
        Self { a, b }
    }

    pub fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self {
            a: u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            b: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
        }
    }

    pub fn to_le_bytes(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.a.to_le_bytes());
        out[8..].copy_from_slice(&self.b.to_le_bytes());
        out
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("gf128mul_x8_ble", gf128mul_x8_ble_raw as usize, false);
    export_symbol_once("gf128mul_lle", gf128mul_lle_raw as usize, false);
    export_symbol_once(
        "gf128mul_init_64k_bbe",
        gf128mul_init_64k_bbe_raw as usize,
        false,
    );
    export_symbol_once("gf128mul_free_64k", gf128mul_free_64k_raw as usize, false);
    export_symbol_once("gf128mul_64k_bbe", gf128mul_64k_bbe_raw as usize, false);
}

#[inline]
fn mask_from_bit(x: u64, which: u32) -> u64 {
    0u64.wrapping_sub((x >> which) & 1)
}

#[inline]
const fn xda_be(i: u64) -> u64 {
    ((if i & 0x80 != 0 { 0x4380 } else { 0 })
        ^ (if i & 0x40 != 0 { 0x21c0 } else { 0 })
        ^ (if i & 0x20 != 0 { 0x10e0 } else { 0 })
        ^ (if i & 0x10 != 0 { 0x0870 } else { 0 })
        ^ (if i & 0x08 != 0 { 0x0438 } else { 0 })
        ^ (if i & 0x04 != 0 { 0x021c } else { 0 })
        ^ (if i & 0x02 != 0 { 0x010e } else { 0 })
        ^ (if i & 0x01 != 0 { 0x0087 } else { 0 })) as u64
}

#[inline]
const fn xda_le(i: u64) -> u64 {
    ((if i & 0x80 != 0 { 0xe100 } else { 0 })
        ^ (if i & 0x40 != 0 { 0x7080 } else { 0 })
        ^ (if i & 0x20 != 0 { 0x3840 } else { 0 })
        ^ (if i & 0x10 != 0 { 0x1c20 } else { 0 })
        ^ (if i & 0x08 != 0 { 0x0e10 } else { 0 })
        ^ (if i & 0x04 != 0 { 0x0708 } else { 0 })
        ^ (if i & 0x02 != 0 { 0x0384 } else { 0 })
        ^ (if i & 0x01 != 0 { 0x01c2 } else { 0 })) as u64
}

#[inline]
fn be128_xor(r: &mut Be128, a: &Be128, b: &Be128) {
    r.a = a.a ^ b.a;
    r.b = a.b ^ b.b;
}

pub fn gf128mul_x_lle(r: &mut Be128, x: &Be128) {
    let a = x.a;
    let b = x.b;
    let tt = mask_from_bit(b, 0) & (0xe1u64 << 56);
    r.b = (b >> 1) | (a << 63);
    r.a = (a >> 1) ^ tt;
}

pub fn gf128mul_x_bbe(r: &mut Be128, x: &Be128) {
    let a = x.a;
    let b = x.b;
    let tt = mask_from_bit(a, 63) & 0x87;
    r.a = (a << 1) | (b >> 63);
    r.b = (b << 1) ^ tt;
}

pub fn gf128mul_x_ble(r: &mut Le128, x: &Le128) {
    let a = x.a;
    let b = x.b;
    let tt = mask_from_bit(a, 63) & 0x87;
    r.a = (a << 1) | (b >> 63);
    r.b = (b << 1) ^ tt;
}

fn gf128mul_x8_lle_ti(x: &mut Be128) {
    let a = x.a;
    let b = x.b;
    let tt = xda_le(b & 0xff);
    x.b = (b >> 8) | (a << 56);
    x.a = (a >> 8) ^ (tt << 48);
}

fn gf128mul_x8_bbe(x: &mut Be128) {
    let a = x.a;
    let b = x.b;
    let tt = xda_be(a >> 56);
    x.a = (a << 8) | (b >> 56);
    x.b = (b << 8) ^ tt;
}

pub fn gf128mul_x8_ble(r: &mut Le128, x: &Le128) {
    let a = x.a;
    let b = x.b;
    let tt = xda_be(a >> 56);
    r.a = (a << 8) | (b >> 56);
    r.b = (b << 8) ^ tt;
}

pub fn gf128mul_lle(r: &mut Be128, b: &Be128) {
    let mut p = [Be128::default(); 16];
    p[0] = *r;
    for i in 0..7 {
        let prev = p[2 * i];
        gf128mul_x_lle(&mut p[2 * i + 2], &prev);
    }

    let b_bytes = b.to_be_bytes();
    *r = Be128::default();
    for i in 0..16 {
        let ch = b_bytes[15 - i];
        let current = *r;
        be128_xor(r, &current, &p[usize::from((ch & 0x80) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[2 + usize::from((ch & 0x40) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[4 + usize::from((ch & 0x20) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[6 + usize::from((ch & 0x10) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[8 + usize::from((ch & 0x08) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[10 + usize::from((ch & 0x04) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[12 + usize::from((ch & 0x02) == 0)]);
        let current = *r;
        be128_xor(r, &current, &p[14 + usize::from((ch & 0x01) == 0)]);
        if i != 15 {
            gf128mul_x8_lle_ti(r);
        }
    }
}

pub fn ghash_mul(x: &mut [u8; 16], h: &[u8; 16]) {
    let mut z = [0u8; 16];
    let mut v = *h;
    for byte in x.iter().copied() {
        for bit in 0..8 {
            if (byte & (0x80 >> bit)) != 0 {
                for i in 0..16 {
                    z[i] ^= v[i];
                }
            }
            let lsb = v[15] & 1;
            for i in (0..16).rev() {
                let carry = if i > 0 { v[i - 1] & 1 } else { 0 };
                v[i] = (v[i] >> 1) | (carry << 7);
            }
            if lsb != 0 {
                v[0] ^= 0xe1;
            }
        }
    }
    *x = z;
}

pub struct Gf128Mul64k {
    pub t: [[Be128; 256]; 16],
}

impl Gf128Mul64k {
    pub fn new_bbe(g: &Be128) -> Self {
        let mut t = [[Be128::default(); 256]; 16];
        t[0][1] = *g;
        let mut j = 1;
        while j <= 64 {
            let prev = t[0][j];
            gf128mul_x_bbe(&mut t[0][j + j], &prev);
            j <<= 1;
        }
        let mut i = 0;
        loop {
            let mut j = 2;
            while j < 256 {
                for k in 1..j {
                    let left = t[i][j];
                    let right = t[i][k];
                    be128_xor(&mut t[i][j + k], &left, &right);
                }
                j += j;
            }
            i += 1;
            if i >= 16 {
                break;
            }
            let mut j = 128;
            while j > 0 {
                t[i][j] = t[i - 1][j];
                gf128mul_x8_bbe(&mut t[i][j]);
                j >>= 1;
            }
        }
        Self { t }
    }

    pub fn mul_bbe(&self, a: &mut Be128) {
        let ap = a.to_be_bytes();
        let mut r = self.t[0][ap[15] as usize];
        for i in 1..16 {
            let current = r;
            be128_xor(&mut r, &current, &self.t[i][ap[15 - i] as usize]);
        }
        *a = r;
    }
}

pub unsafe extern "C" fn gf128mul_x8_ble_raw(r: *mut Le128, x: *const Le128) {
    if r.is_null() || x.is_null() {
        return;
    }
    unsafe { gf128mul_x8_ble(&mut *r, &*x) };
}

pub unsafe extern "C" fn gf128mul_lle_raw(r: *mut Be128, b: *const Be128) {
    if r.is_null() || b.is_null() {
        return;
    }
    unsafe { gf128mul_lle(&mut *r, &*b) };
}

pub unsafe extern "C" fn gf128mul_init_64k_bbe_raw(g: *const Be128) -> *mut Gf128Mul64k {
    if g.is_null() {
        return core::ptr::null_mut();
    }
    Box::into_raw(Box::new(Gf128Mul64k::new_bbe(unsafe { &*g })))
}

pub unsafe extern "C" fn gf128mul_free_64k_raw(t: *mut Gf128Mul64k) {
    if !t.is_null() {
        drop(unsafe { Box::from_raw(t) });
    }
}

pub unsafe extern "C" fn gf128mul_64k_bbe_raw(a: *mut Be128, t: *const Gf128Mul64k) {
    if a.is_null() || t.is_null() {
        return;
    }
    unsafe { (*t).mul_bbe(&mut *a) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf128mul_source_contract_and_x8_flow_match() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/gf128mul.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/gf128mul.h"
        ));
        assert!(source.contains("static const u16 gf128mul_table_be[256]"));
        assert!(source.contains("void gf128mul_x8_ble(le128 *r, const le128 *x)"));
        assert!(source.contains("void gf128mul_lle(be128 *r, const be128 *b)"));
        assert!(source.contains("gf128mul_x8_lle_ti(r); /* use the time invariant version */"));
        assert!(source.contains("struct gf128mul_64k *gf128mul_init_64k_bbe"));
        assert!(source.contains("void gf128mul_free_64k(struct gf128mul_64k *t)"));
        assert!(source.contains("void gf128mul_64k_bbe(be128 *a, const struct gf128mul_64k *t)"));
        assert!(source.contains("EXPORT_SYMBOL(gf128mul_x8_ble);"));
        assert!(source.contains("EXPORT_SYMBOL(gf128mul_lle);"));
        assert!(source.contains("EXPORT_SYMBOL(gf128mul_init_64k_bbe);"));
        assert!(source.contains("EXPORT_SYMBOL(gf128mul_free_64k);"));
        assert!(source.contains("EXPORT_SYMBOL(gf128mul_64k_bbe);"));
        assert!(header.contains("static inline void gf128mul_x_lle(be128 *r, const be128 *x)"));
        assert!(header.contains("static inline void gf128mul_x_ble(le128 *r, const le128 *x)"));

        let x = Le128::new(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210);
        let mut by_x = x;
        for _ in 0..8 {
            let prev = by_x;
            gf128mul_x_ble(&mut by_x, &prev);
        }
        let mut by_x8 = Le128::default();
        gf128mul_x8_ble(&mut by_x8, &x);
        assert_eq!(by_x8, by_x);

        let mut ghash_input = [
            0x03, 0x88, 0xda, 0xce, 0x60, 0xb6, 0xa3, 0x92, 0xf3, 0x28, 0xc2, 0xb9, 0x71, 0xb2,
            0xfe, 0x78,
        ];
        let h = [
            0x66, 0xe9, 0x4b, 0xd4, 0xef, 0x8a, 0x2c, 0x3b, 0x88, 0x4c, 0xfa, 0x59, 0xca, 0x34,
            0x2b, 0x2e,
        ];
        ghash_mul(&mut ghash_input, &h);
        assert_eq!(
            ghash_input,
            [
                0x5e, 0x2e, 0xc7, 0x46, 0x91, 0x70, 0x62, 0x88, 0x2c, 0x85, 0xb0, 0x68, 0x53, 0x53,
                0xde, 0xb7,
            ]
        );
    }

    #[test]
    fn gf128mul_exports_and_64k_table_api_match_linux_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("gf128mul_x8_ble"),
            Some(gf128mul_x8_ble_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("gf128mul_lle"),
            Some(gf128mul_lle_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("gf128mul_init_64k_bbe"),
            Some(gf128mul_init_64k_bbe_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("gf128mul_free_64k"),
            Some(gf128mul_free_64k_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("gf128mul_64k_bbe"),
            Some(gf128mul_64k_bbe_raw as usize)
        );

        let g = Be128::from_be_bytes([0x87, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let table = unsafe { gf128mul_init_64k_bbe_raw(&g) };
        assert!(!table.is_null());
        let mut via_raw =
            Be128::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let mut via_safe = via_raw;
        unsafe { gf128mul_64k_bbe_raw(&mut via_raw, table) };
        unsafe { (*table).mul_bbe(&mut via_safe) };
        assert_eq!(via_raw, via_safe);
        unsafe { gf128mul_free_64k_raw(table) };
    }
}
