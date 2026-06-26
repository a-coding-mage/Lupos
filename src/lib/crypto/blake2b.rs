//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/blake2b.c
//! test-origin: linux:vendor/linux/lib/crypto/blake2b.c
//! BLAKE2b hash and keyed PRF helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const BLAKE2B_BLOCK_SIZE: usize = 128;
pub const BLAKE2B_HASH_SIZE: usize = 64;
pub const BLAKE2B_KEY_SIZE: usize = 64;
pub const BLAKE2B_160_HASH_SIZE: usize = 20;
pub const BLAKE2B_256_HASH_SIZE: usize = 32;
pub const BLAKE2B_384_HASH_SIZE: usize = 48;
pub const BLAKE2B_512_HASH_SIZE: usize = 64;

const BLAKE2B_IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

const SIGMA: [[usize; 16]; 12] = [
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
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blake2bCtx {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; BLAKE2B_BLOCK_SIZE],
    pub buflen: u32,
    pub outlen: u32,
}

impl Default for Blake2bCtx {
    fn default() -> Self {
        Self {
            h: [0; 8],
            t: [0; 2],
            f: [0; 2],
            buf: [0; BLAKE2B_BLOCK_SIZE],
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
    export_symbol_once("blake2b_update", blake2b_update_raw as usize, false);
    export_symbol_once("blake2b_final", blake2b_final_raw as usize, false);
}

pub fn blake2b_init(ctx: &mut Blake2bCtx, outlen: usize) {
    blake2b_init_param(ctx, outlen, &[], 0);
}

pub fn blake2b_init_key(ctx: &mut Blake2bCtx, outlen: usize, key: &[u8]) {
    assert!(!key.is_empty());
    assert!(key.len() <= BLAKE2B_KEY_SIZE);
    blake2b_init_param(ctx, outlen, key, key.len());
}

fn blake2b_init_param(ctx: &mut Blake2bCtx, outlen: usize, key: &[u8], keylen: usize) {
    assert!(outlen > 0 && outlen <= BLAKE2B_HASH_SIZE);
    assert!(keylen <= BLAKE2B_KEY_SIZE);
    ctx.h = BLAKE2B_IV;
    ctx.h[0] ^= 0x0101_0000 | ((keylen as u64) << 8) | outlen as u64;
    ctx.t = [0; 2];
    ctx.f = [0; 2];
    ctx.buf = [0; BLAKE2B_BLOCK_SIZE];
    ctx.buflen = 0;
    ctx.outlen = outlen as u32;
    if keylen != 0 {
        ctx.buf[..keylen].copy_from_slice(&key[..keylen]);
        ctx.buflen = BLAKE2B_BLOCK_SIZE as u32;
    }
}

#[inline]
fn increment_counter(ctx: &mut Blake2bCtx, inc: u32) {
    ctx.t[0] = ctx.t[0].wrapping_add(inc as u64);
    ctx.t[1] = ctx.t[1].wrapping_add(u64::from(ctx.t[0] < inc as u64));
}

#[inline]
fn g(v: &mut [u64; 16], m: &[u64; 16], r: usize, i: usize, a: usize, b: usize, c: usize, d: usize) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[SIGMA[r][2 * i]]);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[SIGMA[r][2 * i + 1]]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

fn blake2b_compress(ctx: &mut Blake2bCtx, mut data: &[u8], nblocks: usize, inc: u32) {
    for _ in 0..nblocks {
        increment_counter(ctx, inc);
        let block = &data[..BLAKE2B_BLOCK_SIZE];
        let mut m = [0u64; 16];
        for i in 0..16 {
            let offset = i * 8;
            m[i] = u64::from_le_bytes(block[offset..offset + 8].try_into().unwrap());
        }

        let mut v = [0u64; 16];
        v[..8].copy_from_slice(&ctx.h);
        v[8] = BLAKE2B_IV[0];
        v[9] = BLAKE2B_IV[1];
        v[10] = BLAKE2B_IV[2];
        v[11] = BLAKE2B_IV[3];
        v[12] = BLAKE2B_IV[4] ^ ctx.t[0];
        v[13] = BLAKE2B_IV[5] ^ ctx.t[1];
        v[14] = BLAKE2B_IV[6] ^ ctx.f[0];
        v[15] = BLAKE2B_IV[7] ^ ctx.f[1];

        for r in 0..12 {
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
        data = &data[BLAKE2B_BLOCK_SIZE..];
    }
}

pub fn blake2b_update(ctx: &mut Blake2bCtx, mut input: &[u8]) {
    let fill = BLAKE2B_BLOCK_SIZE - ctx.buflen as usize;

    if input.is_empty() {
        return;
    }
    if input.len() > fill {
        ctx.buf[ctx.buflen as usize..ctx.buflen as usize + fill].copy_from_slice(&input[..fill]);
        let block = ctx.buf;
        blake2b_compress(ctx, &block, 1, BLAKE2B_BLOCK_SIZE as u32);
        ctx.buflen = 0;
        input = &input[fill..];
    }
    if input.len() > BLAKE2B_BLOCK_SIZE {
        let nblocks = input.len().div_ceil(BLAKE2B_BLOCK_SIZE);
        blake2b_compress(
            ctx,
            &input[..(nblocks - 1) * BLAKE2B_BLOCK_SIZE],
            nblocks - 1,
            BLAKE2B_BLOCK_SIZE as u32,
        );
        input = &input[(nblocks - 1) * BLAKE2B_BLOCK_SIZE..];
    }
    let buflen = ctx.buflen as usize;
    ctx.buf[buflen..buflen + input.len()].copy_from_slice(input);
    ctx.buflen += input.len() as u32;
}

pub fn blake2b_final(ctx: &mut Blake2bCtx, out: &mut [u8]) {
    assert!(out.len() >= ctx.outlen as usize);
    ctx.f[0] = u64::MAX;
    let buflen = ctx.buflen as usize;
    ctx.buf[buflen..].fill(0);
    let block = ctx.buf;
    blake2b_compress(ctx, &block, 1, ctx.buflen);
    let mut full = [0u8; BLAKE2B_HASH_SIZE];
    for i in 0..8 {
        full[i * 8..i * 8 + 8].copy_from_slice(&ctx.h[i].to_le_bytes());
    }
    out[..ctx.outlen as usize].copy_from_slice(&full[..ctx.outlen as usize]);
    *ctx = Blake2bCtx::default();
}

pub fn blake2b(key: Option<&[u8]>, input: &[u8], out: &mut [u8]) {
    let mut ctx = Blake2bCtx::default();
    match key {
        Some(key) if !key.is_empty() => blake2b_init_key(&mut ctx, out.len(), key),
        _ => blake2b_init(&mut ctx, out.len()),
    }
    blake2b_update(&mut ctx, input);
    blake2b_final(&mut ctx, out);
}

pub unsafe extern "C" fn blake2b_update_raw(ctx: *mut Blake2bCtx, input: *const u8, inlen: usize) {
    if ctx.is_null() || (input.is_null() && inlen != 0) {
        return;
    }
    let input = unsafe { core::slice::from_raw_parts(input, inlen) };
    unsafe { blake2b_update(&mut *ctx, input) };
}

pub unsafe extern "C" fn blake2b_final_raw(ctx: *mut Blake2bCtx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let outlen = unsafe { (*ctx).outlen as usize };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    unsafe { blake2b_final(&mut *ctx, out) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; BLAKE2B_HASH_SIZE])> {
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
                if digest.len() == BLAKE2B_HASH_SIZE {
                    let mut array = [0u8; BLAKE2B_HASH_SIZE];
                    array.copy_from_slice(&digest);
                    out.push((data_len.unwrap(), array));
                    digest.clear();
                }
            }
        }
        out
    }

    #[test]
    fn blake2b_matches_linux_source_and_test_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/blake2b.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/blake2b.h"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/blake2b-testvecs.h"
        ));
        assert!(source.contains("static const u8 blake2b_sigma[12][16]"));
        assert!(source.contains("blake2b_compress_generic(struct blake2b_ctx *ctx"));
        assert!(source.contains("blake2b_increment_counter(ctx, inc);"));
        assert!(source.contains("blake2b_set_lastblock(ctx);"));
        assert!(source.contains("EXPORT_SYMBOL(blake2b_update);"));
        assert!(source.contains("EXPORT_SYMBOL(blake2b_final);"));
        assert!(header.contains("BLAKE2B_BLOCK_SIZE = 128"));
        assert!(kunit.contains("test_blake2b_all_key_and_hash_lens"));
        assert!(vectors.contains("blake2b_keyed_testvec_consolidated"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            let mut actual = [0u8; BLAKE2B_HASH_SIZE];
            blake2b(None, &data, &mut actual);
            assert_eq!(actual, expected, "data_len={len}");
        }

        let data = rand_bytes_seeded_from_len(100);
        let mut main_ctx = Blake2bCtx::default();
        blake2b_init(&mut main_ctx, BLAKE2B_HASH_SIZE);
        for key_len in 0..=BLAKE2B_KEY_SIZE {
            let key = rand_bytes_seeded_from_len(key_len);
            for out_len in 1..=BLAKE2B_HASH_SIZE {
                let mut hash = vec![0u8; out_len];
                blake2b(Some(&key), &data, &mut hash);
                blake2b_update(&mut main_ctx, &hash);
            }
        }
        let mut main_hash = [0u8; BLAKE2B_HASH_SIZE];
        blake2b_final(&mut main_ctx, &mut main_hash);
        assert_eq!(
            main_hash,
            [
                0x2b, 0x89, 0x36, 0x3a, 0x36, 0xe4, 0x18, 0x38, 0xc4, 0x5b, 0x5c, 0xa5, 0x9a, 0xed,
                0xf2, 0xee, 0x5a, 0xb6, 0x82, 0x6c, 0x63, 0xf2, 0x29, 0x57, 0xc7, 0xd5, 0x32, 0x27,
                0xba, 0x88, 0xb1, 0xab, 0xf2, 0x2a, 0xc1, 0xea, 0xf3, 0x91, 0x89, 0x66, 0x47, 0x1e,
                0x5b, 0xc6, 0x98, 0x12, 0xe9, 0x25, 0xbf, 0x72, 0xd2, 0x3f, 0x88, 0x97, 0x17, 0x51,
                0xed, 0x96, 0xfb, 0xe9, 0xca, 0x52, 0x42, 0xc9,
            ]
        );
    }
}
