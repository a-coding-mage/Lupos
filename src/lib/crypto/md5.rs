//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/md5.c
//! test-origin: linux:vendor/linux/lib/crypto/md5.c
//! MD5 and HMAC-MD5 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const MD5_DIGEST_SIZE: usize = 16;
pub const MD5_HMAC_BLOCK_SIZE: usize = 64;
pub const MD5_BLOCK_SIZE: usize = 64;
pub const MD5_BLOCK_WORDS: usize = 16;
pub const MD5_HASH_WORDS: usize = 4;
pub const MD5_H0: u32 = 0x6745_2301;
pub const MD5_H1: u32 = 0xefcd_ab89;
pub const MD5_H2: u32 = 0x98ba_dcfe;
pub const MD5_H3: u32 = 0x1032_5476;
const HMAC_IPAD_VALUE: u8 = 0x36;
const HMAC_OPAD_VALUE: u8 = 0x5c;

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Md5BlockState {
    pub h: [u32; MD5_HASH_WORDS],
}

impl Default for Md5BlockState {
    fn default() -> Self {
        Self {
            h: [MD5_H0, MD5_H1, MD5_H2, MD5_H3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Md5Ctx {
    pub state: Md5BlockState,
    pub bytecount: u64,
    pub buf: [u8; MD5_BLOCK_SIZE],
}

impl Default for Md5Ctx {
    fn default() -> Self {
        Self {
            state: Md5BlockState::default(),
            bytecount: 0,
            buf: [0; MD5_BLOCK_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct HmacMd5Key {
    pub istate: Md5BlockState,
    pub ostate: Md5BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct HmacMd5Ctx {
    pub hash_ctx: Md5Ctx,
    pub ostate: Md5BlockState,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("md5_init", md5_init_raw as usize, true);
    export_symbol_once("md5_update", md5_update_raw as usize, true);
    export_symbol_once("md5_final", md5_final_raw as usize, true);
    export_symbol_once("md5", md5_raw as usize, true);
    export_symbol_once(
        "hmac_md5_preparekey",
        hmac_md5_preparekey_raw as usize,
        true,
    );
    export_symbol_once("hmac_md5_init", hmac_md5_init_raw as usize, true);
    export_symbol_once(
        "hmac_md5_init_usingrawkey",
        hmac_md5_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once("hmac_md5_final", hmac_md5_final_raw as usize, true);
    export_symbol_once("hmac_md5", hmac_md5_raw as usize, true);
    export_symbol_once(
        "hmac_md5_usingrawkey",
        hmac_md5_usingrawkey_raw as usize,
        true,
    );
}

pub fn md5_init(ctx: &mut Md5Ctx) {
    *ctx = Md5Ctx::default();
}

fn md5_block_generic(state: &mut Md5BlockState, data: &[u8; MD5_BLOCK_SIZE]) {
    let mut m = [0u32; MD5_BLOCK_WORDS];
    for i in 0..MD5_BLOCK_WORDS {
        let offset = i * 4;
        m[i] = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
    }

    let mut a = state.h[0];
    let mut b = state.h[1];
    let mut c = state.h[2];
    let mut d = state.h[3];

    for i in 0..64 {
        let (f, g) = if i < 16 {
            ((b & c) | ((!b) & d), i)
        } else if i < 32 {
            ((d & b) | ((!d) & c), (5 * i + 1) % 16)
        } else if i < 48 {
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            (c ^ (b | !d), (7 * i) % 16)
        };
        let tmp = d;
        d = c;
        c = b;
        b = b.wrapping_add(
            a.wrapping_add(f)
                .wrapping_add(K[i])
                .wrapping_add(m[g])
                .rotate_left(S[i]),
        );
        a = tmp;
    }

    state.h[0] = state.h[0].wrapping_add(a);
    state.h[1] = state.h[1].wrapping_add(b);
    state.h[2] = state.h[2].wrapping_add(c);
    state.h[3] = state.h[3].wrapping_add(d);
}

fn md5_blocks(state: &mut Md5BlockState, data: &[u8]) {
    for block in data.chunks_exact(MD5_BLOCK_SIZE) {
        let block = <&[u8; MD5_BLOCK_SIZE]>::try_from(block).unwrap();
        md5_block_generic(state, block);
    }
}

pub fn md5_update(ctx: &mut Md5Ctx, mut data: &[u8]) {
    let mut partial = (ctx.bytecount as usize) % MD5_BLOCK_SIZE;
    ctx.bytecount = ctx.bytecount.wrapping_add(data.len() as u64);

    if partial + data.len() >= MD5_BLOCK_SIZE {
        if partial != 0 {
            let take = MD5_BLOCK_SIZE - partial;
            ctx.buf[partial..partial + take].copy_from_slice(&data[..take]);
            md5_blocks(&mut ctx.state, &ctx.buf);
            data = &data[take..];
        }

        let nblocks_len = data.len() / MD5_BLOCK_SIZE * MD5_BLOCK_SIZE;
        if nblocks_len != 0 {
            md5_blocks(&mut ctx.state, &data[..nblocks_len]);
            data = &data[nblocks_len..];
        }
        partial = 0;
    }
    if !data.is_empty() {
        ctx.buf[partial..partial + data.len()].copy_from_slice(data);
    }
}

fn md5_final_nozero(ctx: &mut Md5Ctx, out: &mut [u8; MD5_DIGEST_SIZE]) {
    let bitcount = ctx.bytecount << 3;
    let mut partial = (ctx.bytecount as usize) % MD5_BLOCK_SIZE;
    ctx.buf[partial] = 0x80;
    partial += 1;
    if partial > MD5_BLOCK_SIZE - 8 {
        ctx.buf[partial..].fill(0);
        md5_blocks(&mut ctx.state, &ctx.buf);
        partial = 0;
    }
    ctx.buf[partial..MD5_BLOCK_SIZE - 8].fill(0);
    ctx.buf[MD5_BLOCK_SIZE - 8..].copy_from_slice(&bitcount.to_le_bytes());
    md5_blocks(&mut ctx.state, &ctx.buf);

    for i in 0..MD5_HASH_WORDS {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx.state.h[i].to_le_bytes());
    }
}

pub fn md5_final(ctx: &mut Md5Ctx, out: &mut [u8; MD5_DIGEST_SIZE]) {
    md5_final_nozero(ctx, out);
    *ctx = Md5Ctx {
        state: Md5BlockState { h: [0; 4] },
        bytecount: 0,
        buf: [0; MD5_BLOCK_SIZE],
    };
}

pub fn md5(data: &[u8]) -> [u8; MD5_DIGEST_SIZE] {
    let mut ctx = Md5Ctx::default();
    md5_update(&mut ctx, data);
    let mut out = [0u8; MD5_DIGEST_SIZE];
    md5_final(&mut ctx, &mut out);
    out
}

fn hmac_md5_preparekey_inner(
    istate: &mut Md5BlockState,
    ostate: &mut Md5BlockState,
    raw_key: &[u8],
) {
    let mut derived_key = [0u8; MD5_BLOCK_SIZE];
    if raw_key.len() > MD5_BLOCK_SIZE {
        derived_key[..MD5_DIGEST_SIZE].copy_from_slice(&md5(raw_key));
    } else {
        derived_key[..raw_key.len()].copy_from_slice(raw_key);
    }
    let mut ipad = derived_key;
    for byte in &mut ipad {
        *byte ^= HMAC_IPAD_VALUE;
    }
    *istate = Md5BlockState::default();
    md5_blocks(istate, &ipad);

    let mut opad = derived_key;
    for byte in &mut opad {
        *byte ^= HMAC_OPAD_VALUE;
    }
    *ostate = Md5BlockState::default();
    md5_blocks(ostate, &opad);
}

pub fn hmac_md5_preparekey(key: &mut HmacMd5Key, raw_key: &[u8]) {
    let mut istate = Md5BlockState::default();
    let mut ostate = Md5BlockState::default();
    hmac_md5_preparekey_inner(&mut istate, &mut ostate, raw_key);
    key.istate = istate;
    key.ostate = ostate;
}

pub fn hmac_md5_init(ctx: &mut HmacMd5Ctx, key: &HmacMd5Key) {
    ctx.hash_ctx = Md5Ctx {
        state: key.istate,
        bytecount: MD5_BLOCK_SIZE as u64,
        buf: [0; MD5_BLOCK_SIZE],
    };
    ctx.ostate = key.ostate;
}

pub fn hmac_md5_init_usingrawkey(ctx: &mut HmacMd5Ctx, raw_key: &[u8]) {
    let mut key = HmacMd5Key::default();
    hmac_md5_preparekey(&mut key, raw_key);
    hmac_md5_init(ctx, &key);
}

pub fn hmac_md5_update(ctx: &mut HmacMd5Ctx, data: &[u8]) {
    md5_update(&mut ctx.hash_ctx, data);
}

pub fn hmac_md5_final(ctx: &mut HmacMd5Ctx, out: &mut [u8; MD5_DIGEST_SIZE]) {
    let mut inner = [0u8; MD5_DIGEST_SIZE];
    md5_final_nozero(&mut ctx.hash_ctx, &mut inner);

    let mut block = [0u8; MD5_BLOCK_SIZE];
    block[..MD5_DIGEST_SIZE].copy_from_slice(&inner);
    block[MD5_DIGEST_SIZE] = 0x80;
    block[MD5_BLOCK_SIZE - 8..]
        .copy_from_slice(&(8u64 * (MD5_BLOCK_SIZE as u64 + MD5_DIGEST_SIZE as u64)).to_le_bytes());
    md5_blocks(&mut ctx.ostate, &block);
    for i in 0..MD5_HASH_WORDS {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx.ostate.h[i].to_le_bytes());
    }
    ctx.hash_ctx = Md5Ctx {
        state: Md5BlockState { h: [0; 4] },
        bytecount: 0,
        buf: [0; MD5_BLOCK_SIZE],
    };
    ctx.ostate = Md5BlockState { h: [0; 4] };
}

pub fn hmac_md5(key: &HmacMd5Key, data: &[u8]) -> [u8; MD5_DIGEST_SIZE] {
    let mut ctx = HmacMd5Ctx::default();
    hmac_md5_init(&mut ctx, key);
    hmac_md5_update(&mut ctx, data);
    let mut out = [0u8; MD5_DIGEST_SIZE];
    hmac_md5_final(&mut ctx, &mut out);
    out
}

pub fn hmac_md5_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; MD5_DIGEST_SIZE] {
    let mut ctx = HmacMd5Ctx::default();
    hmac_md5_init_usingrawkey(&mut ctx, raw_key);
    hmac_md5_update(&mut ctx, data);
    let mut out = [0u8; MD5_DIGEST_SIZE];
    hmac_md5_final(&mut ctx, &mut out);
    out
}

pub unsafe extern "C" fn md5_init_raw(ctx: *mut Md5Ctx) {
    if !ctx.is_null() {
        unsafe { md5_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn md5_update_raw(ctx: *mut Md5Ctx, data: *const u8, len: usize) {
    if ctx.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    unsafe { md5_update(&mut *ctx, data) };
}

pub unsafe extern "C" fn md5_final_raw(ctx: *mut Md5Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; MD5_DIGEST_SIZE]) };
    unsafe { md5_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn md5_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = md5(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, MD5_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_md5_preparekey_raw(
    key: *mut HmacMd5Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_md5_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn hmac_md5_init_raw(ctx: *mut HmacMd5Ctx, key: *const HmacMd5Key) {
    if ctx.is_null() || key.is_null() {
        return;
    }
    unsafe { hmac_md5_init(&mut *ctx, &*key) };
}

pub unsafe extern "C" fn hmac_md5_init_usingrawkey_raw(
    ctx: *mut HmacMd5Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_md5_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_md5_final_raw(ctx: *mut HmacMd5Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; MD5_DIGEST_SIZE]) };
    unsafe { hmac_md5_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_md5_raw(
    key: *const HmacMd5Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_md5(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, MD5_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_md5_usingrawkey_raw(
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
    let digest = hmac_md5_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, MD5_DIGEST_SIZE) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; MD5_DIGEST_SIZE])> {
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
                if digest.len() == MD5_DIGEST_SIZE {
                    let mut array = [0u8; MD5_DIGEST_SIZE];
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
    fn md5_matches_linux_kunit_vectors_and_hmac() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/md5.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/md5-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));
        assert!(source.contains("void md5_update(struct md5_ctx *ctx"));
        assert!(source.contains("void hmac_md5_preparekey(struct hmac_md5_key *key"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(md5_final);"));
        assert!(vectors.contains("hash_testvec_consolidated[MD5_DIGEST_SIZE]"));
        assert!(vectors.contains("hmac_testvec_consolidated[MD5_DIGEST_SIZE]"));
        assert!(template.contains("rand_bytes_seeded_from_len(test_buf, data_len);"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            assert_eq!(md5(&data), expected, "data_len={len}");
        }

        let test_buf = rand_bytes_seeded_from_len(4096);
        let mut consolidated_ctx = Md5Ctx::default();
        for len in 0..=4096 {
            let digest = md5(&test_buf[..len]);
            md5_update(&mut consolidated_ctx, &digest);
        }
        let mut consolidated = [0u8; MD5_DIGEST_SIZE];
        md5_final(&mut consolidated_ctx, &mut consolidated);
        assert_eq!(
            consolidated,
            parse_named_array(vectors, "hash_testvec_consolidated")
        );

        let mut ctx = Md5Ctx::default();
        md5_update(&mut ctx, b"The quick ");
        md5_update(&mut ctx, b"brown fox");
        let mut digest = [0u8; MD5_DIGEST_SIZE];
        md5_final(&mut ctx, &mut digest);
        assert_eq!(digest, md5(b"The quick brown fox"));

        let mut raw_key = rand_bytes_seeded_from_len(32);
        let mut key = HmacMd5Key::default();
        hmac_md5_preparekey(&mut key, &raw_key);
        let mut hmac_ctx = HmacMd5Ctx::default();
        hmac_md5_init(&mut hmac_ctx, &key);
        for data_len in 0..=4096 {
            let key_len = data_len % 293;
            hmac_md5_update(&mut hmac_ctx, &test_buf[..data_len]);
            raw_key = rand_bytes_seeded_from_len(key_len);
            let mac = hmac_md5_usingrawkey(&raw_key, &test_buf[..data_len]);
            hmac_md5_update(&mut hmac_ctx, &mac);
            hmac_md5_preparekey(&mut key, &raw_key);
            assert_eq!(hmac_md5(&key, &test_buf[..data_len]), mac);
        }
        let mut mac = [0u8; MD5_DIGEST_SIZE];
        hmac_md5_final(&mut hmac_ctx, &mut mac);
        assert_eq!(mac, parse_named_array(vectors, "hmac_testvec_consolidated"));
        assert_eq!(hmac_ctx.hash_ctx.state.h, [0; MD5_HASH_WORDS]);
        assert_eq!(hmac_ctx.hash_ctx.bytecount, 0);
        assert_eq!(hmac_ctx.hash_ctx.buf, [0; MD5_BLOCK_SIZE]);
        assert_eq!(hmac_ctx.ostate.h, [0; MD5_HASH_WORDS]);

        assert_eq!(
            hmac_md5_usingrawkey(b"key", b"The quick brown fox jumps over the lazy dog"),
            [
                0x80, 0x07, 0x07, 0x13, 0x46, 0x3e, 0x77, 0x49, 0xb9, 0x0c, 0x2d, 0xc2, 0x49, 0x11,
                0xe2, 0x75,
            ]
        );
    }
}
