//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/curve25519.c
//! test-origin: linux:vendor/linux/lib/crypto/curve25519.c
//! Curve25519/X25519 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CURVE25519_KEY_SIZE: usize = 32;
pub const CURVE25519_BASE_POINT: [u8; CURVE25519_KEY_SIZE] = {
    let mut point = [0u8; CURVE25519_KEY_SIZE];
    point[0] = 9;
    point
};
pub const CURVE25519_NULL_POINT: [u8; CURVE25519_KEY_SIZE] = [0; CURVE25519_KEY_SIZE];

const MASK51: u64 = (1u64 << 51) - 1;
const BASE51: u128 = 1u128 << 51;
const FIELD_P: [u64; 5] = [MASK51 - 18, MASK51, MASK51, MASK51, MASK51];
const FIELD_2P: [u128; 5] = [
    BASE51 * 2 - 38,
    BASE51 * 2 - 2,
    BASE51 * 2 - 2,
    BASE51 * 2 - 2,
    BASE51 * 2 - 2,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Field([u64; 5]);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("curve25519", curve25519_raw as usize, false);
    export_symbol_once(
        "curve25519_generate_public",
        curve25519_generate_public_raw as usize,
        false,
    );
}

pub fn curve25519_clamp_secret(secret: &mut [u8; CURVE25519_KEY_SIZE]) {
    secret[0] &= 248;
    secret[31] = (secret[31] & 127) | 64;
}

fn load64(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
        input[offset + 4],
        input[offset + 5],
        input[offset + 6],
        input[offset + 7],
    ])
}

fn field_from_bytes(input: &[u8; CURVE25519_KEY_SIZE]) -> Field {
    let mut bytes = *input;
    bytes[31] &= 0x7f;
    Field([
        load64(&bytes, 0) & MASK51,
        (load64(&bytes, 6) >> 3) & MASK51,
        (load64(&bytes, 12) >> 6) & MASK51,
        (load64(&bytes, 19) >> 1) & MASK51,
        (load64(&bytes, 24) >> 12) & MASK51,
    ])
}

fn carry_reduce(mut h: [u128; 5]) -> Field {
    let mut carry = h[0] >> 51;
    h[0] &= MASK51 as u128;
    h[1] += carry;
    carry = h[1] >> 51;
    h[1] &= MASK51 as u128;
    h[2] += carry;
    carry = h[2] >> 51;
    h[2] &= MASK51 as u128;
    h[3] += carry;
    carry = h[3] >> 51;
    h[3] &= MASK51 as u128;
    h[4] += carry;
    carry = h[4] >> 51;
    h[4] &= MASK51 as u128;
    h[0] += carry * 19;

    carry = h[0] >> 51;
    h[0] &= MASK51 as u128;
    h[1] += carry;
    carry = h[1] >> 51;
    h[1] &= MASK51 as u128;
    h[2] += carry;
    carry = h[2] >> 51;
    h[2] &= MASK51 as u128;
    h[3] += carry;
    carry = h[3] >> 51;
    h[3] &= MASK51 as u128;
    h[4] += carry;
    carry = h[4] >> 51;
    h[4] &= MASK51 as u128;
    h[0] += carry * 19;

    Field([
        h[0] as u64,
        h[1] as u64,
        h[2] as u64,
        h[3] as u64,
        h[4] as u64,
    ])
}

fn field_add(a: Field, b: Field) -> Field {
    carry_reduce([
        a.0[0] as u128 + b.0[0] as u128,
        a.0[1] as u128 + b.0[1] as u128,
        a.0[2] as u128 + b.0[2] as u128,
        a.0[3] as u128 + b.0[3] as u128,
        a.0[4] as u128 + b.0[4] as u128,
    ])
}

fn field_sub(a: Field, b: Field) -> Field {
    carry_reduce([
        a.0[0] as u128 + FIELD_2P[0] - b.0[0] as u128,
        a.0[1] as u128 + FIELD_2P[1] - b.0[1] as u128,
        a.0[2] as u128 + FIELD_2P[2] - b.0[2] as u128,
        a.0[3] as u128 + FIELD_2P[3] - b.0[3] as u128,
        a.0[4] as u128 + FIELD_2P[4] - b.0[4] as u128,
    ])
}

fn field_mul(a: Field, b: Field) -> Field {
    let f = a.0;
    let g = b.0;
    carry_reduce([
        f[0] as u128 * g[0] as u128
            + 19 * (f[1] as u128 * g[4] as u128
                + f[2] as u128 * g[3] as u128
                + f[3] as u128 * g[2] as u128
                + f[4] as u128 * g[1] as u128),
        f[0] as u128 * g[1] as u128
            + f[1] as u128 * g[0] as u128
            + 19 * (f[2] as u128 * g[4] as u128
                + f[3] as u128 * g[3] as u128
                + f[4] as u128 * g[2] as u128),
        f[0] as u128 * g[2] as u128
            + f[1] as u128 * g[1] as u128
            + f[2] as u128 * g[0] as u128
            + 19 * (f[3] as u128 * g[4] as u128 + f[4] as u128 * g[3] as u128),
        f[0] as u128 * g[3] as u128
            + f[1] as u128 * g[2] as u128
            + f[2] as u128 * g[1] as u128
            + f[3] as u128 * g[0] as u128
            + 19 * (f[4] as u128 * g[4] as u128),
        f[0] as u128 * g[4] as u128
            + f[1] as u128 * g[3] as u128
            + f[2] as u128 * g[2] as u128
            + f[3] as u128 * g[1] as u128
            + f[4] as u128 * g[0] as u128,
    ])
}

fn field_square(a: Field) -> Field {
    field_mul(a, a)
}

fn field_mul_small(a: Field, scalar: u64) -> Field {
    carry_reduce([
        a.0[0] as u128 * scalar as u128,
        a.0[1] as u128 * scalar as u128,
        a.0[2] as u128 * scalar as u128,
        a.0[3] as u128 * scalar as u128,
        a.0[4] as u128 * scalar as u128,
    ])
}

fn exp_p_minus_2_bit(bit: i32) -> bool {
    if bit < 8 {
        ((0xebu8 >> bit) & 1) != 0
    } else {
        true
    }
}

fn field_invert(z: Field) -> Field {
    let mut result = Field([1, 0, 0, 0, 0]);
    let mut bit = 254i32;
    while bit >= 0 {
        result = field_square(result);
        if exp_p_minus_2_bit(bit) {
            result = field_mul(result, z);
        }
        bit -= 1;
    }
    result
}

fn ge_p(h: &[u64; 5]) -> bool {
    let mut i = 5usize;
    while i != 0 {
        i -= 1;
        if h[i] != FIELD_P[i] {
            return h[i] > FIELD_P[i];
        }
    }
    true
}

fn subtract_p(mut h: [u64; 5]) -> [u64; 5] {
    let mut borrow = 0i128;
    for i in 0..5 {
        let diff = h[i] as i128 - FIELD_P[i] as i128 - borrow;
        if diff < 0 {
            h[i] = (diff + BASE51 as i128) as u64;
            borrow = 1;
        } else {
            h[i] = diff as u64;
            borrow = 0;
        }
    }
    h
}

fn field_to_bytes(field: Field) -> [u8; CURVE25519_KEY_SIZE] {
    let mut h = carry_reduce([
        field.0[0] as u128,
        field.0[1] as u128,
        field.0[2] as u128,
        field.0[3] as u128,
        field.0[4] as u128,
    ])
    .0;
    if ge_p(&h) {
        h = subtract_p(h);
    }

    let mut out = [0u8; CURVE25519_KEY_SIZE];
    let mut accumulator = 0u128;
    let mut bits = 0usize;
    let mut out_idx = 0usize;
    for limb in h {
        accumulator |= (limb as u128) << bits;
        bits += 51;
        while bits >= 8 && out_idx < CURVE25519_KEY_SIZE {
            out[out_idx] = accumulator as u8;
            accumulator >>= 8;
            bits -= 8;
            out_idx += 1;
        }
    }
    while out_idx < CURVE25519_KEY_SIZE {
        out[out_idx] = accumulator as u8;
        accumulator >>= 8;
        out_idx += 1;
    }
    out[31] &= 0x7f;
    out
}

fn cswap(swap: u8, a: &mut Field, b: &mut Field) {
    if swap != 0 {
        core::mem::swap(a, b);
    }
}

pub fn curve25519_generic(
    secret: &[u8; CURVE25519_KEY_SIZE],
    basepoint: &[u8; CURVE25519_KEY_SIZE],
) -> [u8; CURVE25519_KEY_SIZE] {
    let mut scalar = *secret;
    curve25519_clamp_secret(&mut scalar);
    let x1 = field_from_bytes(basepoint);
    let mut x2 = Field([1, 0, 0, 0, 0]);
    let mut z2 = Field([0, 0, 0, 0, 0]);
    let mut x3 = x1;
    let mut z3 = Field([1, 0, 0, 0, 0]);
    let mut swap = 0u8;

    let mut t = 254i32;
    while t >= 0 {
        let bit = ((scalar[(t as usize) >> 3] >> ((t as usize) & 7)) & 1) as u8;
        swap ^= bit;
        cswap(swap, &mut x2, &mut x3);
        cswap(swap, &mut z2, &mut z3);
        swap = bit;

        let a = field_add(x2, z2);
        let aa = field_square(a);
        let b = field_sub(x2, z2);
        let bb = field_square(b);
        let e = field_sub(aa, bb);
        let c = field_add(x3, z3);
        let d = field_sub(x3, z3);
        let da = field_mul(d, a);
        let cb = field_mul(c, b);
        let da_plus_cb = field_add(da, cb);
        let da_minus_cb = field_sub(da, cb);
        x3 = field_square(da_plus_cb);
        z3 = field_mul(x1, field_square(da_minus_cb));
        x2 = field_mul(aa, bb);
        z2 = field_mul(e, field_add(aa, field_mul_small(e, 121665)));

        t -= 1;
    }

    cswap(swap, &mut x2, &mut x3);
    cswap(swap, &mut z2, &mut z3);

    field_to_bytes(field_mul(x2, field_invert(z2)))
}

pub fn curve25519(
    mypublic: &mut [u8; CURVE25519_KEY_SIZE],
    secret: &[u8; CURVE25519_KEY_SIZE],
    basepoint: &[u8; CURVE25519_KEY_SIZE],
) -> bool {
    *mypublic = curve25519_generic(secret, basepoint);
    *mypublic != CURVE25519_NULL_POINT
}

pub fn curve25519_generate_public(
    public: &mut [u8; CURVE25519_KEY_SIZE],
    secret: &[u8; CURVE25519_KEY_SIZE],
) -> bool {
    if *secret == CURVE25519_NULL_POINT {
        return false;
    }
    curve25519(public, secret, &CURVE25519_BASE_POINT)
}

pub unsafe extern "C" fn curve25519_raw(
    mypublic: *mut u8,
    secret: *const u8,
    basepoint: *const u8,
) -> bool {
    if mypublic.is_null() || secret.is_null() || basepoint.is_null() {
        return false;
    }
    let secret = unsafe { &*(secret.cast::<[u8; CURVE25519_KEY_SIZE]>()) };
    let basepoint = unsafe { &*(basepoint.cast::<[u8; CURVE25519_KEY_SIZE]>()) };
    let mut out = [0u8; CURVE25519_KEY_SIZE];
    let ok = curve25519(&mut out, secret, basepoint);
    unsafe { core::ptr::copy_nonoverlapping(out.as_ptr(), mypublic, CURVE25519_KEY_SIZE) };
    ok
}

pub unsafe extern "C" fn curve25519_generate_public_raw(
    public: *mut u8,
    secret: *const u8,
) -> bool {
    if public.is_null() || secret.is_null() {
        return false;
    }
    let secret = unsafe { &*(secret.cast::<[u8; CURVE25519_KEY_SIZE]>()) };
    let mut out = [0u8; CURVE25519_KEY_SIZE];
    let ok = curve25519_generate_public(&mut out, secret);
    unsafe { core::ptr::copy_nonoverlapping(out.as_ptr(), public, CURVE25519_KEY_SIZE) };
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_hex_32(hex: &str) -> [u8; CURVE25519_KEY_SIZE] {
        fn value(byte: u8) -> u8 {
            match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("invalid hex"),
            }
        }
        let bytes = hex.as_bytes();
        let mut out = [0u8; CURVE25519_KEY_SIZE];
        let mut i = 0usize;
        while i < out.len() {
            out[i] = (value(bytes[i * 2]) << 4) | value(bytes[i * 2 + 1]);
            i += 1;
        }
        out
    }

    #[test]
    fn curve25519_matches_linux_wrapper_and_rfc7748_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/curve25519.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/curve25519.h"
        ));
        assert!(source.contains("curve25519_null_point"));
        assert!(source.contains("curve25519_base_point"));
        assert!(source.contains("curve25519_generic(mypublic, secret, basepoint);"));
        assert!(source.contains("crypto_memneq(mypublic, curve25519_null_point"));
        assert!(source.contains("EXPORT_SYMBOL(curve25519_generate_public);"));
        assert!(header.contains("secret[0] &= 248;"));
        assert!(header.contains("secret[31] = (secret[31] & 127) | 64;"));

        let scalar =
            decode_hex_32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let point =
            decode_hex_32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
        let expected =
            decode_hex_32("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");
        let mut out = [0u8; CURVE25519_KEY_SIZE];
        assert!(curve25519(&mut out, &scalar, &point));
        assert_eq!(out, expected);

        let base_scalar =
            decode_hex_32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let base_expected =
            decode_hex_32("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
        assert!(curve25519_generate_public(&mut out, &base_scalar));
        assert_eq!(out, base_expected);
        assert!(!curve25519_generate_public(
            &mut out,
            &CURVE25519_NULL_POINT
        ));
    }
}
