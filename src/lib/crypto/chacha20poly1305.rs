//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/chacha20poly1305.c
//! test-origin: linux:vendor/linux/lib/crypto/chacha20poly1305.c
//! ChaCha20-Poly1305 and XChaCha20-Poly1305 AEAD helpers.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::crypto::chacha::{
    CHACHA_IV_SIZE, CHACHA_KEY_WORDS, ChachaState, chacha_crypt, chacha_init, hchacha_block_generic,
};
use crate::lib::scatterlist::{LinuxScatterList, SG_CHAIN, SG_END, SG_PAGE_LINK_MASK};

pub const XCHACHA20POLY1305_NONCE_SIZE: usize = 24;
pub const CHACHA20POLY1305_KEY_SIZE: usize = 32;
pub const CHACHA20POLY1305_AUTHTAG_SIZE: usize = 16;
const POLY1305_BLOCK_SIZE: usize = 16;
const POLY1305_KEY_SIZE: usize = 32;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "chacha20poly1305_encrypt",
        chacha20poly1305_encrypt_raw as usize,
        false,
    );
    export_symbol_once(
        "chacha20poly1305_decrypt",
        chacha20poly1305_decrypt_raw as usize,
        false,
    );
    export_symbol_once(
        "xchacha20poly1305_encrypt",
        xchacha20poly1305_encrypt_raw as usize,
        false,
    );
    export_symbol_once(
        "xchacha20poly1305_decrypt",
        xchacha20poly1305_decrypt_raw as usize,
        false,
    );
    export_symbol_once(
        "chacha20poly1305_encrypt_sg_inplace",
        chacha20poly1305_encrypt_sg_inplace_raw as usize,
        false,
    );
    export_symbol_once(
        "chacha20poly1305_decrypt_sg_inplace",
        chacha20poly1305_decrypt_sg_inplace_raw as usize,
        false,
    );
}

fn chacha_load_key(input: &[u8; CHACHA20POLY1305_KEY_SIZE]) -> [u32; CHACHA_KEY_WORDS] {
    let mut k = [0u32; CHACHA_KEY_WORDS];
    for (i, word) in k.iter_mut().enumerate() {
        let offset = i * 4;
        *word = u32::from_le_bytes([
            input[offset],
            input[offset + 1],
            input[offset + 2],
            input[offset + 3],
        ]);
    }
    k
}

fn init_chacha20(key: &[u8; CHACHA20POLY1305_KEY_SIZE], nonce: u64) -> ChachaState {
    let k = chacha_load_key(key);
    let mut iv = [0u8; CHACHA_IV_SIZE];
    iv[8..].copy_from_slice(&nonce.to_le_bytes());
    let mut state = ChachaState::default();
    chacha_init(&mut state, &k, &iv);
    state
}

fn init_xchacha20(
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
    nonce: &[u8; XCHACHA20POLY1305_NONCE_SIZE],
) -> ChachaState {
    let k = chacha_load_key(key);
    let mut state = ChachaState::default();
    chacha_init(&mut state, &k, &nonce[..16]);
    let mut subkey = [0u32; CHACHA_KEY_WORDS];
    hchacha_block_generic(&state, &mut subkey, 20);
    let mut iv = [0u8; CHACHA_IV_SIZE];
    iv[8..].copy_from_slice(&nonce[16..]);
    chacha_init(&mut state, &subkey, &iv);
    state
}

#[derive(Clone, Copy)]
struct Poly1305 {
    r: [u64; 5],
    s: [u64; 4],
    h: [u64; 5],
}

fn limb(block: &[u8; 17], limb: usize) -> u64 {
    let start = limb * 26;
    let mut out = 0u64;
    for bit in 0..26 {
        let idx = start + bit;
        if ((block[idx / 8] >> (idx % 8)) & 1) != 0 {
            out |= 1u64 << bit;
        }
    }
    out
}

impl Poly1305 {
    fn new(key: &[u8; POLY1305_KEY_SIZE]) -> Self {
        let t0 = u32::from_le_bytes([key[0], key[1], key[2], key[3]]) as u64;
        let t1 = u32::from_le_bytes([key[4], key[5], key[6], key[7]]) as u64;
        let t2 = u32::from_le_bytes([key[8], key[9], key[10], key[11]]) as u64;
        let t3 = u32::from_le_bytes([key[12], key[13], key[14], key[15]]) as u64;
        let r = [
            t0 & 0x03ff_ffff,
            ((t0 >> 26) | (t1 << 6)) & 0x03ff_ff03,
            ((t1 >> 20) | (t2 << 12)) & 0x03ff_c0ff,
            ((t2 >> 14) | (t3 << 18)) & 0x03f0_3fff,
            (t3 >> 8) & 0x000f_ffff,
        ];
        let s = [
            u32::from_le_bytes([key[16], key[17], key[18], key[19]]) as u64,
            u32::from_le_bytes([key[20], key[21], key[22], key[23]]) as u64,
            u32::from_le_bytes([key[24], key[25], key[26], key[27]]) as u64,
            u32::from_le_bytes([key[28], key[29], key[30], key[31]]) as u64,
        ];
        Self { r, s, h: [0; 5] }
    }

    fn update(&mut self, data: &[u8]) {
        for chunk in data.chunks(POLY1305_BLOCK_SIZE) {
            let mut block = [0u8; 17];
            block[..chunk.len()].copy_from_slice(chunk);
            block[chunk.len()] = 1;
            for i in 0..5 {
                self.h[i] = self.h[i].wrapping_add(limb(&block, i));
            }
            self.mul_reduce();
        }
    }

    fn mul_reduce(&mut self) {
        let r0 = self.r[0];
        let r1 = self.r[1];
        let r2 = self.r[2];
        let r3 = self.r[3];
        let r4 = self.r[4];
        let s1 = r1 * 5;
        let s2 = r2 * 5;
        let s3 = r3 * 5;
        let s4 = r4 * 5;
        let h0 = self.h[0];
        let h1 = self.h[1];
        let h2 = self.h[2];
        let h3 = self.h[3];
        let h4 = self.h[4];
        let mut d0 = h0 * r0 + h1 * s4 + h2 * s3 + h3 * s2 + h4 * s1;
        let mut d1 = h0 * r1 + h1 * r0 + h2 * s4 + h3 * s3 + h4 * s2;
        let mut d2 = h0 * r2 + h1 * r1 + h2 * r0 + h3 * s4 + h4 * s3;
        let mut d3 = h0 * r3 + h1 * r2 + h2 * r1 + h3 * r0 + h4 * s4;
        let mut d4 = h0 * r4 + h1 * r3 + h2 * r2 + h3 * r1 + h4 * r0;

        let mut c = d0 >> 26;
        self.h[0] = d0 & 0x03ff_ffff;
        d1 += c;
        c = d1 >> 26;
        self.h[1] = d1 & 0x03ff_ffff;
        d2 += c;
        c = d2 >> 26;
        self.h[2] = d2 & 0x03ff_ffff;
        d3 += c;
        c = d3 >> 26;
        self.h[3] = d3 & 0x03ff_ffff;
        d4 += c;
        c = d4 >> 26;
        self.h[4] = d4 & 0x03ff_ffff;
        self.h[0] += c * 5;
        c = self.h[0] >> 26;
        self.h[0] &= 0x03ff_ffff;
        self.h[1] += c;
    }

    fn final_tag(mut self) -> [u8; CHACHA20POLY1305_AUTHTAG_SIZE] {
        let mut c = self.h[1] >> 26;
        self.h[1] &= 0x03ff_ffff;
        self.h[2] += c;
        c = self.h[2] >> 26;
        self.h[2] &= 0x03ff_ffff;
        self.h[3] += c;
        c = self.h[3] >> 26;
        self.h[3] &= 0x03ff_ffff;
        self.h[4] += c;
        c = self.h[4] >> 26;
        self.h[4] &= 0x03ff_ffff;
        self.h[0] += c * 5;
        c = self.h[0] >> 26;
        self.h[0] &= 0x03ff_ffff;
        self.h[1] += c;

        let mut g = [0u64; 5];
        g[0] = self.h[0] + 5;
        c = g[0] >> 26;
        g[0] &= 0x03ff_ffff;
        for i in 1..4 {
            g[i] = self.h[i] + c;
            c = g[i] >> 26;
            g[i] &= 0x03ff_ffff;
        }
        g[4] = self.h[4] + c;
        let use_g = g[4] >> 26 != 0;
        g[4] = g[4].wrapping_sub(1 << 26);
        let h = if use_g { g } else { self.h };

        let f0 = h[0] | (h[1] << 26) | (h[2] << 52);
        let f1 = (h[2] >> 12) | (h[3] << 14) | (h[4] << 40);
        let s0 = self.s[0] | (self.s[1] << 32);
        let s1 = self.s[2] | (self.s[3] << 32);
        let lo = f0.wrapping_add(s0);
        let hi = f1.wrapping_add(s1).wrapping_add(u64::from(lo < f0));
        let mut out = [0u8; CHACHA20POLY1305_AUTHTAG_SIZE];
        out[..8].copy_from_slice(&lo.to_le_bytes());
        out[8..].copy_from_slice(&hi.to_le_bytes());
        out
    }
}

fn poly1305_aead_mac(
    one_time_key: &[u8; POLY1305_KEY_SIZE],
    aad: &[u8],
    ciphertext: &[u8],
) -> [u8; CHACHA20POLY1305_AUTHTAG_SIZE] {
    let mut poly = Poly1305::new(one_time_key);
    let mut msg = Vec::with_capacity(
        aad.len()
            + ((POLY1305_BLOCK_SIZE - aad.len() % POLY1305_BLOCK_SIZE) % POLY1305_BLOCK_SIZE)
            + ciphertext.len()
            + ((POLY1305_BLOCK_SIZE - ciphertext.len() % POLY1305_BLOCK_SIZE)
                % POLY1305_BLOCK_SIZE)
            + 16,
    );
    msg.extend_from_slice(aad);
    if aad.len() % POLY1305_BLOCK_SIZE != 0 {
        msg.extend_from_slice(
            &[0u8; POLY1305_BLOCK_SIZE][..POLY1305_BLOCK_SIZE - aad.len() % POLY1305_BLOCK_SIZE],
        );
    }
    msg.extend_from_slice(ciphertext);
    if ciphertext.len() % POLY1305_BLOCK_SIZE != 0 {
        msg.extend_from_slice(
            &[0u8; POLY1305_BLOCK_SIZE]
                [..POLY1305_BLOCK_SIZE - ciphertext.len() % POLY1305_BLOCK_SIZE],
        );
    }
    msg.extend_from_slice(&(aad.len() as u64).to_le_bytes());
    msg.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    poly.update(&msg);
    poly.final_tag()
}

fn consttime_ne(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return true;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff != 0
}

fn encrypt_with_state(dst: &mut [u8], src: &[u8], aad: &[u8], state: &mut ChachaState) {
    assert!(dst.len() >= src.len() + CHACHA20POLY1305_AUTHTAG_SIZE);
    let mut block0 = [0u8; POLY1305_KEY_SIZE];
    chacha_crypt(state, &mut block0, &[0u8; POLY1305_KEY_SIZE], 20);
    chacha_crypt(state, &mut dst[..src.len()], src, 20);
    let tag = poly1305_aead_mac(&block0, aad, &dst[..src.len()]);
    dst[src.len()..src.len() + CHACHA20POLY1305_AUTHTAG_SIZE].copy_from_slice(&tag);
    *state = ChachaState::default();
}

fn decrypt_with_state(dst: &mut [u8], src: &[u8], aad: &[u8], state: &mut ChachaState) -> bool {
    if src.len() < CHACHA20POLY1305_AUTHTAG_SIZE {
        return false;
    }
    let dst_len = src.len() - CHACHA20POLY1305_AUTHTAG_SIZE;
    assert!(dst.len() >= dst_len);
    let mut block0 = [0u8; POLY1305_KEY_SIZE];
    chacha_crypt(state, &mut block0, &[0u8; POLY1305_KEY_SIZE], 20);
    let tag = poly1305_aead_mac(&block0, aad, &src[..dst_len]);
    if consttime_ne(&tag, &src[dst_len..]) {
        *state = ChachaState::default();
        return false;
    }
    chacha_crypt(state, &mut dst[..dst_len], &src[..dst_len], 20);
    *state = ChachaState::default();
    true
}

pub fn chacha20poly1305_encrypt(
    dst: &mut [u8],
    src: &[u8],
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) {
    let mut state = init_chacha20(key, nonce);
    encrypt_with_state(dst, src, aad, &mut state);
}

pub fn chacha20poly1305_decrypt(
    dst: &mut [u8],
    src: &[u8],
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    let mut state = init_chacha20(key, nonce);
    decrypt_with_state(dst, src, aad, &mut state)
}

pub fn xchacha20poly1305_encrypt(
    dst: &mut [u8],
    src: &[u8],
    aad: &[u8],
    nonce: &[u8; XCHACHA20POLY1305_NONCE_SIZE],
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) {
    let mut state = init_xchacha20(key, nonce);
    encrypt_with_state(dst, src, aad, &mut state);
}

pub fn xchacha20poly1305_decrypt(
    dst: &mut [u8],
    src: &[u8],
    aad: &[u8],
    nonce: &[u8; XCHACHA20POLY1305_NONCE_SIZE],
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    let mut state = init_xchacha20(key, nonce);
    decrypt_with_state(dst, src, aad, &mut state)
}

fn collect_segments_prefix(segments: &mut [&mut [u8]], len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut remaining = len;
    for segment in segments.iter_mut() {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(segment.len());
        out.extend_from_slice(&segment[..take]);
        remaining -= take;
    }
    (remaining == 0).then_some(out)
}

fn copy_to_segments_prefix(segments: &mut [&mut [u8]], src: &[u8]) -> bool {
    let mut offset = 0usize;
    for segment in segments.iter_mut() {
        if offset == src.len() {
            break;
        }
        let take = (src.len() - offset).min(segment.len());
        segment[..take].copy_from_slice(&src[offset..offset + take]);
        offset += take;
    }
    offset == src.len()
}

unsafe fn linux_sg_next(sg: *mut LinuxScatterList) -> *mut LinuxScatterList {
    if sg.is_null() || unsafe { (*sg).page_link } & SG_END != 0 {
        return core::ptr::null_mut();
    }
    let next = unsafe { sg.add(1) };
    if unsafe { (*next).page_link } & SG_CHAIN != 0 {
        (unsafe { (*next).page_link } & !SG_PAGE_LINK_MASK) as *mut LinuxScatterList
    } else {
        next
    }
}

unsafe fn linux_sg_addr(sg: *mut LinuxScatterList) -> *mut u8 {
    let entry = unsafe { &*sg };
    if entry.dma_address != 0 {
        entry.dma_address as *mut u8
    } else {
        ((entry.page_link & !SG_PAGE_LINK_MASK) + entry.offset as usize) as *mut u8
    }
}

unsafe fn collect_linux_sg_segments_mut<'a>(
    src: *mut LinuxScatterList,
    len: usize,
) -> Option<Vec<&'a mut [u8]>> {
    let mut segments = Vec::new();
    let mut remaining = len;
    let mut sg = src;
    while remaining != 0 {
        if sg.is_null() {
            return None;
        }
        if unsafe { (*sg).page_link } & SG_CHAIN != 0 {
            sg = (unsafe { (*sg).page_link } & !SG_PAGE_LINK_MASK) as *mut LinuxScatterList;
            continue;
        }
        let entry_len = unsafe { (*sg).length } as usize;
        if entry_len != 0 {
            let take = remaining.min(entry_len);
            let addr = unsafe { linux_sg_addr(sg) };
            if addr.is_null() {
                return None;
            }
            segments.push(unsafe { core::slice::from_raw_parts_mut(addr, take) });
            remaining -= take;
        }
        sg = unsafe { linux_sg_next(sg) };
    }
    Some(segments)
}

pub fn chacha20poly1305_encrypt_sg_inplace_segments(
    segments: &mut [&mut [u8]],
    src_len: usize,
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    let Some(plaintext) = collect_segments_prefix(segments, src_len) else {
        return false;
    };
    let Some(total_len) = src_len.checked_add(CHACHA20POLY1305_AUTHTAG_SIZE) else {
        return false;
    };
    if collect_segments_prefix(segments, total_len).is_none() {
        return false;
    }

    let mut encrypted = vec![0u8; total_len];
    chacha20poly1305_encrypt(&mut encrypted, &plaintext, aad, nonce, key);
    copy_to_segments_prefix(segments, &encrypted)
}

pub fn chacha20poly1305_decrypt_sg_inplace_segments(
    segments: &mut [&mut [u8]],
    src_len: usize,
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    if src_len < CHACHA20POLY1305_AUTHTAG_SIZE {
        return false;
    }
    let Some(ciphertext) = collect_segments_prefix(segments, src_len) else {
        return false;
    };
    let plain_len = src_len - CHACHA20POLY1305_AUTHTAG_SIZE;
    let mut plaintext = vec![0u8; plain_len];
    if !chacha20poly1305_decrypt(&mut plaintext, &ciphertext, aad, nonce, key) {
        return false;
    }
    copy_to_segments_prefix(segments, &plaintext)
}

pub fn chacha20poly1305_encrypt_sg_inplace(
    src: &mut [u8],
    src_len: usize,
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    chacha20poly1305_encrypt_sg_inplace_segments(&mut [src], src_len, aad, nonce, key)
}

pub fn chacha20poly1305_decrypt_sg_inplace(
    src: &mut [u8],
    src_len: usize,
    aad: &[u8],
    nonce: u64,
    key: &[u8; CHACHA20POLY1305_KEY_SIZE],
) -> bool {
    chacha20poly1305_decrypt_sg_inplace_segments(&mut [src], src_len, aad, nonce, key)
}

pub unsafe extern "C" fn chacha20poly1305_encrypt_raw(
    dst: *mut u8,
    src: *const u8,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: u64,
    key: *const u8,
) {
    if dst.is_null() || (src.is_null() && src_len != 0) || key.is_null() {
        return;
    }
    let Some(dst_len) = src_len.checked_add(CHACHA20POLY1305_AUTHTAG_SIZE) else {
        return;
    };
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, dst_len) };
    let src = unsafe { core::slice::from_raw_parts(src, src_len) };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    chacha20poly1305_encrypt(dst, src, aad, nonce, key);
}

pub unsafe extern "C" fn chacha20poly1305_decrypt_raw(
    dst: *mut u8,
    src: *const u8,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: u64,
    key: *const u8,
) -> bool {
    if dst.is_null() || src.is_null() || key.is_null() || src_len < CHACHA20POLY1305_AUTHTAG_SIZE {
        return false;
    }
    let dst_len = src_len - CHACHA20POLY1305_AUTHTAG_SIZE;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, dst_len) };
    let src = unsafe { core::slice::from_raw_parts(src, src_len) };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    chacha20poly1305_decrypt(dst, src, aad, nonce, key)
}

pub unsafe extern "C" fn xchacha20poly1305_encrypt_raw(
    dst: *mut u8,
    src: *const u8,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: *const u8,
    key: *const u8,
) {
    if dst.is_null() || (src.is_null() && src_len != 0) || nonce.is_null() || key.is_null() {
        return;
    }
    let Some(dst_len) = src_len.checked_add(CHACHA20POLY1305_AUTHTAG_SIZE) else {
        return;
    };
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, dst_len) };
    let src = unsafe { core::slice::from_raw_parts(src, src_len) };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let nonce = unsafe { &*(nonce as *const [u8; XCHACHA20POLY1305_NONCE_SIZE]) };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    xchacha20poly1305_encrypt(dst, src, aad, nonce, key);
}

pub unsafe extern "C" fn xchacha20poly1305_decrypt_raw(
    dst: *mut u8,
    src: *const u8,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: *const u8,
    key: *const u8,
) -> bool {
    if dst.is_null()
        || src.is_null()
        || nonce.is_null()
        || key.is_null()
        || src_len < CHACHA20POLY1305_AUTHTAG_SIZE
    {
        return false;
    }
    let dst_len = src_len - CHACHA20POLY1305_AUTHTAG_SIZE;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, dst_len) };
    let src = unsafe { core::slice::from_raw_parts(src, src_len) };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let nonce = unsafe { &*(nonce as *const [u8; XCHACHA20POLY1305_NONCE_SIZE]) };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    xchacha20poly1305_decrypt(dst, src, aad, nonce, key)
}

pub unsafe extern "C" fn chacha20poly1305_encrypt_sg_inplace_raw(
    src: *mut LinuxScatterList,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: u64,
    key: *const u8,
) -> bool {
    if src.is_null() || key.is_null() {
        return false;
    }
    let Some(total_len) = src_len.checked_add(CHACHA20POLY1305_AUTHTAG_SIZE) else {
        return false;
    };
    let Some(mut segments) = (unsafe { collect_linux_sg_segments_mut(src, total_len) }) else {
        return false;
    };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    chacha20poly1305_encrypt_sg_inplace_segments(&mut segments, src_len, aad, nonce, key)
}

pub unsafe extern "C" fn chacha20poly1305_decrypt_sg_inplace_raw(
    src: *mut LinuxScatterList,
    src_len: usize,
    aad: *const u8,
    aad_len: usize,
    nonce: u64,
    key: *const u8,
) -> bool {
    if src.is_null() || key.is_null() || src_len < CHACHA20POLY1305_AUTHTAG_SIZE {
        return false;
    }
    let Some(mut segments) = (unsafe { collect_linux_sg_segments_mut(src, src_len) }) else {
        return false;
    };
    let aad = if aad_len == 0 {
        &[]
    } else if aad.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(aad, aad_len) }
    };
    let key = unsafe { &*(key as *const [u8; CHACHA20POLY1305_KEY_SIZE]) };
    chacha20poly1305_decrypt_sg_inplace_segments(&mut segments, src_len, aad, nonce, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    fn kunit_bytes(kunit: &str, name: &str) -> Vec<u8> {
        let marker = format!("static const u8 {name}[] = {{");
        let start = kunit.find(&marker).expect("kunit vector start") + marker.len();
        let end = kunit[start..].find("};").expect("kunit vector end") + start;
        let mut out = Vec::new();
        for token in kunit[start..end]
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .filter(|token| token.starts_with("0x"))
        {
            out.push(u8::from_str_radix(&token[2..], 16).expect("hex byte"));
        }
        out
    }

    fn key32(bytes: &[u8]) -> [u8; CHACHA20POLY1305_KEY_SIZE] {
        let mut key = [0u8; CHACHA20POLY1305_KEY_SIZE];
        key.copy_from_slice(bytes);
        key
    }

    fn nonce24(bytes: &[u8]) -> [u8; XCHACHA20POLY1305_NONCE_SIZE] {
        let mut nonce = [0u8; XCHACHA20POLY1305_NONCE_SIZE];
        nonce.copy_from_slice(bytes);
        nonce
    }

    fn sg_entry(buf: &mut [u8], is_last: bool) -> LinuxScatterList {
        let mut page_link = buf.as_mut_ptr() as usize & !SG_PAGE_LINK_MASK;
        if is_last {
            page_link |= SG_END;
        }
        LinuxScatterList {
            page_link,
            offset: 0,
            length: buf.len() as u32,
            dma_address: buf.as_mut_ptr() as usize,
            dma_length: buf.len() as u32,
            dma_flags: 0,
        }
    }

    #[test]
    fn chacha20poly1305_matches_linux_kunit_empty_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/chacha20poly1305.c"
        ));
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/chacha20poly1305_kunit.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/chacha20poly1305.h"
        ));
        assert!(source.contains("static void chacha_load_key(u32 *k, const u8 *in)"));
        assert!(source.contains("poly1305_update(&poly1305_state, ad, ad_len);"));
        assert!(source.contains("b.lens[0] = cpu_to_le64(ad_len);"));
        assert!(source.contains("EXPORT_SYMBOL(chacha20poly1305_encrypt);"));
        assert!(source.contains("chacha20poly1305_crypt_sg_inplace"));
        assert!(source.contains("sg_miter_start(&miter, src, sg_nents(src), flags);"));
        assert!(source.contains("sg_copy_buffer(src, sg_nents(src), b.mac[encrypt]"));
        assert!(source.contains("EXPORT_SYMBOL(chacha20poly1305_encrypt_sg_inplace);"));
        assert!(source.contains("EXPORT_SYMBOL(chacha20poly1305_decrypt_sg_inplace);"));
        assert!(kunit.contains("chacha20poly1305_enc_vectors[]"));
        assert!(kunit.contains("enc_output002"));
        assert!(kunit.contains("sg_init_one(sg_src, computed_output"));
        assert!(kunit.contains("chacha20poly1305_encrypt_sg_inplace(sg_src"));
        assert!(kunit.contains("chacha20poly1305_decrypt_sg_inplace(sg_src"));
        assert!(header.contains("CHACHA20POLY1305_AUTHTAG_SIZE = 16"));
        assert!(header.contains("chacha20poly1305_encrypt_sg_inplace"));
        assert!(header.contains("chacha20poly1305_decrypt_sg_inplace"));

        let key = [
            0x4c, 0xf5, 0x96, 0x83, 0x38, 0xe6, 0xae, 0x7f, 0x2d, 0x29, 0x25, 0x76, 0xd5, 0x75,
            0x27, 0x86, 0x91, 0x9a, 0x27, 0x7a, 0xfb, 0x46, 0xc5, 0xef, 0x94, 0x81, 0x79, 0x57,
            0x14, 0x59, 0x40, 0x68,
        ];
        let nonce = u64::from_le_bytes([0xca, 0xbf, 0x33, 0x71, 0x32, 0x45, 0x77, 0x8e]);
        let mut out = [0u8; CHACHA20POLY1305_AUTHTAG_SIZE];
        chacha20poly1305_encrypt(&mut out, &[], &[], nonce, &key);
        assert_eq!(
            out,
            [
                0xea, 0xe0, 0x1e, 0x9e, 0x2c, 0x91, 0xaa, 0xe1, 0xdb, 0x5d, 0x99, 0x3f, 0x8a, 0xf7,
                0x69, 0x92,
            ]
        );
        let mut plain = [];
        assert!(chacha20poly1305_decrypt(&mut plain, &out, &[], nonce, &key));
        out[0] ^= 1;
        assert!(!chacha20poly1305_decrypt(
            &mut plain,
            &out,
            &[],
            nonce,
            &key
        ));
    }

    #[test]
    fn chacha20poly1305_sg_inplace_matches_linux_kunit_one_byte_vector() {
        let key = [
            0x4b, 0x28, 0x4b, 0xa3, 0x7b, 0xbe, 0xe9, 0xf8, 0x31, 0x80, 0x82, 0xd7, 0xd8, 0xe8,
            0xb5, 0xa1, 0xe2, 0x18, 0x18, 0x8a, 0x9c, 0xfa, 0xa3, 0x3d, 0x25, 0x71, 0x3e, 0x40,
            0xbc, 0x54, 0x7a, 0x3e,
        ];
        let aad = [0x6a, 0xe2, 0xad, 0x3f, 0x88, 0x39, 0x5a, 0x40];
        let nonce = u64::from_le_bytes([0xd2, 0x32, 0x1f, 0x29, 0x28, 0xc6, 0xc4, 0xc4]);
        let expected = [
            0xb7, 0x1b, 0xb0, 0x73, 0x59, 0xb0, 0x84, 0xb2, 0x6d, 0x8e, 0xab, 0x94, 0x31, 0xa1,
            0xae, 0xac, 0x89,
        ];
        let mut buf = [0u8; 1 + CHACHA20POLY1305_AUTHTAG_SIZE];
        buf[0] = 0xa4;

        assert!(chacha20poly1305_encrypt_sg_inplace(
            &mut buf, 1, &aad, nonce, &key
        ));
        assert_eq!(buf, expected);
        assert!(chacha20poly1305_decrypt_sg_inplace(
            &mut buf,
            expected.len(),
            &aad,
            nonce,
            &key
        ));
        assert_eq!(buf[0], 0xa4);

        let mut short = [0u8; CHACHA20POLY1305_AUTHTAG_SIZE - 1];
        assert!(!chacha20poly1305_decrypt_sg_inplace(
            &mut short,
            CHACHA20POLY1305_AUTHTAG_SIZE - 1,
            &aad,
            nonce,
            &key
        ));
    }

    #[test]
    fn chacha20poly1305_raw_sg_inplace_uses_linux_scatterlist_entries() {
        let key = [0x5au8; CHACHA20POLY1305_KEY_SIZE];
        let aad = b"linux scatterlist aad";
        let nonce = 0xfedc_ba98_7654_3210u64;
        let plaintext = b"linux scatterlist raw export spans several entries";
        let mut direct = vec![0u8; plaintext.len() + CHACHA20POLY1305_AUTHTAG_SIZE];
        chacha20poly1305_encrypt(&mut direct, plaintext, aad, nonce, &key);

        let mut first = plaintext[..9].to_vec();
        let mut second = plaintext[9..33].to_vec();
        let mut third = vec![0u8; plaintext.len() - 33 + CHACHA20POLY1305_AUTHTAG_SIZE];
        third[..plaintext.len() - 33].copy_from_slice(&plaintext[33..]);
        let mut sg = [
            sg_entry(&mut first, false),
            sg_entry(&mut second, false),
            sg_entry(&mut third, true),
        ];

        unsafe {
            assert!(chacha20poly1305_encrypt_sg_inplace_raw(
                sg.as_mut_ptr(),
                plaintext.len(),
                aad.as_ptr(),
                aad.len(),
                nonce,
                key.as_ptr(),
            ));
        }
        let mut combined = Vec::new();
        combined.extend_from_slice(&first);
        combined.extend_from_slice(&second);
        combined.extend_from_slice(&third);
        assert_eq!(combined, direct);

        unsafe {
            assert!(chacha20poly1305_decrypt_sg_inplace_raw(
                sg.as_mut_ptr(),
                direct.len(),
                aad.as_ptr(),
                aad.len(),
                nonce,
                key.as_ptr(),
            ));
        }
        let mut restored = Vec::new();
        restored.extend_from_slice(&first);
        restored.extend_from_slice(&second);
        restored.extend_from_slice(&third[..plaintext.len() - 33]);
        assert_eq!(restored, plaintext);
    }

    #[test]
    fn chacha20poly1305_matches_selected_linux_kunit_vectors() {
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/chacha20poly1305_kunit.c"
        ));

        for id in ["001", "004", "006"] {
            let input = kunit_bytes(kunit, &format!("enc_input{id}"));
            let output = kunit_bytes(kunit, &format!("enc_output{id}"));
            let aad = kunit_bytes(kunit, &format!("enc_assoc{id}"));
            let nonce = kunit_bytes(kunit, &format!("enc_nonce{id}"));
            let key = key32(&kunit_bytes(kunit, &format!("enc_key{id}")));
            assert_eq!(nonce.len(), 8);
            assert_eq!(output.len(), input.len() + CHACHA20POLY1305_AUTHTAG_SIZE);

            let nonce = u64::from_le_bytes([
                nonce[0], nonce[1], nonce[2], nonce[3], nonce[4], nonce[5], nonce[6], nonce[7],
            ]);
            let mut encrypted = vec![0u8; output.len()];
            chacha20poly1305_encrypt(&mut encrypted, &input, &aad, nonce, &key);
            assert_eq!(encrypted, output, "encrypt vector {id}");

            let mut decrypted = vec![0u8; input.len()];
            assert!(chacha20poly1305_decrypt(
                &mut decrypted,
                &output,
                &aad,
                nonce,
                &key
            ));
            assert_eq!(decrypted, input, "decrypt vector {id}");
        }
    }

    #[test]
    fn xchacha20poly1305_matches_linux_kunit_vector() {
        let kunit = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/tests/chacha20poly1305_kunit.c"
        ));
        let input = kunit_bytes(kunit, "xenc_input001");
        let output = kunit_bytes(kunit, "xenc_output001");
        let aad = kunit_bytes(kunit, "xenc_assoc001");
        let nonce = nonce24(&kunit_bytes(kunit, "xenc_nonce001"));
        let key = key32(&kunit_bytes(kunit, "xenc_key001"));
        let mut encrypted = vec![0u8; output.len()];

        xchacha20poly1305_encrypt(&mut encrypted, &input, &aad, &nonce, &key);
        assert_eq!(encrypted, output);

        let mut decrypted = vec![0u8; input.len()];
        assert!(xchacha20poly1305_decrypt(
            &mut decrypted,
            &output,
            &aad,
            &nonce,
            &key
        ));
        assert_eq!(decrypted, input);
    }

    #[test]
    fn chacha20poly1305_sg_inplace_segments_match_direct_path() {
        let key = [0x42u8; CHACHA20POLY1305_KEY_SIZE];
        let aad = b"segmented aad";
        let nonce = 0x8877_6655_4433_2211u64;
        let plaintext = b"split scatterlist data crosses segment boundaries";
        let mut direct = vec![0u8; plaintext.len() + CHACHA20POLY1305_AUTHTAG_SIZE];
        chacha20poly1305_encrypt(&mut direct, plaintext, aad, nonce, &key);

        let mut first = plaintext[..7].to_vec();
        let mut second = plaintext[7..31].to_vec();
        let mut third = vec![0u8; plaintext.len() - 31 + CHACHA20POLY1305_AUTHTAG_SIZE];
        third[..plaintext.len() - 31].copy_from_slice(&plaintext[31..]);
        {
            let mut segments = [
                first.as_mut_slice(),
                second.as_mut_slice(),
                third.as_mut_slice(),
            ];
            assert!(chacha20poly1305_encrypt_sg_inplace_segments(
                &mut segments,
                plaintext.len(),
                aad,
                nonce,
                &key
            ));
        }
        let mut combined = Vec::new();
        combined.extend_from_slice(&first);
        combined.extend_from_slice(&second);
        combined.extend_from_slice(&third);
        assert_eq!(combined, direct);

        {
            let mut segments = [
                first.as_mut_slice(),
                second.as_mut_slice(),
                third.as_mut_slice(),
            ];
            assert!(chacha20poly1305_decrypt_sg_inplace_segments(
                &mut segments,
                direct.len(),
                aad,
                nonce,
                &key
            ));
        }
        let mut restored = Vec::new();
        restored.extend_from_slice(&first);
        restored.extend_from_slice(&second);
        restored.extend_from_slice(&third[..plaintext.len() - 31]);
        assert_eq!(restored, plaintext);

        third[plaintext.len() - 31 + 3] ^= 1;
        let mut segments = [
            first.as_mut_slice(),
            second.as_mut_slice(),
            third.as_mut_slice(),
        ];
        assert!(!chacha20poly1305_decrypt_sg_inplace_segments(
            &mut segments,
            direct.len(),
            aad,
            nonce,
            &key
        ));
    }
}
