//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/chacha.c
//! test-origin: linux:vendor/linux/lib/crypto/chacha.c
//! ChaCha and HChaCha stream-cipher helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CHACHA_IV_SIZE: usize = 16;
pub const CHACHA_KEY_SIZE: usize = 32;
pub const CHACHA_BLOCK_SIZE: usize = 64;
pub const CHACHA_KEY_WORDS: usize = 8;
pub const CHACHA_STATE_WORDS: usize = 16;
pub const HCHACHA_OUT_WORDS: usize = 8;

pub const CHACHA_CONSTANT_EXPA: u32 = 0x6170_7865;
pub const CHACHA_CONSTANT_ND_3: u32 = 0x3320_646e;
pub const CHACHA_CONSTANT_2_BY: u32 = 0x7962_2d32;
pub const CHACHA_CONSTANT_TE_K: u32 = 0x6b20_6574;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChachaState {
    pub x: [u32; CHACHA_STATE_WORDS],
}

impl Default for ChachaState {
    fn default() -> Self {
        Self {
            x: [0; CHACHA_STATE_WORDS],
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("chacha_crypt", chacha_crypt_raw as usize, true);
    export_symbol_once("hchacha_block", hchacha_block_raw as usize, true);
}

pub fn chacha_init_consts(state: &mut ChachaState) {
    state.x[0] = CHACHA_CONSTANT_EXPA;
    state.x[1] = CHACHA_CONSTANT_ND_3;
    state.x[2] = CHACHA_CONSTANT_2_BY;
    state.x[3] = CHACHA_CONSTANT_TE_K;
}

pub fn chacha_init(state: &mut ChachaState, key: &[u32; CHACHA_KEY_WORDS], iv: &[u8]) {
    assert!(iv.len() >= CHACHA_IV_SIZE);
    chacha_init_consts(state);
    state.x[4..12].copy_from_slice(key);
    state.x[12] = u32::from_le_bytes([iv[0], iv[1], iv[2], iv[3]]);
    state.x[13] = u32::from_le_bytes([iv[4], iv[5], iv[6], iv[7]]);
    state.x[14] = u32::from_le_bytes([iv[8], iv[9], iv[10], iv[11]]);
    state.x[15] = u32::from_le_bytes([iv[12], iv[13], iv[14], iv[15]]);
}

#[inline]
fn quarter_round(x: &mut [u32; CHACHA_STATE_WORDS], a: usize, b: usize, c: usize, d: usize) {
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(16);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(12);
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(8);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(7);
}

fn chacha_permute(state: &mut ChachaState, nrounds: i32) {
    let rounds = if nrounds == 12 { 12 } else { 20 };
    let mut i = 0;
    while i < rounds {
        quarter_round(&mut state.x, 0, 4, 8, 12);
        quarter_round(&mut state.x, 1, 5, 9, 13);
        quarter_round(&mut state.x, 2, 6, 10, 14);
        quarter_round(&mut state.x, 3, 7, 11, 15);
        quarter_round(&mut state.x, 0, 5, 10, 15);
        quarter_round(&mut state.x, 1, 6, 11, 12);
        quarter_round(&mut state.x, 2, 7, 8, 13);
        quarter_round(&mut state.x, 3, 4, 9, 14);
        i += 2;
    }
}

pub fn chacha_block_generic(
    state: &mut ChachaState,
    out: &mut [u8; CHACHA_BLOCK_SIZE],
    nrounds: i32,
) {
    let mut permuted_state = *state;
    chacha_permute(&mut permuted_state, nrounds);

    for i in 0..CHACHA_STATE_WORDS {
        let word = permuted_state.x[i].wrapping_add(state.x[i]).to_le_bytes();
        out[i * 4..i * 4 + 4].copy_from_slice(&word);
    }
    state.x[12] = state.x[12].wrapping_add(1);
}

pub fn hchacha_block_generic(
    state: &ChachaState,
    out: &mut [u32; HCHACHA_OUT_WORDS],
    nrounds: i32,
) {
    let mut permuted_state = *state;
    chacha_permute(&mut permuted_state, nrounds);
    out[0..4].copy_from_slice(&permuted_state.x[0..4]);
    out[4..8].copy_from_slice(&permuted_state.x[12..16]);
}

pub fn chacha_crypt(state: &mut ChachaState, dst: &mut [u8], src: &[u8], nrounds: i32) {
    assert!(dst.len() >= src.len());
    let mut offset = 0usize;
    let mut stream = [0u8; CHACHA_BLOCK_SIZE];
    while offset < src.len() {
        chacha_block_generic(state, &mut stream, nrounds);
        let take = core::cmp::min(CHACHA_BLOCK_SIZE, src.len() - offset);
        for i in 0..take {
            dst[offset + i] = src[offset + i] ^ stream[i];
        }
        offset += take;
    }
}

pub unsafe extern "C" fn chacha_crypt_raw(
    state: *mut ChachaState,
    dst: *mut u8,
    src: *const u8,
    bytes: u32,
    nrounds: i32,
) {
    if state.is_null() || dst.is_null() || src.is_null() {
        return;
    }
    let len = bytes as usize;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, len) };
    let src = unsafe { core::slice::from_raw_parts(src, len) };
    unsafe { chacha_crypt(&mut *state, dst, src, nrounds) };
}

pub unsafe extern "C" fn hchacha_block_raw(state: *const ChachaState, out: *mut u32, nrounds: i32) {
    if state.is_null() || out.is_null() {
        return;
    }
    let mut words = [0u32; HCHACHA_OUT_WORDS];
    unsafe { hchacha_block_generic(&*state, &mut words, nrounds) };
    unsafe { core::ptr::copy_nonoverlapping(words.as_ptr(), out, HCHACHA_OUT_WORDS) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chacha_crypt_matches_linux_generic_block_flow_and_rfc_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/chacha.c"
        ));
        let block_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/chacha-block-generic.c"
        ));
        assert!(source.contains("chacha_crypt_generic"));
        assert!(source.contains("chacha_block_generic(state, stream, nrounds);"));
        assert!(source.contains("crypto_xor_cpy(dst, src, stream, CHACHA_BLOCK_SIZE);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(chacha_crypt);"));
        assert!(block_source.contains("chacha_permute(&permuted_state, nrounds);"));
        assert!(block_source.contains("state->x[12]++;"));

        let mut key = [0u32; CHACHA_KEY_WORDS];
        let mut byte = 0u8;
        for word in &mut key {
            *word = u32::from_le_bytes([byte, byte + 1, byte + 2, byte + 3]);
            byte += 4;
        }
        let iv = [1u8, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let mut state = ChachaState::default();
        chacha_init(&mut state, &key, &iv);
        let mut block = [0u8; CHACHA_BLOCK_SIZE];
        chacha_crypt(&mut state, &mut block, &[0u8; CHACHA_BLOCK_SIZE], 20);
        assert_eq!(
            &block[..16],
            &[
                0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20,
                0x71, 0xc4,
            ]
        );
        assert_eq!(state.x[12], 2);
    }
}
