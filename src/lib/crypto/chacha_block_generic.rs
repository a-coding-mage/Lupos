//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/chacha-block-generic.c
//! test-origin: linux:vendor/linux/lib/crypto/chacha-block-generic.c
//! Generic ChaCha and HChaCha block functions.

use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::crypto::chacha::{
    CHACHA_BLOCK_SIZE, CHACHA_STATE_WORDS, ChachaState, HCHACHA_OUT_WORDS,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "chacha_block_generic",
        chacha_block_generic_raw as usize,
        false,
    );
    export_symbol_once(
        "hchacha_block_generic",
        hchacha_block_generic_raw as usize,
        false,
    );
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
    debug_assert!(nrounds == 20 || nrounds == 12);

    let mut i = 0;
    while i < nrounds {
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
        out[i * 4..i * 4 + 4]
            .copy_from_slice(&permuted_state.x[i].wrapping_add(state.x[i]).to_le_bytes());
    }

    state.x[12] = state.x[12].wrapping_add(1);
    permuted_state.x.fill(0);
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
    permuted_state.x.fill(0);
}

pub unsafe extern "C" fn chacha_block_generic_raw(
    state: *mut ChachaState,
    out: *mut u8,
    nrounds: i32,
) {
    if state.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u8; CHACHA_BLOCK_SIZE]) };
    unsafe { chacha_block_generic(&mut *state, out, nrounds) };
}

pub unsafe extern "C" fn hchacha_block_generic_raw(
    state: *const ChachaState,
    out: *mut u32,
    nrounds: i32,
) {
    if state.is_null() || out.is_null() {
        return;
    }
    let out = unsafe { &mut *(out as *mut [u32; HCHACHA_OUT_WORDS]) };
    unsafe { hchacha_block_generic(&*state, out, nrounds) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::crypto::chacha::{CHACHA_KEY_WORDS, chacha_init};

    #[test]
    fn chacha_block_generic_matches_linux_source_and_rfc_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/chacha-block-generic.c"
        ));
        assert!(source.contains("static void chacha_permute(struct chacha_state *state"));
        assert!(source.contains("WARN_ON_ONCE(nrounds != 20 && nrounds != 12);"));
        assert!(source.contains("put_unaligned_le32(permuted_state.x[i] + state->x[i]"));
        assert!(source.contains("state->x[12]++;"));
        assert!(source.contains("chacha_zeroize_state(&permuted_state);"));
        assert!(source.contains("memcpy(&out[0], &permuted_state.x[0], 16);"));
        assert!(source.contains("EXPORT_SYMBOL(chacha_block_generic);"));
        assert!(source.contains("EXPORT_SYMBOL(hchacha_block_generic);"));

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
        chacha_block_generic(&mut state, &mut block, 20);
        assert_eq!(
            &block[..16],
            &[
                0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20,
                0x71, 0xc4,
            ]
        );
        assert_eq!(state.x[12], 2);

        let mut hchacha_out = [0u32; HCHACHA_OUT_WORDS];
        hchacha_block_generic(&state, &mut hchacha_out, 20);
        assert_ne!(hchacha_out, [0; HCHACHA_OUT_WORDS]);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("chacha_block_generic"),
            Some(chacha_block_generic_raw as usize)
        );
    }
}
