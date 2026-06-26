//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/sha512.c
//! test-origin: linux:vendor/linux/lib/crypto/sha512.c
//! SHA-384, SHA-512, HMAC-SHA384, and HMAC-SHA512 helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const SHA384_DIGEST_SIZE: usize = 48;
pub const SHA384_BLOCK_SIZE: usize = 128;
pub const SHA512_DIGEST_SIZE: usize = 64;
pub const SHA512_BLOCK_SIZE: usize = 128;
pub const SHA512_STATE_WORDS: usize = 8;
const HMAC_IPAD_VALUE: u8 = 0x36;
const HMAC_OPAD_VALUE: u8 = 0x5c;

pub const SHA384_H0: u64 = 0xcbbb_9d5d_c105_9ed8;
pub const SHA384_H1: u64 = 0x629a_292a_367c_d507;
pub const SHA384_H2: u64 = 0x9159_015a_3070_dd17;
pub const SHA384_H3: u64 = 0x152f_ecd8_f70e_5939;
pub const SHA384_H4: u64 = 0x6733_2667_ffc0_0b31;
pub const SHA384_H5: u64 = 0x8eb4_4a87_6858_1511;
pub const SHA384_H6: u64 = 0xdb0c_2e0d_64f9_8fa7;
pub const SHA384_H7: u64 = 0x47b5_481d_befa_4fa4;

pub const SHA512_H0: u64 = 0x6a09_e667_f3bc_c908;
pub const SHA512_H1: u64 = 0xbb67_ae85_84ca_a73b;
pub const SHA512_H2: u64 = 0x3c6e_f372_fe94_f82b;
pub const SHA512_H3: u64 = 0xa54f_f53a_5f1d_36f1;
pub const SHA512_H4: u64 = 0x510e_527f_ade6_82d1;
pub const SHA512_H5: u64 = 0x9b05_688c_2b3e_6c1f;
pub const SHA512_H6: u64 = 0x1f83_d9ab_fb41_bd6b;
pub const SHA512_H7: u64 = 0x5be0_cd19_137e_2179;

const SHA512_K: [u64; 80] = [
    0x428a_2f98_d728_ae22,
    0x7137_4491_23ef_65cd,
    0xb5c0_fbcf_ec4d_3b2f,
    0xe9b5_dba5_8189_dbbc,
    0x3956_c25b_f348_b538,
    0x59f1_11f1_b605_d019,
    0x923f_82a4_af19_4f9b,
    0xab1c_5ed5_da6d_8118,
    0xd807_aa98_a303_0242,
    0x1283_5b01_4570_6fbe,
    0x2431_85be_4ee4_b28c,
    0x550c_7dc3_d5ff_b4e2,
    0x72be_5d74_f27b_896f,
    0x80de_b1fe_3b16_96b1,
    0x9bdc_06a7_25c7_1235,
    0xc19b_f174_cf69_2694,
    0xe49b_69c1_9ef1_4ad2,
    0xefbe_4786_384f_25e3,
    0x0fc1_9dc6_8b8c_d5b5,
    0x240c_a1cc_77ac_9c65,
    0x2de9_2c6f_592b_0275,
    0x4a74_84aa_6ea6_e483,
    0x5cb0_a9dc_bd41_fbd4,
    0x76f9_88da_8311_53b5,
    0x983e_5152_ee66_dfab,
    0xa831_c66d_2db4_3210,
    0xb003_27c8_98fb_213f,
    0xbf59_7fc7_beef_0ee4,
    0xc6e0_0bf3_3da8_8fc2,
    0xd5a7_9147_930a_a725,
    0x06ca_6351_e003_826f,
    0x1429_2967_0a0e_6e70,
    0x27b7_0a85_46d2_2ffc,
    0x2e1b_2138_5c26_c926,
    0x4d2c_6dfc_5ac4_2aed,
    0x5338_0d13_9d95_b3df,
    0x650a_7354_8baf_63de,
    0x766a_0abb_3c77_b2a8,
    0x81c2_c92e_47ed_aee6,
    0x9272_2c85_1482_353b,
    0xa2bf_e8a1_4cf1_0364,
    0xa81a_664b_bc42_3001,
    0xc24b_8b70_d0f8_9791,
    0xc76c_51a3_0654_be30,
    0xd192_e819_d6ef_5218,
    0xd699_0624_5565_a910,
    0xf40e_3585_5771_202a,
    0x106a_a070_32bb_d1b8,
    0x19a4_c116_b8d2_d0c8,
    0x1e37_6c08_5141_ab53,
    0x2748_774c_df8e_eb99,
    0x34b0_bcb5_e19b_48a8,
    0x391c_0cb3_c5c9_5a63,
    0x4ed8_aa4a_e341_8acb,
    0x5b9c_ca4f_7763_e373,
    0x682e_6ff3_d6b2_b8a3,
    0x748f_82ee_5def_b2fc,
    0x78a5_636f_4317_2f60,
    0x84c8_7814_a1f0_ab72,
    0x8cc7_0208_1a64_39ec,
    0x90be_fffa_2363_1e28,
    0xa450_6ceb_de82_bde9,
    0xbef9_a3f7_b2c6_7915,
    0xc671_78f2_e372_532b,
    0xca27_3ece_ea26_619c,
    0xd186_b8c7_21c0_c207,
    0xeada_7dd6_cde0_eb1e,
    0xf57d_4f7f_ee6e_d178,
    0x06f0_67aa_7217_6fba,
    0x0a63_7dc5_a2c8_98a6,
    0x113f_9804_bef9_0dae,
    0x1b71_0b35_131c_471b,
    0x28db_77f5_2304_7d84,
    0x32ca_ab7b_40c7_2493,
    0x3c9e_be0a_15c9_bebc,
    0x431d_67c4_9c10_0d4c,
    0x4cc5_d4be_cb3e_42b6,
    0x597f_299c_fc65_7e2a,
    0x5fcb_6fab_3ad6_faec,
    0x6c44_198c_4a47_5817,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sha512BlockState {
    pub h: [u64; SHA512_STATE_WORDS],
}

const SHA384_IV: Sha512BlockState = Sha512BlockState {
    h: [
        SHA384_H0, SHA384_H1, SHA384_H2, SHA384_H3, SHA384_H4, SHA384_H5, SHA384_H6, SHA384_H7,
    ],
};
const SHA512_IV: Sha512BlockState = Sha512BlockState {
    h: [
        SHA512_H0, SHA512_H1, SHA512_H2, SHA512_H3, SHA512_H4, SHA512_H5, SHA512_H6, SHA512_H7,
    ],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha512InnerCtx {
    pub state: Sha512BlockState,
    pub bytecount_lo: u64,
    pub bytecount_hi: u64,
    pub buf: [u8; SHA512_BLOCK_SIZE],
}

impl Default for Sha512InnerCtx {
    fn default() -> Self {
        Self {
            state: Sha512BlockState::default(),
            bytecount_lo: 0,
            bytecount_hi: 0,
            buf: [0; SHA512_BLOCK_SIZE],
        }
    }
}

impl Sha512InnerCtx {
    const fn new(state: Sha512BlockState, initial_bytecount: u64) -> Self {
        Self {
            state,
            bytecount_lo: initial_bytecount,
            bytecount_hi: 0,
            buf: [0; SHA512_BLOCK_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha384Ctx {
    pub ctx: Sha512InnerCtx,
}

impl Default for Sha384Ctx {
    fn default() -> Self {
        Self {
            ctx: Sha512InnerCtx::new(SHA384_IV, 0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sha512Ctx {
    pub ctx: Sha512InnerCtx,
}

impl Default for Sha512Ctx {
    fn default() -> Self {
        Self {
            ctx: Sha512InnerCtx::new(SHA512_IV, 0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha512KeyInner {
    pub istate: Sha512BlockState,
    pub ostate: Sha512BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha512CtxInner {
    pub sha_ctx: Sha512InnerCtx,
    pub ostate: Sha512BlockState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha384Key {
    pub key: HmacSha512KeyInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha384Ctx {
    pub ctx: HmacSha512CtxInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha512Key {
    pub key: HmacSha512KeyInner,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HmacSha512Ctx {
    pub ctx: HmacSha512CtxInner,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sha384_init", sha384_init_raw as usize, true);
    export_symbol_once("sha512_init", sha512_init_raw as usize, true);
    export_symbol_once("__sha512_update", __sha512_update_raw as usize, true);
    export_symbol_once("sha384_final", sha384_final_raw as usize, true);
    export_symbol_once("sha512_final", sha512_final_raw as usize, true);
    export_symbol_once("sha384", sha384_raw as usize, true);
    export_symbol_once("sha512", sha512_raw as usize, true);
    export_symbol_once(
        "hmac_sha384_preparekey",
        hmac_sha384_preparekey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha512_preparekey",
        hmac_sha512_preparekey_raw as usize,
        true,
    );
    export_symbol_once("__hmac_sha512_init", __hmac_sha512_init_raw as usize, true);
    export_symbol_once(
        "hmac_sha384_init_usingrawkey",
        hmac_sha384_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha512_init_usingrawkey",
        hmac_sha512_init_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once("hmac_sha384_final", hmac_sha384_final_raw as usize, true);
    export_symbol_once("hmac_sha512_final", hmac_sha512_final_raw as usize, true);
    export_symbol_once("hmac_sha384", hmac_sha384_raw as usize, true);
    export_symbol_once("hmac_sha512", hmac_sha512_raw as usize, true);
    export_symbol_once(
        "hmac_sha384_usingrawkey",
        hmac_sha384_usingrawkey_raw as usize,
        true,
    );
    export_symbol_once(
        "hmac_sha512_usingrawkey",
        hmac_sha512_usingrawkey_raw as usize,
        true,
    );
}

#[inline]
fn ch(x: u64, y: u64, z: u64) -> u64 {
    z ^ (x & (y ^ z))
}

#[inline]
fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & y) | (z & (x | y))
}

#[inline]
fn e0(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}

#[inline]
fn e1(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}

#[inline]
fn s0(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}

#[inline]
fn s1(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}

fn sha512_block_generic(state: &mut Sha512BlockState, data: &[u8; SHA512_BLOCK_SIZE]) {
    let mut w = [0u64; 80];
    for i in 0..16 {
        let offset = i * 8;
        w[i] = u64::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
    }
    for i in 16..80 {
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

    for i in 0..80 {
        let t1 = h
            .wrapping_add(e1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(SHA512_K[i])
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

fn sha512_blocks(state: &mut Sha512BlockState, data: &[u8]) {
    for block in data.chunks_exact(SHA512_BLOCK_SIZE) {
        let block = <&[u8; SHA512_BLOCK_SIZE]>::try_from(block).unwrap();
        sha512_block_generic(state, block);
    }
}

fn __sha512_init(ctx: &mut Sha512InnerCtx, iv: &Sha512BlockState, initial_bytecount: u64) {
    ctx.state = *iv;
    ctx.bytecount_lo = initial_bytecount;
    ctx.bytecount_hi = 0;
    ctx.buf = [0; SHA512_BLOCK_SIZE];
}

pub fn sha384_init(ctx: &mut Sha384Ctx) {
    __sha512_init(&mut ctx.ctx, &SHA384_IV, 0);
}

pub fn sha512_init(ctx: &mut Sha512Ctx) {
    __sha512_init(&mut ctx.ctx, &SHA512_IV, 0);
}

pub fn __sha512_update(ctx: &mut Sha512InnerCtx, mut data: &[u8]) {
    let mut partial = (ctx.bytecount_lo as usize) % SHA512_BLOCK_SIZE;
    let (lo, overflowed) = ctx.bytecount_lo.overflowing_add(data.len() as u64);
    ctx.bytecount_lo = lo;
    if overflowed {
        ctx.bytecount_hi = ctx.bytecount_hi.wrapping_add(1);
    }

    if partial + data.len() >= SHA512_BLOCK_SIZE {
        if partial != 0 {
            let take = SHA512_BLOCK_SIZE - partial;
            ctx.buf[partial..partial + take].copy_from_slice(&data[..take]);
            sha512_blocks(&mut ctx.state, &ctx.buf);
            data = &data[take..];
        }

        let nblocks_len = data.len() / SHA512_BLOCK_SIZE * SHA512_BLOCK_SIZE;
        if nblocks_len != 0 {
            sha512_blocks(&mut ctx.state, &data[..nblocks_len]);
            data = &data[nblocks_len..];
        }
        partial = 0;
    }
    if !data.is_empty() {
        ctx.buf[partial..partial + data.len()].copy_from_slice(data);
    }
}

pub fn sha384_update(ctx: &mut Sha384Ctx, data: &[u8]) {
    __sha512_update(&mut ctx.ctx, data);
}

pub fn sha512_update(ctx: &mut Sha512Ctx, data: &[u8]) {
    __sha512_update(&mut ctx.ctx, data);
}

fn __sha512_final_nozero(ctx: &mut Sha512InnerCtx, out: &mut [u8], digest_size: usize) {
    let bitcount_hi = (ctx.bytecount_hi << 3) | (ctx.bytecount_lo >> 61);
    let bitcount_lo = ctx.bytecount_lo << 3;
    let mut partial = (ctx.bytecount_lo as usize) % SHA512_BLOCK_SIZE;

    ctx.buf[partial] = 0x80;
    partial += 1;
    if partial > SHA512_BLOCK_SIZE - 16 {
        ctx.buf[partial..].fill(0);
        sha512_blocks(&mut ctx.state, &ctx.buf);
        partial = 0;
    }
    ctx.buf[partial..SHA512_BLOCK_SIZE - 16].fill(0);
    ctx.buf[SHA512_BLOCK_SIZE - 16..SHA512_BLOCK_SIZE - 8]
        .copy_from_slice(&bitcount_hi.to_be_bytes());
    ctx.buf[SHA512_BLOCK_SIZE - 8..].copy_from_slice(&bitcount_lo.to_be_bytes());
    sha512_blocks(&mut ctx.state, &ctx.buf);

    for i in (0..digest_size).step_by(8) {
        out[i..i + 8].copy_from_slice(&ctx.state.h[i / 8].to_be_bytes());
    }
}

pub fn sha384_final(ctx: &mut Sha384Ctx, out: &mut [u8; SHA384_DIGEST_SIZE]) {
    __sha512_final_nozero(&mut ctx.ctx, out, SHA384_DIGEST_SIZE);
    ctx.ctx = Sha512InnerCtx::default();
}

pub fn sha512_final(ctx: &mut Sha512Ctx, out: &mut [u8; SHA512_DIGEST_SIZE]) {
    __sha512_final_nozero(&mut ctx.ctx, out, SHA512_DIGEST_SIZE);
    ctx.ctx = Sha512InnerCtx::default();
}

pub fn sha384(data: &[u8]) -> [u8; SHA384_DIGEST_SIZE] {
    let mut ctx = Sha384Ctx::default();
    sha384_update(&mut ctx, data);
    let mut out = [0u8; SHA384_DIGEST_SIZE];
    sha384_final(&mut ctx, &mut out);
    out
}

pub fn sha512(data: &[u8]) -> [u8; SHA512_DIGEST_SIZE] {
    let mut ctx = Sha512Ctx::default();
    sha512_update(&mut ctx, data);
    let mut out = [0u8; SHA512_DIGEST_SIZE];
    sha512_final(&mut ctx, &mut out);
    out
}

fn __hmac_sha512_preparekey(
    istate: &mut Sha512BlockState,
    ostate: &mut Sha512BlockState,
    raw_key: &[u8],
    iv: &Sha512BlockState,
    digest_size: usize,
) {
    let mut derived_key = [0u8; SHA512_BLOCK_SIZE];
    if raw_key.len() > SHA512_BLOCK_SIZE {
        if digest_size == SHA384_DIGEST_SIZE {
            derived_key[..SHA384_DIGEST_SIZE].copy_from_slice(&sha384(raw_key));
        } else {
            derived_key[..SHA512_DIGEST_SIZE].copy_from_slice(&sha512(raw_key));
        }
    } else {
        derived_key[..raw_key.len()].copy_from_slice(raw_key);
    }

    let mut ipad = derived_key;
    for byte in &mut ipad {
        *byte ^= HMAC_IPAD_VALUE;
    }
    *istate = *iv;
    sha512_blocks(istate, &ipad);

    let mut opad = derived_key;
    for byte in &mut opad {
        *byte ^= HMAC_OPAD_VALUE;
    }
    *ostate = *iv;
    sha512_blocks(ostate, &opad);
}

pub fn hmac_sha384_preparekey(key: &mut HmacSha384Key, raw_key: &[u8]) {
    __hmac_sha512_preparekey(
        &mut key.key.istate,
        &mut key.key.ostate,
        raw_key,
        &SHA384_IV,
        SHA384_DIGEST_SIZE,
    );
}

pub fn hmac_sha512_preparekey(key: &mut HmacSha512Key, raw_key: &[u8]) {
    __hmac_sha512_preparekey(
        &mut key.key.istate,
        &mut key.key.ostate,
        raw_key,
        &SHA512_IV,
        SHA512_DIGEST_SIZE,
    );
}

pub fn __hmac_sha512_init(ctx: &mut HmacSha512CtxInner, key: &HmacSha512KeyInner) {
    __sha512_init(&mut ctx.sha_ctx, &key.istate, SHA512_BLOCK_SIZE as u64);
    ctx.ostate = key.ostate;
}

pub fn hmac_sha384_init(ctx: &mut HmacSha384Ctx, key: &HmacSha384Key) {
    __hmac_sha512_init(&mut ctx.ctx, &key.key);
}

pub fn hmac_sha512_init(ctx: &mut HmacSha512Ctx, key: &HmacSha512Key) {
    __hmac_sha512_init(&mut ctx.ctx, &key.key);
}

pub fn hmac_sha384_init_usingrawkey(ctx: &mut HmacSha384Ctx, raw_key: &[u8]) {
    __hmac_sha512_preparekey(
        &mut ctx.ctx.sha_ctx.state,
        &mut ctx.ctx.ostate,
        raw_key,
        &SHA384_IV,
        SHA384_DIGEST_SIZE,
    );
    ctx.ctx.sha_ctx.bytecount_lo = SHA512_BLOCK_SIZE as u64;
    ctx.ctx.sha_ctx.bytecount_hi = 0;
    ctx.ctx.sha_ctx.buf = [0; SHA512_BLOCK_SIZE];
}

pub fn hmac_sha512_init_usingrawkey(ctx: &mut HmacSha512Ctx, raw_key: &[u8]) {
    __hmac_sha512_preparekey(
        &mut ctx.ctx.sha_ctx.state,
        &mut ctx.ctx.ostate,
        raw_key,
        &SHA512_IV,
        SHA512_DIGEST_SIZE,
    );
    ctx.ctx.sha_ctx.bytecount_lo = SHA512_BLOCK_SIZE as u64;
    ctx.ctx.sha_ctx.bytecount_hi = 0;
    ctx.ctx.sha_ctx.buf = [0; SHA512_BLOCK_SIZE];
}

pub fn hmac_sha384_update(ctx: &mut HmacSha384Ctx, data: &[u8]) {
    __sha512_update(&mut ctx.ctx.sha_ctx, data);
}

pub fn hmac_sha512_update(ctx: &mut HmacSha512Ctx, data: &[u8]) {
    __sha512_update(&mut ctx.ctx.sha_ctx, data);
}

fn __hmac_sha512_final(ctx: &mut HmacSha512CtxInner, out: &mut [u8], digest_size: usize) {
    let mut inner = [0u8; SHA512_DIGEST_SIZE];
    __sha512_final_nozero(&mut ctx.sha_ctx, &mut inner[..digest_size], digest_size);

    let mut block = [0u8; SHA512_BLOCK_SIZE];
    block[..digest_size].copy_from_slice(&inner[..digest_size]);
    block[digest_size] = 0x80;
    block[SHA512_BLOCK_SIZE - 4..]
        .copy_from_slice(&(8u32 * (SHA512_BLOCK_SIZE as u32 + digest_size as u32)).to_be_bytes());

    sha512_blocks(&mut ctx.ostate, &block);
    for i in (0..digest_size).step_by(8) {
        out[i..i + 8].copy_from_slice(&ctx.ostate.h[i / 8].to_be_bytes());
    }
    *ctx = HmacSha512CtxInner::default();
}

pub fn hmac_sha384_final(ctx: &mut HmacSha384Ctx, out: &mut [u8; SHA384_DIGEST_SIZE]) {
    __hmac_sha512_final(&mut ctx.ctx, out, SHA384_DIGEST_SIZE);
}

pub fn hmac_sha512_final(ctx: &mut HmacSha512Ctx, out: &mut [u8; SHA512_DIGEST_SIZE]) {
    __hmac_sha512_final(&mut ctx.ctx, out, SHA512_DIGEST_SIZE);
}

pub fn hmac_sha384(key: &HmacSha384Key, data: &[u8]) -> [u8; SHA384_DIGEST_SIZE] {
    let mut ctx = HmacSha384Ctx::default();
    hmac_sha384_init(&mut ctx, key);
    hmac_sha384_update(&mut ctx, data);
    let mut out = [0u8; SHA384_DIGEST_SIZE];
    hmac_sha384_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha512(key: &HmacSha512Key, data: &[u8]) -> [u8; SHA512_DIGEST_SIZE] {
    let mut ctx = HmacSha512Ctx::default();
    hmac_sha512_init(&mut ctx, key);
    hmac_sha512_update(&mut ctx, data);
    let mut out = [0u8; SHA512_DIGEST_SIZE];
    hmac_sha512_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha384_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; SHA384_DIGEST_SIZE] {
    let mut ctx = HmacSha384Ctx::default();
    hmac_sha384_init_usingrawkey(&mut ctx, raw_key);
    hmac_sha384_update(&mut ctx, data);
    let mut out = [0u8; SHA384_DIGEST_SIZE];
    hmac_sha384_final(&mut ctx, &mut out);
    out
}

pub fn hmac_sha512_usingrawkey(raw_key: &[u8], data: &[u8]) -> [u8; SHA512_DIGEST_SIZE] {
    let mut ctx = HmacSha512Ctx::default();
    hmac_sha512_init_usingrawkey(&mut ctx, raw_key);
    hmac_sha512_update(&mut ctx, data);
    let mut out = [0u8; SHA512_DIGEST_SIZE];
    hmac_sha512_final(&mut ctx, &mut out);
    out
}

pub unsafe extern "C" fn sha384_init_raw(ctx: *mut Sha384Ctx) {
    if !ctx.is_null() {
        unsafe { sha384_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn sha512_init_raw(ctx: *mut Sha512Ctx) {
    if !ctx.is_null() {
        unsafe { sha512_init(&mut *ctx) };
    }
}

pub unsafe extern "C" fn __sha512_update_raw(
    ctx: *mut Sha512InnerCtx,
    data: *const u8,
    len: usize,
) {
    if ctx.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    unsafe { __sha512_update(&mut *ctx, data) };
}

pub unsafe extern "C" fn sha384_final_raw(ctx: *mut Sha384Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA384_DIGEST_SIZE]) };
    unsafe { sha384_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sha512_final_raw(ctx: *mut Sha512Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA512_DIGEST_SIZE]) };
    unsafe { sha512_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn sha384_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sha384(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA384_DIGEST_SIZE) };
}

pub unsafe extern "C" fn sha512_raw(data: *const u8, len: usize, out: *mut u8) {
    if out.is_null() || (data.is_null() && len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len) };
    let digest = sha512(data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA512_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha384_preparekey_raw(
    key: *mut HmacSha384Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha384_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn hmac_sha512_preparekey_raw(
    key: *mut HmacSha512Key,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if key.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha512_preparekey(&mut *key, raw_key) };
}

pub unsafe extern "C" fn __hmac_sha512_init_raw(
    ctx: *mut HmacSha512CtxInner,
    key: *const HmacSha512KeyInner,
) {
    if ctx.is_null() || key.is_null() {
        return;
    }
    unsafe { __hmac_sha512_init(&mut *ctx, &*key) };
}

pub unsafe extern "C" fn hmac_sha384_init_usingrawkey_raw(
    ctx: *mut HmacSha384Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha384_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_sha512_init_usingrawkey_raw(
    ctx: *mut HmacSha512Ctx,
    raw_key: *const u8,
    raw_key_len: usize,
) {
    if ctx.is_null() || (raw_key.is_null() && raw_key_len != 0) {
        return;
    }
    let raw_key = unsafe { core::slice::from_raw_parts(raw_key, raw_key_len) };
    unsafe { hmac_sha512_init_usingrawkey(&mut *ctx, raw_key) };
}

pub unsafe extern "C" fn hmac_sha384_final_raw(ctx: *mut HmacSha384Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA384_DIGEST_SIZE]) };
    unsafe { hmac_sha384_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_sha512_final_raw(ctx: *mut HmacSha512Ctx, out: *mut u8) {
    if ctx.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; SHA512_DIGEST_SIZE]) };
    unsafe { hmac_sha512_final(&mut *ctx, out) };
}

pub unsafe extern "C" fn hmac_sha384_raw(
    key: *const HmacSha384Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_sha384(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA384_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha512_raw(
    key: *const HmacSha512Key,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
) {
    if key.is_null() || out.is_null() || (data.is_null() && data_len != 0) {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };
    let digest = unsafe { hmac_sha512(&*key, data) };
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA512_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha384_usingrawkey_raw(
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
    let digest = hmac_sha384_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA384_DIGEST_SIZE) };
}

pub unsafe extern "C" fn hmac_sha512_usingrawkey_raw(
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
    let digest = hmac_sha512_usingrawkey(raw_key, data);
    unsafe { core::ptr::copy_nonoverlapping(digest.as_ptr(), out, SHA512_DIGEST_SIZE) };
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

    fn parse_hash_testvecs(text: &str) -> Vec<(usize, [u8; SHA512_DIGEST_SIZE])> {
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
                if digest.len() == SHA512_DIGEST_SIZE {
                    let mut array = [0u8; SHA512_DIGEST_SIZE];
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
    fn sha512_matches_linux_kunit_vectors_and_hmac() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/sha512.c"
        ));
        let vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/sha512-testvecs.h"
        ));
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/hash-test-template.h"
        ));
        let fips = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/fips.h"
        ));
        assert!(source.contains("static const u64 sha512_K[80]"));
        assert!(source.contains("void __sha512_update(struct __sha512_ctx *ctx"));
        assert!(source.contains("void hmac_sha512_preparekey(struct hmac_sha512_key *key"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sha512_final);"));
        assert!(vectors.contains("hash_testvec_consolidated[SHA512_DIGEST_SIZE]"));
        assert!(vectors.contains("hmac_testvec_consolidated[SHA512_DIGEST_SIZE]"));
        assert!(template.contains("HMAC_USINGRAWKEY(raw_key, key_len, test_buf, data_len, mac);"));
        assert!(fips.contains("fips_test_hmac_sha512_value"));

        for (len, expected) in parse_hash_testvecs(vectors) {
            let data = rand_bytes_seeded_from_len(len);
            assert_eq!(sha512(&data), expected, "data_len={len}");
        }

        assert_eq!(
            sha384(&[]),
            [
                0x38, 0xb0, 0x60, 0xa7, 0x51, 0xac, 0x96, 0x38, 0x4c, 0xd9, 0x32, 0x7e, 0xb1, 0xb1,
                0xe3, 0x6a, 0x21, 0xfd, 0xb7, 0x11, 0x14, 0xbe, 0x07, 0x43, 0x4c, 0x0c, 0xc7, 0xbf,
                0x63, 0xf6, 0xe1, 0xda, 0x27, 0x4e, 0xde, 0xbf, 0xe7, 0x6f, 0x65, 0xfb, 0xd5, 0x1a,
                0xd2, 0xf1, 0x48, 0x98, 0xb9, 0x5b,
            ]
        );

        let test_buf = rand_bytes_seeded_from_len(4096);
        let mut consolidated_ctx = Sha512Ctx::default();
        for len in 0..=4096 {
            let digest = sha512(&test_buf[..len]);
            sha512_update(&mut consolidated_ctx, &digest);
        }
        let mut consolidated = [0u8; SHA512_DIGEST_SIZE];
        sha512_final(&mut consolidated_ctx, &mut consolidated);
        assert_eq!(
            consolidated,
            parse_named_array(vectors, "hash_testvec_consolidated")
        );

        let mut raw_key = rand_bytes_seeded_from_len(32);
        let mut key = HmacSha512Key::default();
        hmac_sha512_preparekey(&mut key, &raw_key);
        let mut hmac_ctx = HmacSha512Ctx::default();
        hmac_sha512_init(&mut hmac_ctx, &key);
        for data_len in 0..=4096 {
            let key_len = data_len % 293;
            hmac_sha512_update(&mut hmac_ctx, &test_buf[..data_len]);
            raw_key = rand_bytes_seeded_from_len(key_len);
            let mac = hmac_sha512_usingrawkey(&raw_key, &test_buf[..data_len]);
            hmac_sha512_update(&mut hmac_ctx, &mac);
            hmac_sha512_preparekey(&mut key, &raw_key);
            assert_eq!(hmac_sha512(&key, &test_buf[..data_len]), mac);
        }
        let mut mac = [0u8; SHA512_DIGEST_SIZE];
        hmac_sha512_final(&mut hmac_ctx, &mut mac);
        assert_eq!(mac, parse_named_array(vectors, "hmac_testvec_consolidated"));
        assert_eq!(hmac_ctx, HmacSha512Ctx::default());

        let mut hmac384_key = HmacSha384Key::default();
        hmac_sha384_preparekey(&mut hmac384_key, b"key");
        assert_eq!(
            hmac_sha384(&hmac384_key, b"The quick brown fox jumps over the lazy dog"),
            hmac_sha384_usingrawkey(b"key", b"The quick brown fox jumps over the lazy dog")
        );
    }
}
