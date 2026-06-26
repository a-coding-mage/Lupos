//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/sha1.c
//! test-origin: linux:vendor/linux/lib/crypto/sha1.c
//! SHA-1 and HMAC-SHA1 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const SHA1_DIGEST_SIZE: usize = 20;
pub const SHA1_BLOCK_SIZE: usize = 64;
pub const SHA1_HASH_WORDS: usize = SHA1_DIGEST_SIZE / 4;
pub const SHA1_H0: u32 = 0x6745_2301;
pub const SHA1_H1: u32 = 0xefcd_ab89;
pub const SHA1_H2: u32 = 0x98ba_dcfe;
pub const SHA1_H3: u32 = 0x1032_5476;
pub const SHA1_H4: u32 = 0xc3d2_e1f0;
const HMAC_IPAD_VALUE: u8 = 0x36;
const HMAC_OPAD_VALUE: u8 = 0x5c;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha1BlockState {
    pub h: [u32; SHA1_HASH_WORDS],
}

impl Default for Sha1BlockState {
    fn default() -> Self {
        Self {
            h: [SHA1_H0, SHA1_H1, SHA1_H2, SHA1_H3, SHA1_H4],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha1Ctx {
    pub state: Sha1BlockState,
    pub bytecount: u64,
    pub buf: [u8; SHA1_BLOCK_SIZE],
}

impl Default for Sha1Ctx {
    fn default() -> Self {
        Self {
            state: Sha1BlockState::default(),
            bytecount: 0,
            buf: [0; SHA1_BLOCK_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct HmacSha1Key {
    pub istate: Sha1BlockState,
    pub ostate: Sha1BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct HmacSha1Ctx {
    pub sha_ctx: Sha1Ctx,
    pub ostate: Sha1BlockState,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sha1_init", sha1_init_raw as usize, true);
    export_symbol_once("sha1_update", sha1_update_raw as usize, true);
    export_symbol_once("sha1_final", sha1_final_raw as usize, true);
    export_symbol_once("sha1", sha1_raw as usize, true);
    export_symbol_once(
        "hmac_sha1_preparekey",
        hmac_sha1_preparekey_raw as usize,
        true,
    );
    export_symbol_once("hmac_sha1_init", hmac_sha1_init_raw as usize, true);
    export_symbol_once(
        "hmac_sha1_init_usingrawkey",
        hmac_sha1_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once("hmac_sha1_final", hmac_sha1_final_raw as usize, true);
    export_symbol_once("hmac_sha1", hmac_sha1_raw as usize, true);
    export_symbol_once(
        "hmac_sha1_usingrawkey",
        hmac_sha1_usingrawkey_raw as usize,
        true,
    );
}

pub fn sha1_init(ctx: &mut Sha1Ctx) {
    *ctx = Sha1Ctx::default();
}

fn sha1_block_generic(state: &mut Sha1BlockState, data: &[u8; SHA1_BLOCK_SIZE]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        let offset = i * 4;
        w[i] = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let mut a = state.h[0];
    let mut b = state.h[1];
    let mut c = state.h[2];
    let mut d = state.h[3];
    let mut e = state.h[4];

    for (i, word) in w.iter().copied().enumerate() {
        let (f, k) = if i < 20 {
            (((c ^ d) & b) ^ d, 0x5a82_7999)
        } else if i < 40 {
            (b ^ c ^ d, 0x6ed9_eba1)
        } else if i < 60 {
            ((b & c) | (d & (b ^ c)), 0x8f1b_bcdc)
        } else {
            (b ^ c ^ d, 0xca62_c1d6)
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(word);
        e = d;
        d = c;
        c = b.rotate_right(2);
        b = a;
        a = temp;
    }

    state.h[0] = state.h[0].wrapping_add(a);
    state.h[1] = state.h[1].wrapping_add(b);
    state.h[2] = state.h[2].wrapping_add(c);
    state.h[3] = state.h[3].wrapping_add(d);
    state.h[4] = state.h[4].wrapping_add(e);
}

fn sha1_blocks(state: &mut Sha1BlockState, data: &[u8]) {
    for block in data.chunks_exact(SHA1_BLOCK_SIZE) {
        let block = <&[u8; SHA1_BLOCK_SIZE]>::try_from(block).unwrap();
        sha1_block_generic(state, block);
    }
}

pub fn sha1_update(ctx: &mut Sha1Ctx, mut data: &[u8]) {
    let mut partial = (ctx.bytecount as usize) % SHA1_BLOCK_SIZE;
    ctx.bytecount = ctx.bytecount.wrapping_add(data.len() as u64);

    if partial + data.len() >= SHA1_BLOCK_SIZE {
        if partial != 0 {
            let take = SHA1_BLOCK_SIZE - partial;
            ctx.buf[partial..partial + take].copy_from_slice(&data[..take]);
            sha1_blocks(&mut ctx.state, &ctx.buf);
            data = &data[take..];
        }
        let nblocks_len = data.len() / SHA1_BLOCK_SIZE * SHA1_BLOCK_SIZE;
        if nblocks_len != 0 {
            sha1_blocks(&mut ctx.state, &data[..nblocks_len]);
            data = &data[nblocks_len..];
        }
        partial = 0;
    }
    if !data.is_empty() {
        ctx.buf[partial..partial + data.len()].copy_from_slice(data);
    }
}

fn sha1_final_nozero(ctx: &mut Sha1Ctx, out: &mut [u8; SHA1_DIGEST_SIZE]) {
    let bitcount = ctx.bytecount << 3;
    let mut partial = (ctx.bytecount as usize) % SHA1_BLOCK_SIZE;
    ctx.buf[partial] = 0x80;
    partial += 1;
    if partial > SHA1_BLOCK_SIZE - 8 {
        ctx.buf[partial..].fill(0);
        sha1_blocks(&mut ctx.state, &ctx.buf);
        partial = 0;
    }
    ctx.buf[partial..SHA1_BLOCK_SIZE - 8].fill(0);
    ctx.buf[SHA1_BLOCK_SIZE - 8..].copy_from_slice(&bitcount.to_be_bytes());
    sha1_blocks(&mut ctx.state, &ctx.buf);
    for i in 0..SHA1_HASH_WORDS {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx.state.h[i].to_be_bytes());
    }
}

pub fn sha1_final(ctx: &mut Sha1Ctx, out: &mut [u8; SHA1_DIGEST_SIZE]) {
    sha1_final_nozero(ctx, out);
    *ctx = Sha1Ctx {
        state: Sha1BlockState { h: [0; 5] },
        bytecount: 0,
        buf: [0; SHA1_BLOCK_SIZE],
    };
}

pub fn sha1(data: &[u8]) -> [u8; SHA1_DIGEST_SIZE] {
    let mut ctx = Sha1Ctx::default();
    sha1_update(&mut ctx, data);
    let mut out = [0u8; SHA1_DIGEST_SIZE];
    sha1_final(&mut ctx, &mut out);
    out
}

fn hmac_sha1_preparekey_inner(
    istate: &mut Sha1BlockState,
    ostate: &mut Sha1BlockState,
    raw_key: &[u8],
) {
    let mut derived_key = [0u8; SHA1_BLOCK_SIZE];
    if raw_key.len() > SHA1_BLOCK_SIZE {
        derived_key[..SHA1_DIGEST_SIZE].copy_from_slice(&sha1(raw_key));
    } else {
        derived_key[..raw_key.len()].copy_from_slice(raw_key);
    }

    let mut ipad = derived_key;
    for byte in &mut ipad {
        *byte ^= HMAC_IPAD_VALUE;
    }
    *istate = Sha1BlockState::default();
    sha1_blocks(istate, &ipad);

    let mut opad = derived_key;
    for byte in &mut opad {
        *byte ^= HMAC_OPAD_VALUE;
    }
    *ostate = Sha1BlockState::default();
    sha1_blocks(ostate, &opad);
}

pub fn hmac_sha1_preparekey(key: &mut HmacSha1Key, raw_key: &[u8]) {
    let mut istate = Sha1BlockState::default();
    let mut ostate = Sha1BlockState::default();
    hmac_sha1_preparekey_inner(&mut istate, &mut ostate, raw_key);
    key.istate = istate;
    key.ostate = ostate;
}

pub fn hmac_sha1_init(ctx: &mut HmacSha1Ctx, key: &HmacSha1Key) {
    ctx.sha_ctx = Sha1Ctx {
        state: key.istate,
        bytecount: SHA1_BLOCK_SIZE as u64,
        buf: [0; SHA1_BLOCK_SIZE],
    };
    ctx.ostate = key.ostate;
}

pub fn hmac_sha1_init_usingrawkey(ctx: &mut HmacSha1Ctx, raw_key: &[u8]) {
    let mut key = HmacSha1Key::default();
    hmac_sha1_preparekey(&mut key, raw_key);
    hmac_sha1_init(ctx, &key);
}

pub fn hmac_sha1_update(ctx: &mut HmacSha1Ctx, data: &[u8]) {
    sha1_update(&mut ctx.sha_ctx, data);
}

pub fn hmac_sha1_final(ctx: &mut HmacSha1Ctx, out: &mut [u8; SHA1_DIGEST_SIZE]) {
    let mut inner = [0u8; SHA1_DIGEST_SIZE];
    sha1_final_nozero(&mut ctx.sha_ctx, &mut inner);

    let mut block = [0u8; SHA1_BLOCK_SIZE];
    block[..SHA1_DIGEST_SIZE].copy_from_slice(&inner);
    block[SHA1_DIGEST_SIZE] = 0x80;
    block[SHA1_BLOCK_SIZE - 4..].copy_from_slice(
        &(8u32 * (SHA1_BLOCK_SIZE as u32 + SHA1_DIGEST_SIZE as u32)).to_be_bytes(),
    );
    sha1_blocks(&mut ctx.ostate, &block);
    for i in 0..SHA1_HASH_WORDS {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx.ostate.h[i].to_be_bytes());
    }
    ctx.sha_ctx = Sha1Ctx {
        state: Sha1BlockState { h: [0; 5] },
        bytecount: 0,
        buf: [0; SHA1_BLOCK_SIZE],
    };
    ctx.ostate = Sha1BlockState { h: [0; 5] };
}

pub fn hmac_sha1(key: &HmacSha1Key, data: &[u8]) -> [u8; SHA1_DIGEST_SIZE] {
    let mut ctx = HmacSha1Ctx::default();
    hmac_sha1_init(&mut ctx, key);
    hmac_sha1_update(&mut ctx, data);
    let mut out = [0u8; SHA1_DIGEST_SIZE];
    hmac_sha1_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha1_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; SHA1_DIGEST_SIZE] {
    let mut ctx = HmacSha1Ctx::default();
    hmac_sha1_init_usingrawkey(&mut ctx, raw_key);
    hmac_sha1_update(&mut ctx, data);
    let mut out = [0u8; SHA1_DIGEST_SIZE];
    hmac_sha1_final(&mut ctx, &mut out);
    out
}

pub unsafe extern "C" fn sha1_init_raw(ctx: *mut Sha1Ctx) {
    if !ctx.is_null() {
        unsafe { sha1_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn sha1_update_raw(ctx: *mut Sha1Ctx, data: *const u8, len: usize) {
    if ctx.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    unsafe { sha1_update(&mut *ctx, data) };
}

pub unsafe extern "C" fn sha1_final_raw(ctx: *mut Sha1Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA1_DIGEST_SIZE]) };
    unsafe { sha1_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sha1_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sha1(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA1_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha1_preparekey_raw(
    key: *mut HmacSha1Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha1_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn hmac_sha1_init_raw(ctx: *mut HmacSha1Ctx, key: *const HmacSha1Key) {
    if ctx.is_null() || key.is_null() {
        return;
    }
    unsafe { hmac_sha1_init(&mut *ctx, &*key) };
}

pub unsafe extern "C" fn hmac_sha1_init_usingrawkey_raw(
    ctx: *mut HmacSha1Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha1_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_sha1_final_raw(ctx: *mut HmacSha1Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA1_DIGEST_SIZE]) };
    unsafe { hmac_sha1_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_sha1_raw(
    key: *const HmacSha1Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_sha1(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA1_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha1_usingrawkey_raw(
    raw_key: *const u8,
    raw_key_len: usize,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if out.is_null() || (raw_key.is_null() && raw_key_len != 0) || (data.is_null() && data_len != 0)
    {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = hmac_sha1_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA1_DIGEST_SIZE) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; SHA1_DIGEST_SIZE])> {
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
                if digest.len() == SHA1_DIGEST_SIZE {
                    let mut array = [0u8; SHA1_DIGEST_SIZE];
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
    fn sha1_matches_linux_kunit_vectors_and_hmac() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/sha1.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha1-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));
        assert!(source.contains("void sha1_update(struct sha1_ctx *ctx"));
        assert!(source.contains("void hmac_sha1_preparekey(struct hmac_sha1_key *key"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sha1_final);"));
        assert!(vectors.contains("hash_testvec_consolidated[SHA1_DIGEST_SIZE]"));
        assert!(vectors.contains("hmac_testvec_consolidated[SHA1_DIGEST_SIZE]"));
        assert!(template.contains("HASH_UPDATE(&ctx, &test_buf[cur_offset], part_len);"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            assert_eq!(sha1(&data), expected, "data_len={len}");
        }

        let test_buf = rand_bytes_seeded_from_len(4096);
        let mut consolidated_ctx = Sha1Ctx::default();
        for len in 0..=4096 {
            let digest = sha1(&test_buf[..len]);
            sha1_update(&mut consolidated_ctx, &digest);
        }
        let mut consolidated = [0u8; SHA1_DIGEST_SIZE];
        sha1_final(&mut consolidated_ctx, &mut consolidated);
        assert_eq!(
            consolidated,
            parse_named_array(vectors, "hash_testvec_consolidated")
        );

        let mut ctx = Sha1Ctx::default();
        sha1_update(&mut ctx, b"The quick ");
        sha1_update(&mut ctx, b"brown fox");
        let mut digest = [0u8; SHA1_DIGEST_SIZE];
        sha1_final(&mut ctx, &mut digest);
        assert_eq!(digest, sha1(b"The quick brown fox"));

        let mut raw_key = rand_bytes_seeded_from_len(32);
        let mut key = HmacSha1Key::default();
        hmac_sha1_preparekey(&mut key, &raw_key);
        let mut hmac_ctx = HmacSha1Ctx::default();
        hmac_sha1_init(&mut hmac_ctx, &key);
        for data_len in 0..=4096 {
            let key_len = data_len % 293;
            hmac_sha1_update(&mut hmac_ctx, &test_buf[..data_len]);
            raw_key = rand_bytes_seeded_from_len(key_len);
            let mac = hmac_sha1_usingrawkey(&raw_key, &test_buf[..data_len]);
            hmac_sha1_update(&mut hmac_ctx, &mac);
            hmac_sha1_preparekey(&mut key, &raw_key);
            assert_eq!(hmac_sha1(&key, &test_buf[..data_len]), mac);
        }
        let mut mac = [0u8; SHA1_DIGEST_SIZE];
        hmac_sha1_final(&mut hmac_ctx, &mut mac);
        assert_eq!(mac, parse_named_array(vectors, "hmac_testvec_consolidated"));
        assert_eq!(hmac_ctx.sha_ctx.state.h, [0; SHA1_HASH_WORDS]);
        assert_eq!(hmac_ctx.sha_ctx.bytecount, 0);
        assert_eq!(hmac_ctx.sha_ctx.buf, [0; SHA1_BLOCK_SIZE]);
        assert_eq!(hmac_ctx.ostate.h, [0; SHA1_HASH_WORDS]);

        assert_eq!(
            hmac_sha1_usingrawkey(b"key", b"The quick brown fox jumps over the lazy dog"),
            [
                0xde, 0x7c, 0x9b, 0x85, 0xb8, 0xb7, 0x8a, 0xa6, 0xbc, 0x8a, 0x7a, 0x36, 0xf7, 0x0a,
                0x90, 0x70, 0x1c, 0x9d, 0xb4, 0xd9,
            ]
        );
    }
}
