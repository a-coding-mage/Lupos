//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/poly1305.c
//! linux-source: vendor/linux/lib/crypto/poly1305-donna64.c
//! test-origin: linux:vendor/linux/lib/crypto/poly1305.c
//! Poly1305 authenticator algorithm, RFC7539.

use crate::kernel::module::{export_symbol, find_symbol};

pub const POLY1305_BLOCK_SIZE: usize = 16;
pub const POLY1305_KEY_SIZE: usize = 32;
pub const POLY1305_DIGEST_SIZE: usize = 16;
pub const MODULE_DESCRIPTION: &str = "Poly1305 authenticator algorithm, RFC7539";

const MASK44: u64 = 0x0fff_ffff_ffff;
const MASK42: u64 = 0x03ff_ffff_ffff;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Poly1305CoreKey {
    pub r64: [u64; 3],
    pub s64: [u64; 2],
}

impl Poly1305CoreKey {
    pub const fn new() -> Self {
        Self {
            r64: [0; 3],
            s64: [0; 2],
        }
    }
}

impl Default for Poly1305CoreKey {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Poly1305State {
    pub h64: [u64; 3],
}

impl Poly1305State {
    pub const fn new() -> Self {
        Self { h64: [0; 3] }
    }
}

impl Default for Poly1305State {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Poly1305BlockState {
    pub h: Poly1305State,
    pub core_r: Poly1305CoreKey,
}

impl Poly1305BlockState {
    pub const fn new() -> Self {
        Self {
            h: Poly1305State::new(),
            core_r: Poly1305CoreKey::new(),
        }
    }
}

impl Default for Poly1305BlockState {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Poly1305DescCtx {
    pub buf: [u8; POLY1305_BLOCK_SIZE],
    pub buflen: usize,
    pub s: [u32; 4],
    pub state: Poly1305BlockState,
}

impl Poly1305DescCtx {
    pub const fn new() -> Self {
        Self {
            buf: [0; POLY1305_BLOCK_SIZE],
            buflen: 0,
            s: [0; 4],
            state: Poly1305BlockState::new(),
        }
    }
}

impl Default for Poly1305DescCtx {
    fn default() -> Self {
        Self::new()
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("poly1305_init", poly1305_init_raw as usize, false);
    export_symbol_once("poly1305_update", poly1305_update_raw as usize, false);
    export_symbol_once("poly1305_final", poly1305_final_raw as usize, false);
}

pub fn poly1305_core_setkey(key: &mut Poly1305CoreKey, raw_key: &[u8; POLY1305_BLOCK_SIZE]) {
    let t0 = u64::from_le_bytes(raw_key[0..8].try_into().unwrap());
    let t1 = u64::from_le_bytes(raw_key[8..16].try_into().unwrap());

    key.r64[0] = t0 & 0xffc0fffffff;
    key.r64[1] = ((t0 >> 44) | (t1 << 20)) & 0xfffffc0ffff;
    key.r64[2] = (t1 >> 24) & 0x00ffffffc0f;

    key.s64[0] = key.r64[1] * 20;
    key.s64[1] = key.r64[2] * 20;
}

pub fn poly1305_core_init(state: &mut Poly1305State) {
    *state = Poly1305State::new();
}

pub fn poly1305_core_blocks(
    state: &mut Poly1305State,
    key: &Poly1305CoreKey,
    mut src: &[u8],
    nblocks: usize,
    hibit: u32,
) {
    if nblocks == 0 {
        return;
    }

    let hibit64 = (hibit as u64) << 40;
    let r0 = key.r64[0];
    let r1 = key.r64[1];
    let r2 = key.r64[2];
    let s1 = key.s64[0];
    let s2 = key.s64[1];
    let mut h0 = state.h64[0];
    let mut h1 = state.h64[1];
    let mut h2 = state.h64[2];

    for _ in 0..nblocks {
        let t0 = u64::from_le_bytes(src[0..8].try_into().unwrap());
        let t1 = u64::from_le_bytes(src[8..16].try_into().unwrap());

        h0 = h0.wrapping_add(t0 & MASK44);
        h1 = h1.wrapping_add(((t0 >> 44) | (t1 << 20)) & MASK44);
        h2 = h2.wrapping_add(((t1 >> 24) & MASK42) | hibit64);

        let mut d0 = (h0 as u128) * (r0 as u128);
        d0 += (h1 as u128) * (s2 as u128);
        d0 += (h2 as u128) * (s1 as u128);
        let mut d1 = (h0 as u128) * (r1 as u128);
        d1 += (h1 as u128) * (r0 as u128);
        d1 += (h2 as u128) * (s2 as u128);
        let mut d2 = (h0 as u128) * (r2 as u128);
        d2 += (h1 as u128) * (r1 as u128);
        d2 += (h2 as u128) * (r0 as u128);

        let mut c = (d0 >> 44) as u64;
        h0 = d0 as u64 & MASK44;
        d1 += c as u128;
        c = (d1 >> 44) as u64;
        h1 = d1 as u64 & MASK44;
        d2 += c as u128;
        c = (d2 >> 42) as u64;
        h2 = d2 as u64 & MASK42;
        h0 = h0.wrapping_add(c * 5);
        c = h0 >> 44;
        h0 &= MASK44;
        h1 = h1.wrapping_add(c);

        src = &src[POLY1305_BLOCK_SIZE..];
    }

    state.h64[0] = h0;
    state.h64[1] = h1;
    state.h64[2] = h2;
}

pub fn poly1305_core_emit(
    state: &Poly1305State,
    nonce: Option<&[u32; 4]>,
    dst: &mut [u8; POLY1305_DIGEST_SIZE],
) {
    let mut h0 = state.h64[0];
    let mut h1 = state.h64[1];
    let mut h2 = state.h64[2];

    let mut c = h1 >> 44;
    h1 &= MASK44;
    h2 = h2.wrapping_add(c);
    c = h2 >> 42;
    h2 &= MASK42;
    h0 = h0.wrapping_add(c * 5);
    c = h0 >> 44;
    h0 &= MASK44;
    h1 = h1.wrapping_add(c);
    c = h1 >> 44;
    h1 &= MASK44;
    h2 = h2.wrapping_add(c);
    c = h2 >> 42;
    h2 &= MASK42;
    h0 = h0.wrapping_add(c * 5);
    c = h0 >> 44;
    h0 &= MASK44;
    h1 = h1.wrapping_add(c);

    let mut g0 = h0.wrapping_add(5);
    c = g0 >> 44;
    g0 &= MASK44;
    let mut g1 = h1.wrapping_add(c);
    c = g1 >> 44;
    g1 &= MASK44;
    let mut g2 = h2.wrapping_add(c).wrapping_sub(1u64 << 42);

    c = (g2 >> 63).wrapping_sub(1);
    g0 &= c;
    g1 &= c;
    g2 &= c;
    c = !c;
    h0 = (h0 & c) | g0;
    h1 = (h1 & c) | g1;
    h2 = (h2 & c) | g2;

    if let Some(nonce) = nonce {
        let t0 = ((nonce[1] as u64) << 32) | nonce[0] as u64;
        let t1 = ((nonce[3] as u64) << 32) | nonce[2] as u64;

        h0 = h0.wrapping_add(t0 & MASK44);
        c = h0 >> 44;
        h0 &= MASK44;
        h1 = h1.wrapping_add(((t0 >> 44) | (t1 << 20)) & MASK44);
        h1 = h1.wrapping_add(c);
        c = h1 >> 44;
        h1 &= MASK44;
        h2 = h2.wrapping_add((t1 >> 24) & MASK42);
        h2 = h2.wrapping_add(c);
        h2 &= MASK42;
    }

    h0 |= h1 << 44;
    h1 = (h1 >> 20) | (h2 << 24);

    dst[0..8].copy_from_slice(&h0.to_le_bytes());
    dst[8..16].copy_from_slice(&h1.to_le_bytes());
}

pub fn poly1305_block_init(desc: &mut Poly1305BlockState, raw_key: &[u8; POLY1305_BLOCK_SIZE]) {
    poly1305_core_init(&mut desc.h);
    poly1305_core_setkey(&mut desc.core_r, raw_key);
}

pub fn poly1305_blocks(state: &mut Poly1305BlockState, src: &[u8], len: usize, padbit: u32) {
    poly1305_core_blocks(
        &mut state.h,
        &state.core_r,
        src,
        len / POLY1305_BLOCK_SIZE,
        padbit,
    );
}

pub fn poly1305_emit(
    state: &Poly1305State,
    dst: &mut [u8; POLY1305_DIGEST_SIZE],
    nonce: &[u32; 4],
) {
    poly1305_core_emit(state, Some(nonce), dst);
}

pub fn poly1305_init(desc: &mut Poly1305DescCtx, key: &[u8; POLY1305_KEY_SIZE]) {
    desc.s[0] = u32::from_le_bytes(key[16..20].try_into().unwrap());
    desc.s[1] = u32::from_le_bytes(key[20..24].try_into().unwrap());
    desc.s[2] = u32::from_le_bytes(key[24..28].try_into().unwrap());
    desc.s[3] = u32::from_le_bytes(key[28..32].try_into().unwrap());
    desc.buflen = 0;
    poly1305_block_init(
        &mut desc.state,
        key[..POLY1305_BLOCK_SIZE].try_into().unwrap(),
    );
}

pub fn poly1305_update(desc: &mut Poly1305DescCtx, mut src: &[u8]) {
    if desc.buflen + src.len() >= POLY1305_BLOCK_SIZE {
        if desc.buflen != 0 {
            let l = POLY1305_BLOCK_SIZE - desc.buflen;
            desc.buf[desc.buflen..POLY1305_BLOCK_SIZE].copy_from_slice(&src[..l]);
            src = &src[l..];

            let block = desc.buf;
            poly1305_blocks(&mut desc.state, &block, POLY1305_BLOCK_SIZE, 1);
            desc.buflen = 0;
        }

        let bulk_len = src.len() / POLY1305_BLOCK_SIZE * POLY1305_BLOCK_SIZE;
        let tail_len = src.len() % POLY1305_BLOCK_SIZE;

        if bulk_len != 0 {
            poly1305_blocks(&mut desc.state, &src[..bulk_len], bulk_len, 1);
            src = &src[bulk_len..];
        }
        debug_assert_eq!(src.len(), tail_len);
    }

    if !src.is_empty() {
        desc.buf[desc.buflen..desc.buflen + src.len()].copy_from_slice(src);
        desc.buflen += src.len();
    }
}

pub fn poly1305_final(desc: &mut Poly1305DescCtx, dst: &mut [u8; POLY1305_DIGEST_SIZE]) {
    if desc.buflen != 0 {
        desc.buf[desc.buflen] = 1;
        desc.buflen += 1;
        desc.buf[desc.buflen..POLY1305_BLOCK_SIZE].fill(0);
        let block = desc.buf;
        poly1305_blocks(&mut desc.state, &block, POLY1305_BLOCK_SIZE, 0);
    }

    poly1305_emit(&desc.state.h, dst, &desc.s);
    *desc = Poly1305DescCtx::new();
}

pub fn poly1305(key: &[u8; POLY1305_KEY_SIZE], input: &[u8], out: &mut [u8; POLY1305_DIGEST_SIZE]) {
    let mut ctx = Poly1305DescCtx::new();
    poly1305_init(&mut ctx, key);
    poly1305_update(&mut ctx, input);
    poly1305_final(&mut ctx, out);
}

pub unsafe extern "C" fn poly1305_init_raw(desc: *mut Poly1305DescCtx, key: *const u8) {
    if desc.is_null() || key.is_null() {
        return;
    }
    let key = unsafe { &*(key as *const [u8; POLY1305_KEY_SIZE]) };
    unsafe { poly1305_init(&mut *desc, key) };
}

pub unsafe extern "C" fn poly1305_update_raw(
    desc: *mut Poly1305DescCtx,
    src: *const u8,
    nbytes: u32,
) {
    if desc.is_null() || (src.is_null() && nbytes != 0) {
        return;
    }
    let src = unsafe { core::slice::from_raw_parts(src, nbytes as usize) };
    unsafe { poly1305_update(&mut *desc, src) };
}

pub unsafe extern "C" fn poly1305_final_raw(desc: *mut Poly1305DescCtx, dst: *mut u8) {
    if desc.is_null() || dst.is_null() {
        return;
    }
    let dst = unsafe { &mut *(dst as *mut [u8; POLY1305_DIGEST_SIZE]) };
    unsafe { poly1305_final(&mut *desc, dst) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; POLY1305_DIGEST_SIZE])> {
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
                if digest.len() == POLY1305_DIGEST_SIZE {
                    let mut array = [0u8; POLY1305_DIGEST_SIZE];
                    array.copy_from_slice(&digest);
                    out.push((data_len.unwrap(), array));
                    digest.clear();
                }
            }
        }
        out
    }

    #[test]
    fn poly1305_matches_linux_source_and_test_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/poly1305.c"
        ));
        let core = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/poly1305-donna64.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/poly1305.h"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/poly1305_kunit.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/poly1305-testvecs.h"
        ));
        assert!(source.contains("poly1305_block_init(&desc->state, key);"));
        assert!(source.contains("poly1305_blocks(&desc->state, src, bulk_len, 1);"));
        assert!(source.contains("poly1305_emit(&desc->state.h, dst, desc->s);"));
        assert!(source.contains("*desc = (struct poly1305_desc_ctx){};"));
        assert!(core.contains("poly1305_core_blocks(struct poly1305_state *state"));
        assert!(core.contains("key->key.r64[0] = t0 & 0xffc0fffffffULL;"));
        assert!(core.contains("poly1305_core_emit(const struct poly1305_state *state"));
        assert!(header.contains("struct poly1305_desc_ctx"));
        assert!(kunit.contains("test_poly1305_allones_keys_and_message"));
        assert!(vectors.contains("poly1305_allones_macofmacs"));

        let test_key: [u8; POLY1305_KEY_SIZE] = rand_bytes_seeded_from_len(POLY1305_KEY_SIZE)
            .try_into()
            .unwrap();
        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            let mut actual = [0u8; POLY1305_DIGEST_SIZE];
            poly1305(&test_key, &data, &mut actual);
            assert_eq!(actual, expected, "data_len={len}");
        }

        let mut mac_ctx = Poly1305DescCtx::new();
        let mut macofmacs_ctx = Poly1305DescCtx::new();
        let allones = [0xffu8; 4096];
        let key = [0xffu8; POLY1305_KEY_SIZE];
        let mut mac = [0u8; POLY1305_DIGEST_SIZE];
        poly1305_init(&mut mac_ctx, &key);
        poly1305_init(&mut macofmacs_ctx, &key);
        for _ in 0..32 {
            for len in (0..=4096).step_by(16) {
                poly1305_update(&mut mac_ctx, &allones[..len]);
                let mut tmp_ctx = mac_ctx;
                poly1305_final(&mut tmp_ctx, &mut mac);
                poly1305_update(&mut macofmacs_ctx, &mac);
            }
        }
        poly1305_final(&mut macofmacs_ctx, &mut mac);
        assert_eq!(
            mac,
            [
                0x0c, 0x26, 0x6b, 0x45, 0x87, 0x06, 0xcf, 0xc4, 0x3f, 0x70, 0x7d, 0xb3, 0x50, 0xdd,
                0x81, 0x25,
            ]
        );

        let key = [
            1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let mut data = [0u8; 3 * POLY1305_BLOCK_SIZE];
        for i in 1..=10u8 {
            data[0] = 0u8.wrapping_sub(i);
            data[1..POLY1305_BLOCK_SIZE].fill(0xff);
            let mut expected = [0u8; POLY1305_DIGEST_SIZE];
            if i <= 5 {
                expected[0] = 5 - i;
            } else {
                expected[0] = 0u8.wrapping_sub(i);
                expected[1..].fill(0xff);
            }
            let mut actual = [0u8; POLY1305_DIGEST_SIZE];
            poly1305(&key, &data, &mut actual);
            assert_eq!(actual, expected, "reduction i={i}");
        }
    }
}
