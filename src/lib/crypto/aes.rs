//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/aes.c
//! test-origin: linux:vendor/linux/lib/crypto/aes.c
//! Generic AES key schedule and single-block cipher helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const AES_MIN_KEY_SIZE: usize = 16;
pub const AES_MAX_KEY_SIZE: usize = 32;
pub const AES_KEYSIZE_128: usize = 16;
pub const AES_KEYSIZE_192: usize = 24;
pub const AES_KEYSIZE_256: usize = 32;
pub const AES_BLOCK_SIZE: usize = 16;
pub const AES_MAX_KEYLENGTH: usize = 15 * 16;
pub const AES_MAX_KEYLENGTH_U32: usize = AES_MAX_KEYLENGTH / core::mem::size_of::<u32>();
pub const EINVAL: i32 = -22;

const CRYPTO_AES_SBOX_DATA: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const CRYPTO_AES_INV_SBOX_DATA: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

pub static CRYPTO_AES_SBOX: [u8; 256] = CRYPTO_AES_SBOX_DATA;
pub static CRYPTO_AES_INV_SBOX: [u8; 256] = CRYPTO_AES_INV_SBOX_DATA;

#[inline]
const fn xtime_byte(x: u8) -> u8 {
    (x << 1) ^ (((x >> 7) & 1) * 0x1b)
}

#[inline]
const fn gf_mul_byte(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    let mut i = 0;
    while i < 8 {
        if b & 1 != 0 {
            p ^= a;
        }
        a = xtime_byte(a);
        b >>= 1;
        i += 1;
    }
    p
}

const fn pack_le_bytes(b0: u8, b1: u8, b2: u8, b3: u8) -> u32 {
    (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16) | ((b3 as u32) << 24)
}

const fn aes_enc_tab_entry(index: usize) -> u32 {
    let s = CRYPTO_AES_SBOX_DATA[index];
    pack_le_bytes(gf_mul_byte(s, 2), s, s, gf_mul_byte(s, 3))
}

const fn aes_dec_tab_entry(index: usize) -> u32 {
    let s = CRYPTO_AES_INV_SBOX_DATA[index];
    pack_le_bytes(
        gf_mul_byte(s, 14),
        gf_mul_byte(s, 9),
        gf_mul_byte(s, 13),
        gf_mul_byte(s, 11),
    )
}

const fn make_aes_enc_tab() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = aes_enc_tab_entry(i);
        i += 1;
    }
    table
}

const fn make_aes_dec_tab() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = aes_dec_tab_entry(i);
        i += 1;
    }
    table
}

/// Linux `aes_enc_tab`: MixColumn([SubByte(i), 0, 0, 0]).
const AES_ENC_TAB_DATA: [u32; 256] = make_aes_enc_tab();

/// Linux `aes_dec_tab`: InvMixColumn([InvSubByte(i), 0, 0, 0]).
const AES_DEC_TAB_DATA: [u32; 256] = make_aes_dec_tab();

pub static AES_ENC_TAB: [u32; 256] = AES_ENC_TAB_DATA;
pub static AES_DEC_TAB: [u32; 256] = AES_DEC_TAB_DATA;

const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesEncKey {
    pub len: u32,
    pub nrounds: u32,
    pub padding: [u32; 2],
    pub round_keys: [u8; AES_MAX_KEYLENGTH],
}

impl Default for AesEncKey {
    fn default() -> Self {
        Self {
            len: 0,
            nrounds: 0,
            padding: [0; 2],
            round_keys: [0; AES_MAX_KEYLENGTH],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesKey {
    pub enc_key: AesEncKey,
    pub inv_round_keys: [u32; AES_MAX_KEYLENGTH_U32],
}

impl Default for AesKey {
    fn default() -> Self {
        Self {
            enc_key: AesEncKey::default(),
            inv_round_keys: [0; AES_MAX_KEYLENGTH_U32],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoAesCtx {
    pub key_enc: [u32; AES_MAX_KEYLENGTH_U32],
    pub key_dec: [u32; AES_MAX_KEYLENGTH_U32],
    pub key_length: u32,
}

impl Default for CryptoAesCtx {
    fn default() -> Self {
        Self {
            key_enc: [0; AES_MAX_KEYLENGTH_U32],
            key_dec: [0; AES_MAX_KEYLENGTH_U32],
            key_length: 0,
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("crypto_aes_sbox", CRYPTO_AES_SBOX.as_ptr() as usize, false);
    export_symbol_once(
        "crypto_aes_inv_sbox",
        CRYPTO_AES_INV_SBOX.as_ptr() as usize,
        false,
    );
    export_symbol_once("aes_enc_tab", AES_ENC_TAB.as_ptr() as usize, false);
    export_symbol_once("aes_dec_tab", AES_DEC_TAB.as_ptr() as usize, false);
    export_symbol_once("aes_expandkey", aes_expandkey_raw as usize, false);
    export_symbol_once("aes_preparekey", aes_preparekey_raw as usize, false);
    export_symbol_once("aes_prepareenckey", aes_prepareenckey_raw as usize, false);
    export_symbol_once("aes_encrypt", aes_encrypt_raw as usize, false);
    export_symbol_once("aes_decrypt", aes_decrypt_raw as usize, false);
}

pub const fn aes_check_keylen(key_len: usize) -> i32 {
    match key_len {
        AES_KEYSIZE_128 | AES_KEYSIZE_192 | AES_KEYSIZE_256 => 0,
        _ => EINVAL,
    }
}

fn expand_round_keys(out: &mut [u8; AES_MAX_KEYLENGTH], in_key: &[u8]) -> i32 {
    if aes_check_keylen(in_key.len()) != 0 {
        return EINVAL;
    }
    out.fill(0);
    let nk = in_key.len() / 4;
    let nr = 6 + nk;
    let words = 4 * (nr + 1);
    out[..in_key.len()].copy_from_slice(in_key);

    let mut temp = [0u8; 4];
    for i in nk..words {
        temp.copy_from_slice(&out[(i - 1) * 4..i * 4]);
        if i % nk == 0 {
            temp.rotate_left(1);
            for byte in &mut temp {
                *byte = CRYPTO_AES_SBOX[*byte as usize];
            }
            temp[0] ^= RCON[i / nk - 1];
        } else if nk > 6 && i % nk == 4 {
            for byte in &mut temp {
                *byte = CRYPTO_AES_SBOX[*byte as usize];
            }
        }
        for j in 0..4 {
            out[i * 4 + j] = out[(i - nk) * 4 + j] ^ temp[j];
        }
    }
    0
}

fn round_key_words(round_keys: &[u8; AES_MAX_KEYLENGTH]) -> [u32; AES_MAX_KEYLENGTH_U32] {
    let mut words = [0u32; AES_MAX_KEYLENGTH_U32];
    for (i, chunk) in round_keys.chunks_exact(4).enumerate() {
        words[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    words
}

fn inv_mix_columns_word(x: u32) -> u32 {
    let [b0, b1, b2, b3] = x.to_le_bytes();
    pack_le_bytes(
        gf_mul_byte(b0, 14) ^ gf_mul_byte(b1, 11) ^ gf_mul_byte(b2, 13) ^ gf_mul_byte(b3, 9),
        gf_mul_byte(b0, 9) ^ gf_mul_byte(b1, 14) ^ gf_mul_byte(b2, 11) ^ gf_mul_byte(b3, 13),
        gf_mul_byte(b0, 13) ^ gf_mul_byte(b1, 9) ^ gf_mul_byte(b2, 14) ^ gf_mul_byte(b3, 11),
        gf_mul_byte(b0, 11) ^ gf_mul_byte(b1, 13) ^ gf_mul_byte(b2, 9) ^ gf_mul_byte(b3, 14),
    )
}

fn inverse_round_keys(
    enc_words: &[u32; AES_MAX_KEYLENGTH_U32],
    key_len: usize,
) -> [u32; AES_MAX_KEYLENGTH_U32] {
    let mut inv = [0u32; AES_MAX_KEYLENGTH_U32];
    inv[0] = enc_words[key_len + 24];
    inv[1] = enc_words[key_len + 25];
    inv[2] = enc_words[key_len + 26];
    inv[3] = enc_words[key_len + 27];

    let mut i = 4;
    let mut j = key_len + 20;
    while j > 0 {
        inv[i] = inv_mix_columns_word(enc_words[j]);
        inv[i + 1] = inv_mix_columns_word(enc_words[j + 1]);
        inv[i + 2] = inv_mix_columns_word(enc_words[j + 2]);
        inv[i + 3] = inv_mix_columns_word(enc_words[j + 3]);
        i += 4;
        j -= 4;
    }

    inv[i] = enc_words[0];
    inv[i + 1] = enc_words[1];
    inv[i + 2] = enc_words[2];
    inv[i + 3] = enc_words[3];
    inv
}

pub fn aes_prepareenckey(key: &mut AesEncKey, in_key: &[u8]) -> i32 {
    if aes_check_keylen(in_key.len()) != 0 {
        return EINVAL;
    }
    key.len = in_key.len() as u32;
    key.nrounds = (6 + in_key.len() / 4) as u32;
    expand_round_keys(&mut key.round_keys, in_key)
}

pub fn aes_preparekey(key: &mut AesKey, in_key: &[u8]) -> i32 {
    let ret = aes_prepareenckey(&mut key.enc_key, in_key);
    if ret == 0 {
        let enc_words = round_key_words(&key.enc_key.round_keys);
        key.inv_round_keys = inverse_round_keys(&enc_words, in_key.len());
    }
    ret
}

pub fn aes_expandkey(ctx: &mut CryptoAesCtx, in_key: &[u8]) -> i32 {
    let mut key = AesEncKey::default();
    let ret = aes_prepareenckey(&mut key, in_key);
    if ret != 0 {
        return ret;
    }
    ctx.key_length = in_key.len() as u32;
    ctx.key_enc = round_key_words(&key.round_keys);
    ctx.key_dec = inverse_round_keys(&ctx.key_enc, in_key.len());
    ret
}

fn add_round_key(state: &mut [u8; AES_BLOCK_SIZE], round_key: &[u8]) {
    for i in 0..AES_BLOCK_SIZE {
        state[i] ^= round_key[i];
    }
}

fn sub_bytes(state: &mut [u8; AES_BLOCK_SIZE]) {
    for byte in state {
        *byte = CRYPTO_AES_SBOX[*byte as usize];
    }
}

fn inv_sub_bytes(state: &mut [u8; AES_BLOCK_SIZE]) {
    for byte in state {
        *byte = CRYPTO_AES_INV_SBOX[*byte as usize];
    }
}

fn shift_rows(state: &mut [u8; AES_BLOCK_SIZE]) {
    let s = *state;
    state[1] = s[5];
    state[5] = s[9];
    state[9] = s[13];
    state[13] = s[1];
    state[2] = s[10];
    state[6] = s[14];
    state[10] = s[2];
    state[14] = s[6];
    state[3] = s[15];
    state[7] = s[3];
    state[11] = s[7];
    state[15] = s[11];
}

fn inv_shift_rows(state: &mut [u8; AES_BLOCK_SIZE]) {
    let s = *state;
    state[1] = s[13];
    state[5] = s[1];
    state[9] = s[5];
    state[13] = s[9];
    state[2] = s[10];
    state[6] = s[14];
    state[10] = s[2];
    state[14] = s[6];
    state[3] = s[7];
    state[7] = s[11];
    state[11] = s[15];
    state[15] = s[3];
}

fn mix_columns(state: &mut [u8; AES_BLOCK_SIZE]) {
    for col in 0..4 {
        let i = col * 4;
        let a0 = state[i];
        let a1 = state[i + 1];
        let a2 = state[i + 2];
        let a3 = state[i + 3];
        state[i] = gf_mul_byte(a0, 2) ^ gf_mul_byte(a1, 3) ^ a2 ^ a3;
        state[i + 1] = a0 ^ gf_mul_byte(a1, 2) ^ gf_mul_byte(a2, 3) ^ a3;
        state[i + 2] = a0 ^ a1 ^ gf_mul_byte(a2, 2) ^ gf_mul_byte(a3, 3);
        state[i + 3] = gf_mul_byte(a0, 3) ^ a1 ^ a2 ^ gf_mul_byte(a3, 2);
    }
}

fn inv_mix_columns(state: &mut [u8; AES_BLOCK_SIZE]) {
    for col in 0..4 {
        let i = col * 4;
        let a0 = state[i];
        let a1 = state[i + 1];
        let a2 = state[i + 2];
        let a3 = state[i + 3];
        state[i] =
            gf_mul_byte(a0, 14) ^ gf_mul_byte(a1, 11) ^ gf_mul_byte(a2, 13) ^ gf_mul_byte(a3, 9);
        state[i + 1] =
            gf_mul_byte(a0, 9) ^ gf_mul_byte(a1, 14) ^ gf_mul_byte(a2, 11) ^ gf_mul_byte(a3, 13);
        state[i + 2] =
            gf_mul_byte(a0, 13) ^ gf_mul_byte(a1, 9) ^ gf_mul_byte(a2, 14) ^ gf_mul_byte(a3, 11);
        state[i + 3] =
            gf_mul_byte(a0, 11) ^ gf_mul_byte(a1, 13) ^ gf_mul_byte(a2, 9) ^ gf_mul_byte(a3, 14);
    }
}

pub fn aes_encrypt(key: &AesEncKey, out: &mut [u8; AES_BLOCK_SIZE], input: &[u8; AES_BLOCK_SIZE]) {
    let nr = key.nrounds as usize;
    let mut state = *input;
    add_round_key(&mut state, &key.round_keys[..AES_BLOCK_SIZE]);
    for round in 1..nr {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(
            &mut state,
            &key.round_keys[round * AES_BLOCK_SIZE..(round + 1) * AES_BLOCK_SIZE],
        );
    }
    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(
        &mut state,
        &key.round_keys[nr * AES_BLOCK_SIZE..(nr + 1) * AES_BLOCK_SIZE],
    );
    *out = state;
}

pub fn aes_decrypt(key: &AesKey, out: &mut [u8; AES_BLOCK_SIZE], input: &[u8; AES_BLOCK_SIZE]) {
    let enc = &key.enc_key;
    let nr = enc.nrounds as usize;
    let mut state = *input;
    add_round_key(
        &mut state,
        &enc.round_keys[nr * AES_BLOCK_SIZE..(nr + 1) * AES_BLOCK_SIZE],
    );
    for round in (1..nr).rev() {
        inv_shift_rows(&mut state);
        inv_sub_bytes(&mut state);
        add_round_key(
            &mut state,
            &enc.round_keys[round * AES_BLOCK_SIZE..(round + 1) * AES_BLOCK_SIZE],
        );
        inv_mix_columns(&mut state);
    }
    inv_shift_rows(&mut state);
    inv_sub_bytes(&mut state);
    add_round_key(&mut state, &enc.round_keys[..AES_BLOCK_SIZE]);
    *out = state;
}

pub unsafe extern "C" fn aes_prepareenckey_raw(
    key: *mut AesEncKey,
    in_key: *const u8,
    key_len: usize,
) -> i32 {
    if key.is_null() || in_key.is_null() {
        return EINVAL;
    }
    let in_key = unsafe { core::slice::from_raw_parts(in_key, key_len) };
    unsafe { aes_prepareenckey(&mut *key, in_key) }
}

pub unsafe extern "C" fn aes_preparekey_raw(
    key: *mut AesKey,
    in_key: *const u8,
    key_len: usize,
) -> i32 {
    if key.is_null() || in_key.is_null() {
        return EINVAL;
    }
    let in_key = unsafe { core::slice::from_raw_parts(in_key, key_len) };
    unsafe { aes_preparekey(&mut *key, in_key) }
}

pub unsafe extern "C" fn aes_expandkey_raw(
    ctx: *mut CryptoAesCtx,
    in_key: *const u8,
    key_len: u32,
) -> i32 {
    if ctx.is_null() || in_key.is_null() {
        return EINVAL;
    }
    let in_key = unsafe { core::slice::from_raw_parts(in_key, key_len as usize) };
    unsafe { aes_expandkey(&mut *ctx, in_key) }
}

pub unsafe extern "C" fn aes_encrypt_raw(key: *const AesEncKey, out: *mut u8, input: *const u8) {
    if key.is_null() || out.is_null() || input.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; AES_BLOCK_SIZE]) };
    let input = unsafe { &*(input as *const [u8; AES_BLOCK_SIZE]) };
    unsafe { aes_encrypt(&*key, out, input) };
}

pub unsafe extern "C" fn aes_decrypt_raw(key: *const AesKey, out: *mut u8, input: *const u8) {
    if key.is_null() || out.is_null() || input.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; AES_BLOCK_SIZE]) };
    let input = unsafe { &*(input as *const [u8; AES_BLOCK_SIZE]) };
    unsafe { aes_decrypt(&*key, out, input) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_source_contract_and_nist_block_vectors_match() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/aes.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/aes.h"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        assert!(source.contains("static const u8 ____cacheline_aligned aes_sbox[]"));
        assert!(source.contains("EXPORT_SYMBOL(aes_enc_tab);"));
        assert!(source.contains("EXPORT_SYMBOL(aes_dec_tab);"));
        assert!(source.contains("aes_expandkey_generic(ctx->key_enc, ctx->key_dec"));
        assert!(source.contains("EXPORT_SYMBOL(aes_preparekey);"));
        assert!(source.contains("EXPORT_SYMBOL(aes_encrypt);"));
        assert!(source.contains("EXPORT_SYMBOL(aes_decrypt);"));
        assert!(header.contains("#define AES_BLOCK_SIZE\t\t16"));
        assert!(header.contains("int aes_prepareenckey(struct aes_enckey *key"));
        assert!(header.contains("u32 key_dec[AES_MAX_KEYLENGTH_U32];"));
        assert!(testmgr.contains("static const struct cipher_testvec aes_tv_template[]"));
        assert_eq!(core::mem::size_of::<AesEncKey>(), 16 + AES_MAX_KEYLENGTH);
        assert_eq!(
            core::mem::size_of::<AesKey>(),
            16 + AES_MAX_KEYLENGTH + AES_MAX_KEYLENGTH
        );
        assert_eq!(
            core::mem::size_of::<CryptoAesCtx>(),
            AES_MAX_KEYLENGTH + AES_MAX_KEYLENGTH + core::mem::size_of::<u32>()
        );
        assert_eq!(AES_ENC_TAB[0], 0xa56363c6);
        assert_eq!(AES_ENC_TAB[1], 0x847c7cf8);
        assert_eq!(AES_ENC_TAB[255], 0x3a16162c);
        assert_eq!(AES_DEC_TAB[0], 0x50a7f451);
        assert_eq!(AES_DEC_TAB[1], 0x5365417e);
        assert_eq!(AES_DEC_TAB[255], 0x4257b8d0);

        let vectors: [(&[u8], [u8; AES_BLOCK_SIZE]); 3] = [
            (
                &[
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f,
                ],
                [
                    0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70,
                    0xb4, 0xc5, 0x5a,
                ],
            ),
            (
                &[
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
                ],
                [
                    0xdd, 0xa9, 0x7c, 0xa4, 0x86, 0x4c, 0xdf, 0xe0, 0x6e, 0xaf, 0x70, 0xa0, 0xec,
                    0x0d, 0x71, 0x91,
                ],
            ),
            (
                &[
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
                    0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
                ],
                [
                    0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b,
                    0x49, 0x60, 0x89,
                ],
            ),
        ];
        let plain = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        for (raw_key, expected) in vectors {
            let mut enc = AesEncKey::default();
            assert_eq!(aes_prepareenckey(&mut enc, raw_key), 0);
            assert_eq!(enc.nrounds, (6 + raw_key.len() / 4) as u32);
            let mut encrypted = [0u8; AES_BLOCK_SIZE];
            aes_encrypt(&enc, &mut encrypted, &plain);
            assert_eq!(encrypted, expected);

            let mut full = AesKey::default();
            assert_eq!(aes_preparekey(&mut full, raw_key), 0);
            let enc_words = round_key_words(&full.enc_key.round_keys);
            let expected_dec = inverse_round_keys(&enc_words, raw_key.len());
            assert_eq!(full.inv_round_keys, expected_dec);

            let mut ctx = CryptoAesCtx::default();
            assert_eq!(aes_expandkey(&mut ctx, raw_key), 0);
            assert_eq!(ctx.key_length, raw_key.len() as u32);
            assert_eq!(ctx.key_enc, enc_words);
            assert_eq!(ctx.key_dec, full.inv_round_keys);
            assert_eq!(
                &ctx.key_dec[..4],
                &ctx.key_enc[raw_key.len() + 24..raw_key.len() + 28]
            );
            let final_offset = raw_key.len() + 24;
            assert_eq!(
                &ctx.key_dec[final_offset..final_offset + 4],
                &ctx.key_enc[..4]
            );

            let mut decrypted = [0u8; AES_BLOCK_SIZE];
            aes_decrypt(&full, &mut decrypted, &encrypted);
            assert_eq!(decrypted, plain);
        }
        assert_eq!(aes_check_keylen(15), EINVAL);
    }

    #[test]
    fn aes_raw_exports_register_and_prepare_inverse_keys() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("aes_enc_tab"),
            Some(AES_ENC_TAB.as_ptr() as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("aes_dec_tab"),
            Some(AES_DEC_TAB.as_ptr() as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("aes_expandkey"),
            Some(aes_expandkey_raw as usize)
        );

        let raw_key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let mut key = AesKey::default();
        let mut ctx = CryptoAesCtx::default();
        unsafe {
            assert_eq!(
                aes_preparekey_raw(&mut key, raw_key.as_ptr(), raw_key.len()),
                0
            );
            assert_eq!(
                aes_expandkey_raw(&mut ctx, raw_key.as_ptr(), raw_key.len() as u32),
                0
            );
        }
        assert_eq!(key.inv_round_keys, ctx.key_dec);
        assert_ne!(ctx.key_dec, [0; AES_MAX_KEYLENGTH_U32]);
    }
}
