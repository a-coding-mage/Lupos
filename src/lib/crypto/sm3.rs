//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/sm3.c
//! test-origin: linux:vendor/linux/lib/crypto/sm3.c
//! SM3 hash helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub const SM3_DIGEST_SIZE: usize = 32;
pub const SM3_BLOCK_SIZE: usize = 64;
pub const SM3_STATE_WORDS: usize = SM3_DIGEST_SIZE / 4;

pub const SM3_IVA: u32 = 0x7380_166f;
pub const SM3_IVB: u32 = 0x4914_b2b9;
pub const SM3_IVC: u32 = 0x1724_42d7;
pub const SM3_IVD: u32 = 0xda8a_0600;
pub const SM3_IVE: u32 = 0xa96f_30bc;
pub const SM3_IVF: u32 = 0x1631_38aa;
pub const SM3_IVG: u32 = 0xe38d_ee4d;
pub const SM3_IVH: u32 = 0xb0fb_0e4e;

const K: [u32; 64] = [
    0x79cc_4519,
    0xf398_8a32,
    0xe731_1465,
    0xce62_28cb,
    0x9cc4_5197,
    0x3988_a32f,
    0x7311_465e,
    0xe622_8cbc,
    0xcc45_1979,
    0x988a_32f3,
    0x3114_65e7,
    0x6228_cbce,
    0xc451_979c,
    0x88a3_2f39,
    0x1146_5e73,
    0x228c_bce6,
    0x9d8a_7a87,
    0x3b14_f50f,
    0x7629_ea1e,
    0xec53_d43c,
    0xd8a7_a879,
    0xb14f_50f3,
    0x629e_a1e7,
    0xc53d_43ce,
    0x8a7a_879d,
    0x14f5_0f3b,
    0x29ea_1e76,
    0x53d4_3cec,
    0xa7a8_79d8,
    0x4f50_f3b1,
    0x9ea1_e762,
    0x3d43_cec5,
    0x7a87_9d8a,
    0xf50f_3b14,
    0xea1e_7629,
    0xd43c_ec53,
    0xa879_d8a7,
    0x50f3_b14f,
    0xa1e7_629e,
    0x43ce_c53d,
    0x879d_8a7a,
    0x0f3b_14f5,
    0x1e76_29ea,
    0x3cec_53d4,
    0x79d8_a7a8,
    0xf3b1_4f50,
    0xe762_9ea1,
    0xcec5_3d43,
    0x9d8a_7a87,
    0x3b14_f50f,
    0x7629_ea1e,
    0xec53_d43c,
    0xd8a7_a879,
    0xb14f_50f3,
    0x629e_a1e7,
    0xc53d_43ce,
    0x8a7a_879d,
    0x14f5_0f3b,
    0x29ea_1e76,
    0x53d4_3cec,
    0xa7a8_79d8,
    0x4f50_f3b1,
    0x9ea1_e762,
    0x3d43_cec5,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sm3BlockState {
    pub h: [u32; SM3_STATE_WORDS],
}

const SM3_IV: Sm3BlockState = Sm3BlockState {
    h: [
        SM3_IVA, SM3_IVB, SM3_IVC, SM3_IVD, SM3_IVE, SM3_IVF, SM3_IVG, SM3_IVH,
    ],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sm3Ctx {
    pub state: Sm3BlockState,
    pub bytecount: u64,
    pub buf: [u8; SM3_BLOCK_SIZE],
}

impl Default for Sm3Ctx {
    fn default() -> Self {
        Self {
            state: SM3_IV,
            bytecount: 0,
            buf: [0; SM3_BLOCK_SIZE],
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sm3_init", sm3_init_raw as usize, true);
    export_symbol_once("sm3_update", sm3_update_raw as usize, true);
    export_symbol_once("sm3_final", sm3_final_raw as usize, true);
    export_symbol_once("sm3", sm3_raw as usize, true);
}

#[inline]
fn ff1(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

#[inline]
fn ff2(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (x & z) | (y & z)
}

#[inline]
fn gg1(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

#[inline]
fn gg2(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}

#[inline]
fn p0(x: u32) -> u32 {
    x ^ x.rotate_left(9) ^ x.rotate_left(17)
}

#[inline]
fn p1(x: u32) -> u32 {
    x ^ x.rotate_left(15) ^ x.rotate_left(23)
}

fn sm3_block_generic(state: &mut Sm3BlockState, data: &[u8; SM3_BLOCK_SIZE]) {
    let mut w = [0u32; 68];
    for i in 0..16 {
        let offset = i * 4;
        w[i] = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
    }
    for i in 16..68 {
        w[i] = p1(w[i - 16] ^ w[i - 9] ^ w[i - 3].rotate_left(15))
            ^ w[i - 13].rotate_left(7)
            ^ w[i - 6];
    }

    let mut a = state.h[0];
    let mut b = state.h[1];
    let mut c = state.h[2];
    let mut d = state.h[3];
    let mut e = state.h[4];
    let mut f = state.h[5];
    let mut g = state.h[6];
    let mut h = state.h[7];

    for i in 0..64 {
        let ss1 = a
            .rotate_left(12)
            .wrapping_add(e)
            .wrapping_add(K[i])
            .rotate_left(7);
        let ss2 = ss1 ^ a.rotate_left(12);
        let w1 = w[i] ^ w[i + 4];
        let tt1 = if i < 16 { ff1(a, b, c) } else { ff2(a, b, c) }
            .wrapping_add(d)
            .wrapping_add(ss2)
            .wrapping_add(w1);
        let tt2 = if i < 16 { gg1(e, f, g) } else { gg2(e, f, g) }
            .wrapping_add(h)
            .wrapping_add(ss1)
            .wrapping_add(w[i]);
        d = c;
        c = b.rotate_left(9);
        b = a;
        a = tt1;
        h = g;
        g = f.rotate_left(19);
        f = e;
        e = p0(tt2);
    }

    state.h[0] ^= a;
    state.h[1] ^= b;
    state.h[2] ^= c;
    state.h[3] ^= d;
    state.h[4] ^= e;
    state.h[5] ^= f;
    state.h[6] ^= g;
    state.h[7] ^= h;
}

fn sm3_blocks(state: &mut Sm3BlockState, data: &[u8]) {
    for block in data.chunks_exact(SM3_BLOCK_SIZE) {
        let block = <&[u8; SM3_BLOCK_SIZE]>::try_from(block).unwrap();
        sm3_block_generic(state, block);
    }
}

pub fn sm3_init(ctx: &mut Sm3Ctx) {
    *ctx = Sm3Ctx::default();
}

pub fn sm3_update(ctx: &mut Sm3Ctx, mut data: &[u8]) {
    let mut partial = (ctx.bytecount as usize) % SM3_BLOCK_SIZE;
    ctx.bytecount = ctx.bytecount.wrapping_add(data.len() as u64);

    if partial + data.len() >= SM3_BLOCK_SIZE {
        if partial != 0 {
            let take = SM3_BLOCK_SIZE - partial;
            ctx.buf[partial..partial + take].copy_from_slice(&data[..take]);
            sm3_blocks(&mut ctx.state, &ctx.buf);
            data = &data[take..];
        }

        let nblocks_len = data.len() / SM3_BLOCK_SIZE * SM3_BLOCK_SIZE;
        if nblocks_len != 0 {
            sm3_blocks(&mut ctx.state, &data[..nblocks_len]);
            data = &data[nblocks_len..];
        }
        partial = 0;
    }
    if !data.is_empty() {
        ctx.buf[partial..partial + data.len()].copy_from_slice(data);
    }
}

fn __sm3_final(ctx: &mut Sm3Ctx, out: &mut [u8; SM3_DIGEST_SIZE]) {
    let bitcount = ctx.bytecount << 3;
    let mut partial = (ctx.bytecount as usize) % SM3_BLOCK_SIZE;

    ctx.buf[partial] = 0x80;
    partial += 1;
    if partial > SM3_BLOCK_SIZE - 8 {
        ctx.buf[partial..].fill(0);
        sm3_blocks(&mut ctx.state, &ctx.buf);
        partial = 0;
    }
    ctx.buf[partial..SM3_BLOCK_SIZE - 8].fill(0);
    ctx.buf[SM3_BLOCK_SIZE - 8..].copy_from_slice(&bitcount.to_be_bytes());
    sm3_blocks(&mut ctx.state, &ctx.buf);

    for i in 0..SM3_STATE_WORDS {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx.state.h[i].to_be_bytes());
    }
}

pub fn sm3_final(ctx: &mut Sm3Ctx, out: &mut [u8; SM3_DIGEST_SIZE]) {
    __sm3_final(ctx, out);
    *ctx = Sm3Ctx {
        state: Sm3BlockState {
            h: [0; SM3_STATE_WORDS],
        },
        bytecount: 0,
        buf: [0; SM3_BLOCK_SIZE],
    };
}

pub fn sm3(data: &[u8]) -> [u8; SM3_DIGEST_SIZE] {
    let mut ctx = Sm3Ctx::default();
    sm3_update(&mut ctx, data);
    let mut out = [0u8; SM3_DIGEST_SIZE];
    sm3_final(&mut ctx, &mut out);
    out
}

pub unsafe extern "C" fn sm3_init_raw(ctx: *mut Sm3Ctx) {
    if !ctx.is_null() {
        unsafe { sm3_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn sm3_update_raw(ctx: *mut Sm3Ctx, data: *const u8, len: usize) {
    if ctx.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    unsafe { sm3_update(&mut *ctx, data) };
}

pub unsafe extern "C" fn sm3_final_raw(ctx: *mut Sm3Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SM3_DIGEST_SIZE]) };
    unsafe { sm3_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sm3_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sm3(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SM3_DIGEST_SIZE) };
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    fn rand_bytes_seeded_from_len(len: usize) -> Vec<u8> {
        let mut seed = len as u64;
        let mut out = vec![0u8; len];
        for byte in &mut out {
            seed = (seed.wrapping_mul(25_214_903_917).wrapping_add(11)) & ((1u64 << 48) - 1);
            *byte = (seed >> 16) as u8;
        }
        out
    }

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; SM3_DIGEST_SIZE])> {
        let mut out = Vec::new();
        let mut data_len = None;
        let mut digest = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("static const u8 ") {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix(".data_len = ") {
                data_len = Some(rest.trim_end_matches(',').parse::<usize>().unwrap());
                digest.clear();
            } else if trimmed.starts_with("0x") {
                for token in trimmed.split(',') {
                    let token = token.trim();
                    if let Some(hex) = token.strip_prefix("0x") {
                        digest.push(u8::from_str_radix(hex, 16).unwrap());
                    }
                }
                if digest.len() == SM3_DIGEST_SIZE {
                    let mut array = [0u8; SM3_DIGEST_SIZE];
                    array.copy_from_slice(&digest);
                    out.push((data_len.unwrap(), array));
                    digest.clear();
                }
            }
        }
        out
    }

    fn parse_named_array<const N: usize>(text: &str, name: &str) -> [u8; N] {
        let marker = alloc::format!("static const u8 {name}");
        let mut bytes = Vec::new();
        let mut in_array = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(&marker) {
                in_array = true;
                continue;
            }
            if in_array && trimmed.starts_with("};") {
                break;
            }
            if in_array && trimmed.starts_with("0x") {
                for token in trimmed.split(',') {
                    let token = token.trim();
                    if let Some(hex) = token.strip_prefix("0x") {
                        bytes.push(u8::from_str_radix(hex, 16).unwrap());
                    }
                }
            }
        }
        assert_eq!(bytes.len(), N);
        let mut array = [0u8; N];
        array.copy_from_slice(&bytes);
        array
    }

    #[test]
    fn sm3_matches_linux_kunit_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/sm3.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sm3-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));
        assert!(source.contains("static const u32 ____cacheline_aligned K[64]"));
        assert!(source.contains("state->h[0] ^= a;"));
        assert!(source.contains("void sm3_update(struct sm3_ctx *ctx"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sm3_final);"));
        assert!(vectors.contains("hash_testvec_consolidated[SM3_DIGEST_SIZE]"));
        assert!(template.contains("KUNIT_CASE(test_hash_all_lens_up_to_4096)"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            assert_eq!(sm3(&data), expected, "data_len={len}");
        }

        let test_buf = rand_bytes_seeded_from_len(4096);
        let mut consolidated_ctx = Sm3Ctx::default();
        for len in 0..=4096 {
            let digest = sm3(&test_buf[..len]);
            sm3_update(&mut consolidated_ctx, &digest);
        }
        let mut consolidated = [0u8; SM3_DIGEST_SIZE];
        sm3_final(&mut consolidated_ctx, &mut consolidated);
        assert_eq!(
            consolidated,
            parse_named_array(vectors, "hash_testvec_consolidated")
        );

        let mut ctx = Sm3Ctx::default();
        sm3_update(&mut ctx, b"The quick ");
        sm3_update(&mut ctx, b"brown fox");
        let mut digest = [0u8; SM3_DIGEST_SIZE];
        sm3_final(&mut ctx, &mut digest);
        assert_eq!(digest, sm3(b"The quick brown fox"));
        assert_eq!(ctx.state.h, [0; SM3_STATE_WORDS]);
        assert_eq!(ctx.bytecount, 0);
        assert_eq!(ctx.buf, [0; SM3_BLOCK_SIZE]);
    }
}
