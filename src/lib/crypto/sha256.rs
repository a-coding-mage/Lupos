//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/sha256.c
//! test-origin: linux:vendor/linux/lib/crypto/sha256.c
//! SHA-224, SHA-256, HMAC-SHA224, and HMAC-SHA256 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const SHA224_DIGEST_SIZE: usize = 28;
pub const SHA224_BLOCK_SIZE: usize = 64;
pub const SHA256_DIGEST_SIZE: usize = 32;
pub const SHA256_BLOCK_SIZE: usize = 64;
pub const SHA256_STATE_WORDS: usize = 8;
const HMAC_IPAD_VALUE: u8 = 0x36;
const HMAC_OPAD_VALUE: u8 = 0x5c;

pub const SHA224_H0: u32 = 0xc105_9ed8;
pub const SHA224_H1: u32 = 0x367c_d507;
pub const SHA224_H2: u32 = 0x3070_dd17;
pub const SHA224_H3: u32 = 0xf70e_5939;
pub const SHA224_H4: u32 = 0xffc0_0b31;
pub const SHA224_H5: u32 = 0x6858_1511;
pub const SHA224_H6: u32 = 0x64f9_8fa7;
pub const SHA224_H7: u32 = 0xbefa_4fa4;

pub const SHA256_H0: u32 = 0x6a09_e667;
pub const SHA256_H1: u32 = 0xbb67_ae85;
pub const SHA256_H2: u32 = 0x3c6e_f372;
pub const SHA256_H3: u32 = 0xa54f_f53a;
pub const SHA256_H4: u32 = 0x510e_527f;
pub const SHA256_H5: u32 = 0x9b05_688c;
pub const SHA256_H6: u32 = 0x1f83_d9ab;
pub const SHA256_H7: u32 = 0x5be0_cd19;

const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sha256BlockState {
    pub h: [u32; SHA256_STATE_WORDS],
}

const SHA224_IV: Sha256BlockState = Sha256BlockState {
    h: [
        SHA224_H0, SHA224_H1, SHA224_H2, SHA224_H3, SHA224_H4, SHA224_H5, SHA224_H6, SHA224_H7,
    ],
};
const SHA256_IV: Sha256BlockState = Sha256BlockState {
    h: [
        SHA256_H0, SHA256_H1, SHA256_H2, SHA256_H3, SHA256_H4, SHA256_H5, SHA256_H6, SHA256_H7,
    ],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha256InnerCtx {
    pub state: Sha256BlockState,
    pub bytecount: u64,
    pub buf: [u8; SHA256_BLOCK_SIZE],
}

impl Default for Sha256InnerCtx {
    fn default() -> Self {
        Self {
            state: Sha256BlockState::default(),
            bytecount: 0,
            buf: [0; SHA256_BLOCK_SIZE],
        }
    }
}

impl Sha256InnerCtx {
    const fn new(state: Sha256BlockState, bytecount: u64) -> Self {
        Self {
            state,
            bytecount,
            buf: [0; SHA256_BLOCK_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha224Ctx {
    pub ctx: Sha256InnerCtx,
}

impl Default for Sha224Ctx {
    fn default() -> Self {
        Self {
            ctx: Sha256InnerCtx::new(SHA224_IV, 0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha256Ctx {
    pub ctx: Sha256InnerCtx,
}

impl Default for Sha256Ctx {
    fn default() -> Self {
        Self {
            ctx: Sha256InnerCtx::new(SHA256_IV, 0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha256KeyInner {
    pub istate: Sha256BlockState,
    pub ostate: Sha256BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha256CtxInner {
    pub sha_ctx: Sha256InnerCtx,
    pub ostate: Sha256BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha224Key {
    pub key: HmacSha256KeyInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha224Ctx {
    pub ctx: HmacSha256CtxInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha256Key {
    pub key: HmacSha256KeyInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha256Ctx {
    pub ctx: HmacSha256CtxInner,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sha224_init", sha224_init_raw as usize, true);
    export_symbol_once("sha256_init", sha256_init_raw as usize, true);
    export_symbol_once("__sha256_update", __sha256_update_raw as usize, false);
    export_symbol_once("sha224_final", sha224_final_raw as usize, false);
    export_symbol_once("sha256_final", sha256_final_raw as usize, false);
    export_symbol_once("sha224", sha224_raw as usize, false);
    export_symbol_once("sha256", sha256_raw as usize, false);
    export_symbol_once("sha256_finup_2x", sha256_finup_2x_raw as usize, true);
    export_symbol_once(
        "sha256_finup_2x_is_optimized",
        sha256_finup_2x_is_optimized_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha224_preparekey",
        hmac_sha224_preparekey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha256_preparekey",
        hmac_sha256_preparekey_raw as usize,
        true,
    );
    export_symbol_once("__hmac_sha256_init", __hmac_sha256_init_raw as usize, true);
    export_symbol_once(
        "hmac_sha224_init_usingrawkey",
        hmac_sha224_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha256_init_usingrawkey",
        hmac_sha256_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once("hmac_sha224_final", hmac_sha224_final_raw as usize, true);
    export_symbol_once("hmac_sha256_final", hmac_sha256_final_raw as usize, true);
    export_symbol_once("hmac_sha224", hmac_sha224_raw as usize, true);
    export_symbol_once("hmac_sha256", hmac_sha256_raw as usize, true);
    export_symbol_once(
        "hmac_sha224_usingrawkey",
        hmac_sha224_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha256_usingrawkey",
        hmac_sha256_usingrawkey_raw as usize,
        true,
    );
}

#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    z ^ (x & (y ^ z))
}

#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (z & (x | y))
}

#[inline]
fn e0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

#[inline]
fn e1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

#[inline]
fn s0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

#[inline]
fn s1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

fn sha256_block_generic(state: &mut Sha256BlockState, input: &[u8; SHA256_BLOCK_SIZE]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        let offset = i * 4;
        w[i] = u32::from_be_bytes([
            input[offset],
            input[offset + 1],
            input[offset + 2],
            input[offset + 3],
        ]);
    }
    for i in 16..64 {
        w[i] = s1(w[i - 2])
            .wrapping_add(w[i - 7])
            .wrapping_add(s0(w[i - 15]))
            .wrapping_add(w[i - 16]);
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
        let t1 = h
            .wrapping_add(e1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let t2 = e0(a).wrapping_add(maj(a, b, c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    state.h[0] = state.h[0].wrapping_add(a);
    state.h[1] = state.h[1].wrapping_add(b);
    state.h[2] = state.h[2].wrapping_add(c);
    state.h[3] = state.h[3].wrapping_add(d);
    state.h[4] = state.h[4].wrapping_add(e);
    state.h[5] = state.h[5].wrapping_add(f);
    state.h[6] = state.h[6].wrapping_add(g);
    state.h[7] = state.h[7].wrapping_add(h);
}

fn sha256_blocks(state: &mut Sha256BlockState, data: &[u8]) {
    for block in data.chunks_exact(SHA256_BLOCK_SIZE) {
        let block = <&[u8; SHA256_BLOCK_SIZE]>::try_from(block).unwrap();
        sha256_block_generic(state, block);
    }
}

fn __sha256_init(ctx: &mut Sha256InnerCtx, iv: &Sha256BlockState, initial_bytecount: u64) {
    ctx.state = *iv;
    ctx.bytecount = initial_bytecount;
    ctx.buf = [0; SHA256_BLOCK_SIZE];
}

pub fn sha224_init(ctx: &mut Sha224Ctx) {
    __sha256_init(&mut ctx.ctx, &SHA224_IV, 0);
}

pub fn sha256_init(ctx: &mut Sha256Ctx) {
    __sha256_init(&mut ctx.ctx, &SHA256_IV, 0);
}

pub fn __sha256_update(ctx: &mut Sha256InnerCtx, mut data: &[u8]) {
    let mut partial = (ctx.bytecount as usize) % SHA256_BLOCK_SIZE;
    ctx.bytecount = ctx.bytecount.wrapping_add(data.len() as u64);

    if partial + data.len() >= SHA256_BLOCK_SIZE {
        if partial != 0 {
            let take = SHA256_BLOCK_SIZE - partial;
            ctx.buf[partial..partial + take].copy_from_slice(&data[..take]);
            sha256_blocks(&mut ctx.state, &ctx.buf);
            data = &data[take..];
        }

        let nblocks_len = data.len() / SHA256_BLOCK_SIZE * SHA256_BLOCK_SIZE;
        if nblocks_len != 0 {
            sha256_blocks(&mut ctx.state, &data[..nblocks_len]);
            data = &data[nblocks_len..];
        }
        partial = 0;
    }
    if !data.is_empty() {
        ctx.buf[partial..partial + data.len()].copy_from_slice(data);
    }
}

pub fn sha224_update(ctx: &mut Sha224Ctx, data: &[u8]) {
    __sha256_update(&mut ctx.ctx, data);
}

pub fn sha256_update(ctx: &mut Sha256Ctx, data: &[u8]) {
    __sha256_update(&mut ctx.ctx, data);
}

fn __sha256_final_nozero(ctx: &mut Sha256InnerCtx, out: &mut [u8], digest_size: usize) {
    let bitcount = ctx.bytecount << 3;
    let mut partial = (ctx.bytecount as usize) % SHA256_BLOCK_SIZE;

    ctx.buf[partial] = 0x80;
    partial += 1;
    if partial > SHA256_BLOCK_SIZE - 8 {
        ctx.buf[partial..].fill(0);
        sha256_blocks(&mut ctx.state, &ctx.buf);
        partial = 0;
    }
    ctx.buf[partial..SHA256_BLOCK_SIZE - 8].fill(0);
    ctx.buf[SHA256_BLOCK_SIZE - 8..].copy_from_slice(&bitcount.to_be_bytes());
    sha256_blocks(&mut ctx.state, &ctx.buf);

    for i in (0..digest_size).step_by(4) {
        out[i..i + 4].copy_from_slice(&ctx.state.h[i / 4].to_be_bytes());
    }
}

pub fn sha224_final(ctx: &mut Sha224Ctx, out: &mut [u8; SHA224_DIGEST_SIZE]) {
    __sha256_final_nozero(&mut ctx.ctx, out, SHA224_DIGEST_SIZE);
    ctx.ctx = Sha256InnerCtx::default();
}

pub fn sha256_final(ctx: &mut Sha256Ctx, out: &mut [u8; SHA256_DIGEST_SIZE]) {
    __sha256_final_nozero(&mut ctx.ctx, out, SHA256_DIGEST_SIZE);
    ctx.ctx = Sha256InnerCtx::default();
}

pub fn sha224(data: &[u8]) -> [u8; SHA224_DIGEST_SIZE] {
    let mut ctx = Sha224Ctx::default();
    sha224_update(&mut ctx, data);
    let mut out = [0u8; SHA224_DIGEST_SIZE];
    sha224_final(&mut ctx, &mut out);
    out
}

pub fn sha256(data: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    let mut ctx = Sha256Ctx::default();
    sha256_update(&mut ctx, data);
    let mut out = [0u8; SHA256_DIGEST_SIZE];
    sha256_final(&mut ctx, &mut out);
    out
}

pub fn sha256_finup_2x(
    ctx: Option<&Sha256Ctx>,
    data1: &[u8],
    data2: &[u8],
    out1: &mut [u8; SHA256_DIGEST_SIZE],
    out2: &mut [u8; SHA256_DIGEST_SIZE],
) {
    assert_eq!(data1.len(), data2.len());
    let initial = ctx.map_or(Sha256InnerCtx::new(SHA256_IV, 0), |ctx| ctx.ctx);

    let mut mut_ctx = initial;
    __sha256_update(&mut mut_ctx, data1);
    __sha256_final_nozero(&mut mut_ctx, out1, SHA256_DIGEST_SIZE);

    mut_ctx = initial;
    __sha256_update(&mut mut_ctx, data2);
    __sha256_final_nozero(&mut mut_ctx, out2, SHA256_DIGEST_SIZE);
}

pub fn sha256_finup_2x_is_optimized() -> bool {
    false
}

fn __hmac_sha256_preparekey(
    istate: &mut Sha256BlockState,
    ostate: &mut Sha256BlockState,
    raw_key: &[u8],
    iv: &Sha256BlockState,
    digest_size: usize,
) {
    let mut derived_key = [0u8; SHA256_BLOCK_SIZE];
    if raw_key.len() > SHA256_BLOCK_SIZE {
        if digest_size == SHA224_DIGEST_SIZE {
            derived_key[..SHA224_DIGEST_SIZE].copy_from_slice(&sha224(raw_key));
        } else {
            derived_key[..SHA256_DIGEST_SIZE].copy_from_slice(&sha256(raw_key));
        }
    } else {
        derived_key[..raw_key.len()].copy_from_slice(raw_key);
    }

    let mut ipad = derived_key;
    for byte in &mut ipad {
        *byte ^= HMAC_IPAD_VALUE;
    }
    *istate = *iv;
    sha256_blocks(istate, &ipad);

    let mut opad = derived_key;
    for byte in &mut opad {
        *byte ^= HMAC_OPAD_VALUE;
    }
    *ostate = *iv;
    sha256_blocks(ostate, &opad);
}

pub fn hmac_sha224_preparekey(key: &mut HmacSha224Key, raw_key: &[u8]) {
    __hmac_sha256_preparekey(
        &mut key.key.istate,
        &mut key.key.ostate,
        raw_key,
        &SHA224_IV,
        SHA224_DIGEST_SIZE,
    );
}

pub fn hmac_sha256_preparekey(key: &mut HmacSha256Key, raw_key: &[u8]) {
    __hmac_sha256_preparekey(
        &mut key.key.istate,
        &mut key.key.ostate,
        raw_key,
        &SHA256_IV,
        SHA256_DIGEST_SIZE,
    );
}

pub fn __hmac_sha256_init(ctx: &mut HmacSha256CtxInner, key: &HmacSha256KeyInner) {
    __sha256_init(&mut ctx.sha_ctx, &key.istate, SHA256_BLOCK_SIZE as u64);
    ctx.ostate = key.ostate;
}

pub fn hmac_sha224_init(ctx: &mut HmacSha224Ctx, key: &HmacSha224Key) {
    __hmac_sha256_init(&mut ctx.ctx, &key.key);
}

pub fn hmac_sha256_init(ctx: &mut HmacSha256Ctx, key: &HmacSha256Key) {
    __hmac_sha256_init(&mut ctx.ctx, &key.key);
}

pub fn hmac_sha224_init_usingrawkey(ctx: &mut HmacSha224Ctx, raw_key: &[u8]) {
    __hmac_sha256_preparekey(
        &mut ctx.ctx.sha_ctx.state,
        &mut ctx.ctx.ostate,
        raw_key,
        &SHA224_IV,
        SHA224_DIGEST_SIZE,
    );
    ctx.ctx.sha_ctx.bytecount = SHA256_BLOCK_SIZE as u64;
    ctx.ctx.sha_ctx.buf = [0; SHA256_BLOCK_SIZE];
}

pub fn hmac_sha256_init_usingrawkey(ctx: &mut HmacSha256Ctx, raw_key: &[u8]) {
    __hmac_sha256_preparekey(
        &mut ctx.ctx.sha_ctx.state,
        &mut ctx.ctx.ostate,
        raw_key,
        &SHA256_IV,
        SHA256_DIGEST_SIZE,
    );
    ctx.ctx.sha_ctx.bytecount = SHA256_BLOCK_SIZE as u64;
    ctx.ctx.sha_ctx.buf = [0; SHA256_BLOCK_SIZE];
}

pub fn hmac_sha224_update(ctx: &mut HmacSha224Ctx, data: &[u8]) {
    __sha256_update(&mut ctx.ctx.sha_ctx, data);
}

pub fn hmac_sha256_update(ctx: &mut HmacSha256Ctx, data: &[u8]) {
    __sha256_update(&mut ctx.ctx.sha_ctx, data);
}

fn __hmac_sha256_final(ctx: &mut HmacSha256CtxInner, out: &mut [u8], digest_size: usize) {
    let mut inner = [0u8; SHA256_DIGEST_SIZE];
    __sha256_final_nozero(&mut ctx.sha_ctx, &mut inner[..digest_size], digest_size);

    let mut block = [0u8; SHA256_BLOCK_SIZE];
    block[..digest_size].copy_from_slice(&inner[..digest_size]);
    block[digest_size] = 0x80;
    block[SHA256_BLOCK_SIZE - 4..]
        .copy_from_slice(&(8u32 * (SHA256_BLOCK_SIZE as u32 + digest_size as u32)).to_be_bytes());

    sha256_blocks(&mut ctx.ostate, &block);
    for i in (0..digest_size).step_by(4) {
        out[i..i + 4].copy_from_slice(&ctx.ostate.h[i / 4].to_be_bytes());
    }
    *ctx = HmacSha256CtxInner::default();
}

pub fn hmac_sha224_final(ctx: &mut HmacSha224Ctx, out: &mut [u8; SHA224_DIGEST_SIZE]) {
    __hmac_sha256_final(&mut ctx.ctx, out, SHA224_DIGEST_SIZE);
}

pub fn hmac_sha256_final(ctx: &mut HmacSha256Ctx, out: &mut [u8; SHA256_DIGEST_SIZE]) {
    __hmac_sha256_final(&mut ctx.ctx, out, SHA256_DIGEST_SIZE);
}

pub fn hmac_sha224(key: &HmacSha224Key, data: &[u8]) -> [u8; SHA224_DIGEST_SIZE] {
    let mut ctx = HmacSha224Ctx::default();
    hmac_sha224_init(&mut ctx, key);
    hmac_sha224_update(&mut ctx, data);
    let mut out = [0u8; SHA224_DIGEST_SIZE];
    hmac_sha224_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha256(key: &HmacSha256Key, data: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    let mut ctx = HmacSha256Ctx::default();
    hmac_sha256_init(&mut ctx, key);
    hmac_sha256_update(&mut ctx, data);
    let mut out = [0u8; SHA256_DIGEST_SIZE];
    hmac_sha256_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha224_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; SHA224_DIGEST_SIZE] {
    let mut ctx = HmacSha224Ctx::default();
    hmac_sha224_init_usingrawkey(&mut ctx, raw_key);
    hmac_sha224_update(&mut ctx, data);
    let mut out = [0u8; SHA224_DIGEST_SIZE];
    hmac_sha224_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha256_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; SHA256_DIGEST_SIZE] {
    let mut ctx = HmacSha256Ctx::default();
    hmac_sha256_init_usingrawkey(&mut ctx, raw_key);
    hmac_sha256_update(&mut ctx, data);
    let mut out = [0u8; SHA256_DIGEST_SIZE];
    hmac_sha256_final(&mut ctx, &mut out);
    out
}

pub unsafe extern "C" fn sha224_init_raw(ctx: *mut Sha224Ctx) {
    if !ctx.is_null() {
        unsafe { sha224_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn sha256_init_raw(ctx: *mut Sha256Ctx) {
    if !ctx.is_null() {
        unsafe { sha256_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn __sha256_update_raw(
    ctx: *mut Sha256InnerCtx,
    data: *const u8,
    len: usize,
) {
    if ctx.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    unsafe { __sha256_update(&mut *ctx, data) };
}

pub unsafe extern "C" fn sha224_final_raw(ctx: *mut Sha224Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA224_DIGEST_SIZE]) };
    unsafe { sha224_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sha256_final_raw(ctx: *mut Sha256Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA256_DIGEST_SIZE]) };
    unsafe { sha256_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sha224_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sha224(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA224_DIGEST_SIZE) };
}

pub unsafe extern "C" fn sha256_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sha256(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA256_DIGEST_SIZE) };
}

pub unsafe extern "C" fn sha256_finup_2x_raw(
    ctx: *const Sha256Ctx,
    data1: *const u8,
    data2: *const u8,
    len: usize,
    out1: *mut u8,
    out2: *mut u8,
) {
    if out1.is_null()
        || out2.is_null()
        || (data1.is_null() && len != 0)
        || (data2.is_null() && len != 0)
    {
        return;
    }
    let data1 = unsafe { core::slice::from_raw_parts(data1, len) };
    let data2 = unsafe { core::slice::from_raw_parts(data2, len) };
    let mut hash1 = [0u8; SHA256_DIGEST_SIZE];
    let mut hash2 = [0u8; SHA256_DIGEST_SIZE];
    let ctx = if ctx.is_null() {
        None
    } else {
        Some(unsafe { &*ctx })
    };
    sha256_finup_2x(ctx, data1, data2, &mut hash1, &mut hash2);
    unsafe {
        core::ptr::copy_nonoverlapping(hash1.as_ptr(), out1, SHA256_DIGEST_SIZE);
        core::ptr::copy_nonoverlapping(hash2.as_ptr(), out2, SHA256_DIGEST_SIZE);
    }
}

pub extern "C" fn sha256_finup_2x_is_optimized_raw() -> bool {
    sha256_finup_2x_is_optimized()
}

pub unsafe extern "C" fn hmac_sha224_preparekey_raw(
    key: *mut HmacSha224Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha224_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn hmac_sha256_preparekey_raw(
    key: *mut HmacSha256Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha256_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn __hmac_sha256_init_raw(
    ctx: *mut HmacSha256CtxInner,
    key: *const HmacSha256KeyInner,
) {
    if ctx.is_null() || key.is_null() {
        return;
    }
    unsafe { __hmac_sha256_init(&mut *ctx, &*key) };
}

pub unsafe extern "C" fn hmac_sha224_init_usingrawkey_raw(
    ctx: *mut HmacSha224Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha224_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_sha256_init_usingrawkey_raw(
    ctx: *mut HmacSha256Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha256_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_sha224_final_raw(ctx: *mut HmacSha224Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA224_DIGEST_SIZE]) };
    unsafe { hmac_sha224_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_sha256_final_raw(ctx: *mut HmacSha256Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA256_DIGEST_SIZE]) };
    unsafe { hmac_sha256_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_sha224_raw(
    key: *const HmacSha224Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_sha224(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA224_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha256_raw(
    key: *const HmacSha256Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_sha256(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA256_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha224_usingrawkey_raw(
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
    let digest = hmac_sha224_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA224_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha256_usingrawkey_raw(
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
    let digest = hmac_sha256_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA256_DIGEST_SIZE) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; SHA256_DIGEST_SIZE])> {
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
                if digest.len() == SHA256_DIGEST_SIZE {
                    let mut array = [0u8; SHA256_DIGEST_SIZE];
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
    fn sha256_matches_linux_kunit_vectors_hmac_and_finup_2x() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/sha256.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha256-testvecs.h"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha256_kunit.c"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));
        assert!(source.contains("static const u32 sha256_K[64]"));
        assert!(source.contains("void sha256_finup_2x(const struct sha256_ctx *ctx"));
        assert!(source.contains("void hmac_sha256_preparekey(struct hmac_sha256_key *key"));
        assert!(source.contains("EXPORT_SYMBOL(sha256_final);"));
        assert!(vectors.contains("hash_testvec_consolidated[SHA256_DIGEST_SIZE]"));
        assert!(vectors.contains("hmac_testvec_consolidated[SHA256_DIGEST_SIZE]"));
        assert!(kunit.contains("KUNIT_CASE(test_sha256_finup_2x_defaultctx)"));
        assert!(template.contains("HMAC_USINGRAWKEY(raw_key, key_len, test_buf, data_len, mac);"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            assert_eq!(sha256(&data), expected, "data_len={len}");
        }

        assert_eq!(
            sha224(&[]),
            [
                0xd1, 0x4a, 0x02, 0x8c, 0x2a, 0x3a, 0x2b, 0xc9, 0x47, 0x61, 0x02, 0xbb, 0x28, 0x82,
                0x34, 0xc4, 0x15, 0xa2, 0xb0, 0x1f, 0x82, 0x8e, 0xa6, 0x2a, 0xc5, 0xb3, 0xe4, 0x2f,
            ]
        );

        let test_buf = rand_bytes_seeded_from_len(4096);
        let mut consolidated_ctx = Sha256Ctx::default();
        for len in 0..=4096 {
            let digest = sha256(&test_buf[..len]);
            sha256_update(&mut consolidated_ctx, &digest);
        }
        let mut consolidated = [0u8; SHA256_DIGEST_SIZE];
        sha256_final(&mut consolidated_ctx, &mut consolidated);
        assert_eq!(
            consolidated,
            parse_named_array(vectors, "hash_testvec_consolidated")
        );

        let mut raw_key = rand_bytes_seeded_from_len(32);
        let mut key = HmacSha256Key::default();
        hmac_sha256_preparekey(&mut key, &raw_key);
        let mut hmac_ctx = HmacSha256Ctx::default();
        hmac_sha256_init(&mut hmac_ctx, &key);
        for data_len in 0..=4096 {
            let key_len = data_len % 293;
            hmac_sha256_update(&mut hmac_ctx, &test_buf[..data_len]);
            raw_key = rand_bytes_seeded_from_len(key_len);
            let mac = hmac_sha256_usingrawkey(&raw_key, &test_buf[..data_len]);
            hmac_sha256_update(&mut hmac_ctx, &mac);
            hmac_sha256_preparekey(&mut key, &raw_key);
            assert_eq!(hmac_sha256(&key, &test_buf[..data_len]), mac);
        }
        let mut mac = [0u8; SHA256_DIGEST_SIZE];
        hmac_sha256_final(&mut hmac_ctx, &mut mac);
        assert_eq!(mac, parse_named_array(vectors, "hmac_testvec_consolidated"));
        assert_eq!(hmac_ctx, HmacSha256Ctx::default());

        let salt = rand_bytes_seeded_from_len(17);
        let data1 = rand_bytes_seeded_from_len(257);
        let data2 = rand_bytes_seeded_from_len(258);
        let mut ctx = Sha256Ctx::default();
        sha256_update(&mut ctx, &salt);
        let original_ctx = ctx;
        let mut out1 = [0u8; SHA256_DIGEST_SIZE];
        let mut out2 = [0u8; SHA256_DIGEST_SIZE];
        sha256_finup_2x(
            Some(&ctx),
            &data1,
            &data2[..data1.len()],
            &mut out1,
            &mut out2,
        );
        assert_eq!(ctx, original_ctx);

        let mut expected_ctx = original_ctx;
        sha256_update(&mut expected_ctx, &data1);
        let mut expected1 = [0u8; SHA256_DIGEST_SIZE];
        sha256_final(&mut expected_ctx, &mut expected1);
        expected_ctx = original_ctx;
        sha256_update(&mut expected_ctx, &data2[..data1.len()]);
        let mut expected2 = [0u8; SHA256_DIGEST_SIZE];
        sha256_final(&mut expected_ctx, &mut expected2);
        assert_eq!(out1, expected1);
        assert_eq!(out2, expected2);

        let mut default1 = [0u8; SHA256_DIGEST_SIZE];
        let mut default2 = [0u8; SHA256_DIGEST_SIZE];
        let mut init1 = [0u8; SHA256_DIGEST_SIZE];
        let mut init2 = [0u8; SHA256_DIGEST_SIZE];
        let empty_ctx = Sha256Ctx::default();
        sha256_finup_2x(
            None,
            &data1,
            &data2[..data1.len()],
            &mut default1,
            &mut default2,
        );
        sha256_finup_2x(
            Some(&empty_ctx),
            &data1,
            &data2[..data1.len()],
            &mut init1,
            &mut init2,
        );
        assert_eq!(default1, init1);
        assert_eq!(default2, init2);
        assert!(!sha256_finup_2x_is_optimized());

        let mut hmac224_key = HmacSha224Key::default();
        hmac_sha224_preparekey(&mut hmac224_key, b"key");
        assert_eq!(
            hmac_sha224(&hmac224_key, b"The quick brown fox jumps over the lazy dog"),
            hmac_sha224_usingrawkey(b"key", b"The quick brown fox jumps over the lazy dog")
        );
    }
}
