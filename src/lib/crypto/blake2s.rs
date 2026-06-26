//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/blake2s.c
//! test-origin: linux:vendor/linux/lib/crypto/blake2s.c
//! BLAKE2s hash and keyed PRF helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const BLAKE2S_BLOCK_SIZE: usize = 64;
pub const BLAKE2S_HASH_SIZE: usize = 32;
pub const BLAKE2S_KEY_SIZE: usize = 32;
pub const BLAKE2S_128_HASH_SIZE: usize = 16;
pub const BLAKE2S_160_HASH_SIZE: usize = 20;
pub const BLAKE2S_224_HASH_SIZE: usize = 28;
pub const BLAKE2S_256_HASH_SIZE: usize = 32;

const BLAKE2S_IV: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blake2sCtx {
    pub h: [u32; 8],
    pub t: [u32; 2],
    pub f: [u32; 2],
    pub buf: [u8; BLAKE2S_BLOCK_SIZE],
    pub buflen: u32,
    pub outlen: u32,
}

impl Default for Blake2sCtx {
    fn default() -> Self {
        Self {
            h: [0; 8],
            t: [0; 2],
            f: [0; 2],
            buf: [0; BLAKE2S_BLOCK_SIZE],
            buflen: 0,
            outlen: 0,
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("blake2s_update", blake2s_update_raw as usize, false);
    export_symbol_once("blake2s_final", blake2s_final_raw as usize, false);
}

pub fn blake2s_init(ctx: &mut Blake2sCtx, outlen: usize) {
    blake2s_init_param(ctx, outlen, &[], 0);
}

pub fn blake2s_init_key(ctx: &mut Blake2sCtx, outlen: usize, key: &[u8]) {
    assert!(!key.is_empty());
    assert!(key.len() <= BLAKE2S_KEY_SIZE);
    blake2s_init_param(ctx, outlen, key, key.len());
}

fn blake2s_init_param(ctx: &mut Blake2sCtx, outlen: usize, key: &[u8], keylen: usize) {
    assert!(outlen > 0 && outlen <= BLAKE2S_HASH_SIZE);
    assert!(keylen <= BLAKE2S_KEY_SIZE);
    ctx.h = BLAKE2S_IV;
    ctx.h[0] ^= 0x0101_0000 | ((keylen as u32) << 8) | outlen as u32;
    ctx.t = [0; 2];
    ctx.f = [0; 2];
    ctx.buf = [0; BLAKE2S_BLOCK_SIZE];
    ctx.buflen = 0;
    ctx.outlen = outlen as u32;
    if keylen != 0 {
        ctx.buf[..keylen].copy_from_slice(&key[..keylen]);
        ctx.buflen = BLAKE2S_BLOCK_SIZE as u32;
    }
}

#[inline]
fn increment_counter(ctx: &mut Blake2sCtx, inc: u32) {
    ctx.t[0] = ctx.t[0].wrapping_add(inc);
    ctx.t[1] = ctx.t[1].wrapping_add(u32::from(ctx.t[0] < inc));
}

#[inline]
fn g(v: &mut [u32; 16], m: &[u32; 16], r: usize, i: usize, a: usize, b: usize, c: usize, d: usize) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[SIGMA[r][2 * i]]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[SIGMA[r][2 * i + 1]]);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

fn blake2s_compress(ctx: &mut Blake2sCtx, mut data: &[u8], nblocks: usize, inc: u32) {
    for _ in 0..nblocks {
        increment_counter(ctx, inc);
        let block = &data[..BLAKE2S_BLOCK_SIZE];
        let mut m = [0u32; 16];
        for i in 0..16 {
            let offset = i * 4;
            m[i] = u32::from_le_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        let mut v = [0u32; 16];
        v[..8].copy_from_slice(&ctx.h);
        v[8..].copy_from_slice(&BLAKE2S_IV);
        v[12] ^= ctx.t[0];
        v[13] ^= ctx.t[1];
        v[14] ^= ctx.f[0];
        v[15] ^= ctx.f[1];

        for r in 0..10 {
            g(&mut v, &m, r, 0, 0, 4, 8, 12);
            g(&mut v, &m, r, 1, 1, 5, 9, 13);
            g(&mut v, &m, r, 2, 2, 6, 10, 14);
            g(&mut v, &m, r, 3, 3, 7, 11, 15);
            g(&mut v, &m, r, 4, 0, 5, 10, 15);
            g(&mut v, &m, r, 5, 1, 6, 11, 12);
            g(&mut v, &m, r, 6, 2, 7, 8, 13);
            g(&mut v, &m, r, 7, 3, 4, 9, 14);
        }
        for i in 0..8 {
            ctx.h[i] ^= v[i] ^ v[i + 8];
        }
        data = &data[BLAKE2S_BLOCK_SIZE..];
    }
}

pub fn blake2s_update(ctx: &mut Blake2sCtx, mut input: &[u8]) {
    if input.is_empty() {
        return;
    }
    let fill = BLAKE2S_BLOCK_SIZE - ctx.buflen as usize;
    if input.len() > fill {
        ctx.buf[ctx.buflen as usize..ctx.buflen as usize + fill].copy_from_slice(&input[..fill]);
        let block = ctx.buf;
        blake2s_compress(ctx, &block, 1, BLAKE2S_BLOCK_SIZE as u32);
        ctx.buflen = 0;
        input = &input[fill..];
    }
    if input.len() > BLAKE2S_BLOCK_SIZE {
        let nblocks = input.len().div_ceil(BLAKE2S_BLOCK_SIZE) - 1;
        blake2s_compress(
            ctx,
            &input[..nblocks * BLAKE2S_BLOCK_SIZE],
            nblocks,
            BLAKE2S_BLOCK_SIZE as u32,
        );
        input = &input[nblocks * BLAKE2S_BLOCK_SIZE..];
    }
    let buflen = ctx.buflen as usize;
    ctx.buf[buflen..buflen + input.len()].copy_from_slice(input);
    ctx.buflen += input.len() as u32;
}

pub fn blake2s_final(ctx: &mut Blake2sCtx, out: &mut [u8]) {
    assert!(out.len() >= ctx.outlen as usize);
    ctx.f[0] = u32::MAX;
    let buflen = ctx.buflen as usize;
    ctx.buf[buflen..].fill(0);
    let block = ctx.buf;
    blake2s_compress(ctx, &block, 1, ctx.buflen);
    let mut full = [0u8; BLAKE2S_HASH_SIZE];
    for i in 0..8 {
        full[i * 4..i * 4 + 4].copy_from_slice(&ctx.h[i].to_le_bytes());
    }
    out[..ctx.outlen as usize].copy_from_slice(&full[..ctx.outlen as usize]);
    *ctx = Blake2sCtx::default();
}

pub fn blake2s(key: Option<&[u8]>, input: &[u8], out: &mut [u8]) {
    let mut ctx = Blake2sCtx::default();
    match key {
        Some(key) if !key.is_empty() => blake2s_init_key(&mut ctx, out.len(), key),
        _ => blake2s_init(&mut ctx, out.len()),
    }
    blake2s_update(&mut ctx, input);
    blake2s_final(&mut ctx, out);
}

pub unsafe extern "C" fn blake2s_update_raw(ctx: *mut Blake2sCtx, input: *const u8, inlen: usize) {
    if ctx.is_null() || (input.is_null() && inlen != 0) {
        return;
    }
    let input = unsafe { core::slice::from_raw_parts(input, inlen) };
    unsafe { blake2s_update(&mut *ctx, input) };
}

pub unsafe extern "C" fn blake2s_final_raw(ctx: *mut Blake2sCtx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let outlen = unsafe { (*ctx).outlen as usize };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    unsafe { blake2s_final(&mut *ctx, out) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; BLAKE2S_HASH_SIZE])> {
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
                if digest.len() == BLAKE2S_HASH_SIZE {
                    let mut array = [0u8; BLAKE2S_HASH_SIZE];
                    array.copy_from_slice(&digest);
                    out.push((data_len.unwrap(), array));
                    digest.clear();
                }
            }
        }
        out
    }

    #[test]
    fn blake2s_matches_linux_kunit_vectors_and_keyed_lengths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/blake2s.c"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2s_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2s-testvecs.h"
        ));
        assert!(source.contains("blake2s_compress_generic(struct blake2s_ctx *ctx"));
        assert!(source.contains("EXPORT_SYMBOL(blake2s_update);"));
        assert!(source.contains("EXPORT_SYMBOL(blake2s_final);"));
        assert!(kunit.contains("test_blake2s_all_key_and_hash_lens"));
        assert!(kunit.contains("for (int key_len = 0; key_len <= BLAKE2S_KEY_SIZE; key_len++)"));
        assert!(vectors.contains("blake2s_keyed_testvec_consolidated"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            let mut actual = [0u8; BLAKE2S_HASH_SIZE];
            blake2s(None, &data, &mut actual);
            assert_eq!(actual, expected, "data_len={len}");
        }

        let data = rand_bytes_seeded_from_len(100);
        let mut main_ctx = Blake2sCtx::default();
        blake2s_init(&mut main_ctx, BLAKE2S_HASH_SIZE);
        for key_len in 0..=BLAKE2S_KEY_SIZE {
            let key = rand_bytes_seeded_from_len(key_len);
            for out_len in 1..=BLAKE2S_HASH_SIZE {
                let mut hash = vec![0u8; out_len];
                blake2s(Some(&key), &data, &mut hash);
                blake2s_update(&mut main_ctx, &hash);
            }
        }
        let mut main_hash = [0u8; BLAKE2S_HASH_SIZE];
        blake2s_final(&mut main_ctx, &mut main_hash);
        assert_eq!(
            main_hash,
            [
                0xa6, 0xad, 0xcd, 0xb8, 0xd9, 0xdd, 0xc7, 0x70, 0x07, 0x09, 0x7f, 0x9f, 0x41, 0xa9,
                0x70, 0xa4, 0x1c, 0xca, 0x61, 0xbb, 0x58, 0xb5, 0xb2, 0x1d, 0xd1, 0x71, 0x16, 0xb0,
                0x49, 0x4f, 0x9e, 0x1b,
            ]
        );
    }
}
