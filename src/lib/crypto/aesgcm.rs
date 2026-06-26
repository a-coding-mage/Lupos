//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/aesgcm.c
//! test-origin: linux:vendor/linux/lib/crypto/aesgcm.c
//! Generic AES-GCM helper.

use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::crypto::aes::{AES_BLOCK_SIZE, AesEncKey, EINVAL, aes_encrypt, aes_prepareenckey};
use crate::lib::crypto::gf128mul::ghash_mul;
use crate::lib::crypto::utils::crypto_xor_cpy;

pub const GCM_AES_IV_SIZE: usize = 12;
pub const GHASH_BLOCK_SIZE: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesGcmCtx {
    pub ghash_key: [u8; GHASH_BLOCK_SIZE],
    pub aes_key: AesEncKey,
    pub authsize: u32,
}

impl Default for AesGcmCtx {
    fn default() -> Self {
        Self {
            ghash_key: [0; GHASH_BLOCK_SIZE],
            aes_key: AesEncKey::default(),
            authsize: 0,
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("aesgcm_expandkey", aesgcm_expandkey_raw as usize, false);
    export_symbol_once("aesgcm_encrypt", aesgcm_encrypt_raw as usize, false);
    export_symbol_once("aesgcm_decrypt", aesgcm_decrypt_raw as usize, false);
}

pub const fn crypto_gcm_check_authsize(authsize: usize) -> i32 {
    match authsize {
        4 | 8 | 12 | 13 | 14 | 15 | 16 => 0,
        _ => EINVAL,
    }
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

pub fn aesgcm_expandkey(ctx: &mut AesGcmCtx, key: &[u8], authsize: usize) -> i32 {
    let mut ret = crypto_gcm_check_authsize(authsize);
    if ret != 0 {
        return ret;
    }
    ret = aes_prepareenckey(&mut ctx.aes_key, key);
    if ret != 0 {
        return ret;
    }
    ctx.authsize = authsize as u32;
    let zero = [0u8; AES_BLOCK_SIZE];
    aes_encrypt(&ctx.aes_key, &mut ctx.ghash_key, &zero);
    0
}

fn ghash_update_block(y: &mut [u8; GHASH_BLOCK_SIZE], h: &[u8; GHASH_BLOCK_SIZE], block: &[u8]) {
    for i in 0..block.len() {
        y[i] ^= block[i];
    }
    ghash_mul(y, h);
}

fn ghash_update_padded(
    y: &mut [u8; GHASH_BLOCK_SIZE],
    h: &[u8; GHASH_BLOCK_SIZE],
    mut data: &[u8],
) {
    while data.len() >= GHASH_BLOCK_SIZE {
        ghash_update_block(y, h, &data[..GHASH_BLOCK_SIZE]);
        data = &data[GHASH_BLOCK_SIZE..];
    }
    if !data.is_empty() {
        let mut block = [0u8; GHASH_BLOCK_SIZE];
        block[..data.len()].copy_from_slice(data);
        ghash_update_block(y, h, &block);
    }
}

fn aesgcm_mac(ctx: &AesGcmCtx, src: &[u8], assoc: &[u8], iv: &[u8; GCM_AES_IV_SIZE]) -> [u8; 16] {
    let mut y = [0u8; GHASH_BLOCK_SIZE];
    ghash_update_padded(&mut y, &ctx.ghash_key, assoc);
    ghash_update_padded(&mut y, &ctx.ghash_key, src);

    let mut lens = [0u8; GHASH_BLOCK_SIZE];
    lens[..8].copy_from_slice(&((assoc.len() as u64) * 8).to_be_bytes());
    lens[8..].copy_from_slice(&((src.len() as u64) * 8).to_be_bytes());
    ghash_update_block(&mut y, &ctx.ghash_key, &lens);

    let mut j0 = [0u8; AES_BLOCK_SIZE];
    j0[..GCM_AES_IV_SIZE].copy_from_slice(iv);
    j0[12..16].copy_from_slice(&1u32.to_be_bytes());
    let mut enc_ctr = [0u8; AES_BLOCK_SIZE];
    aes_encrypt(&ctx.aes_key, &mut enc_ctr, &j0);
    for i in 0..GHASH_BLOCK_SIZE {
        y[i] ^= enc_ctr[i];
    }
    y
}

fn aesgcm_crypt(ctx: &AesGcmCtx, dst: &mut [u8], src: &[u8], iv: &[u8; GCM_AES_IV_SIZE]) {
    assert!(dst.len() >= src.len());
    let mut ctr = [0u8; AES_BLOCK_SIZE];
    ctr[..GCM_AES_IV_SIZE].copy_from_slice(iv);
    let mut block_counter = 2u32;
    let mut offset = 0usize;
    while offset < src.len() {
        ctr[12..16].copy_from_slice(&block_counter.to_be_bytes());
        block_counter = block_counter.wrapping_add(1);
        let mut stream = [0u8; AES_BLOCK_SIZE];
        aes_encrypt(&ctx.aes_key, &mut stream, &ctr);
        let take = core::cmp::min(AES_BLOCK_SIZE, src.len() - offset);
        crypto_xor_cpy(
            &mut dst[offset..offset + take],
            &src[offset..offset + take],
            &stream[..take],
        );
        offset += take;
    }
}

pub fn aesgcm_encrypt(
    ctx: &AesGcmCtx,
    dst: &mut [u8],
    src: &[u8],
    assoc: &[u8],
    iv: &[u8; GCM_AES_IV_SIZE],
    authtag: &mut [u8],
) {
    assert!(dst.len() >= src.len());
    assert!(authtag.len() >= ctx.authsize as usize);
    aesgcm_crypt(ctx, dst, src, iv);
    let tag = aesgcm_mac(ctx, &dst[..src.len()], assoc, iv);
    authtag[..ctx.authsize as usize].copy_from_slice(&tag[..ctx.authsize as usize]);
}

pub fn aesgcm_decrypt(
    ctx: &AesGcmCtx,
    dst: &mut [u8],
    src: &[u8],
    assoc: &[u8],
    iv: &[u8; GCM_AES_IV_SIZE],
    authtag: &[u8],
) -> bool {
    assert!(dst.len() >= src.len());
    assert!(authtag.len() >= ctx.authsize as usize);
    let tag = aesgcm_mac(ctx, src, assoc, iv);
    if consttime_ne(
        &tag[..ctx.authsize as usize],
        &authtag[..ctx.authsize as usize],
    ) {
        return false;
    }
    aesgcm_crypt(ctx, dst, src, iv);
    true
}

pub unsafe extern "C" fn aesgcm_expandkey_raw(
    ctx: *mut AesGcmCtx,
    key: *const u8,
    keysize: u32,
    authsize: u32,
) -> i32 {
    if ctx.is_null() || key.is_null() {
        return EINVAL;
    }
    let key = unsafe { core::slice::from_raw_parts(key, keysize as usize) };
    unsafe { aesgcm_expandkey(&mut *ctx, key, authsize as usize) }
}

pub unsafe extern "C" fn aesgcm_encrypt_raw(
    ctx: *const AesGcmCtx,
    dst: *mut u8,
    src: *const u8,
    crypt_len: i32,
    assoc: *const u8,
    assoc_len: i32,
    iv: *const u8,
    authtag: *mut u8,
) {
    if ctx.is_null()
        || dst.is_null()
        || src.is_null()
        || iv.is_null()
        || authtag.is_null()
        || crypt_len < 0
        || assoc_len < 0
    {
        return;
    }
    let crypt_len = crypt_len as usize;
    let assoc_len = assoc_len as usize;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, crypt_len) };
    let src = unsafe { core::slice::from_raw_parts(src, crypt_len) };
    let assoc = if assoc_len == 0 {
        &[]
    } else if assoc.is_null() {
        return;
    } else {
        unsafe { core::slice::from_raw_parts(assoc, assoc_len) }
    };
    let iv = unsafe { &*(iv as *const [u8; GCM_AES_IV_SIZE]) };
    let tag_len = unsafe { (*ctx).authsize as usize };
    let authtag = unsafe { core::slice::from_raw_parts_mut(authtag, tag_len) };
    unsafe { aesgcm_encrypt(&*ctx, dst, src, assoc, iv, authtag) };
}

pub unsafe extern "C" fn aesgcm_decrypt_raw(
    ctx: *const AesGcmCtx,
    dst: *mut u8,
    src: *const u8,
    crypt_len: i32,
    assoc: *const u8,
    assoc_len: i32,
    iv: *const u8,
    authtag: *const u8,
) -> bool {
    if ctx.is_null()
        || dst.is_null()
        || src.is_null()
        || iv.is_null()
        || authtag.is_null()
        || crypt_len < 0
        || assoc_len < 0
    {
        return false;
    }
    let crypt_len = crypt_len as usize;
    let assoc_len = assoc_len as usize;
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, crypt_len) };
    let src = unsafe { core::slice::from_raw_parts(src, crypt_len) };
    let assoc = if assoc_len == 0 {
        &[]
    } else if assoc.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(assoc, assoc_len) }
    };
    let iv = unsafe { &*(iv as *const [u8; GCM_AES_IV_SIZE]) };
    let tag_len = unsafe { (*ctx).authsize as usize };
    let authtag = unsafe { core::slice::from_raw_parts(authtag, tag_len) };
    unsafe { aesgcm_decrypt(&*ctx, dst, src, assoc, iv, authtag) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aesgcm_matches_linux_selftest_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/aesgcm.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/gcm.h"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        assert!(source.contains("static const u8 __initconst ctext0[16]"));
        assert!(source.contains("aesgcm_encrypt(&ctx, buf, buf, plen"));
        assert!(source.contains("aesgcm_decrypt(&ctx, buf, aesgcm_tv[i].ctext, plen"));
        assert!(source.contains("EXPORT_SYMBOL(aesgcm_expandkey);"));
        assert!(header.contains("#define GCM_AES_IV_SIZE 12"));
        assert!(header.contains("static inline int crypto_gcm_check_authsize"));
        assert!(testmgr.contains("static const struct aead_testvec aes_gcm_tv_template[]"));

        let mut ctx = AesGcmCtx::default();
        assert_eq!(aesgcm_expandkey(&mut ctx, &[0u8; 16], 16), 0);
        let iv = [0u8; GCM_AES_IV_SIZE];
        let mut tag = [0u8; 16];
        let mut empty = [];
        aesgcm_encrypt(&ctx, &mut empty, &[], &[], &iv, &mut tag);
        assert_eq!(
            tag,
            [
                0x58, 0xe2, 0xfc, 0xce, 0xfa, 0x7e, 0x30, 0x61, 0x36, 0x7f, 0x1d, 0x57, 0xa4, 0xe7,
                0x45, 0x5a,
            ]
        );

        let ptext = [0u8; 16];
        let mut ctext = [0u8; 16];
        aesgcm_encrypt(&ctx, &mut ctext, &ptext, &[], &iv, &mut tag);
        assert_eq!(
            ctext,
            [
                0x03, 0x88, 0xda, 0xce, 0x60, 0xb6, 0xa3, 0x92, 0xf3, 0x28, 0xc2, 0xb9, 0x71, 0xb2,
                0xfe, 0x78,
            ]
        );
        assert_eq!(
            tag,
            [
                0xab, 0x6e, 0x47, 0xd4, 0x2c, 0xec, 0x13, 0xbd, 0xf5, 0x3a, 0x67, 0xb2, 0x12, 0x57,
                0xbd, 0xdf,
            ]
        );
        let mut decrypted = [0u8; 16];
        assert!(aesgcm_decrypt(&ctx, &mut decrypted, &ctext, &[], &iv, &tag));
        assert_eq!(decrypted, ptext);
        tag[0] ^= 1;
        assert!(!aesgcm_decrypt(
            &ctx,
            &mut decrypted,
            &ctext,
            &[],
            &iv,
            &tag
        ));
    }

    #[test]
    fn aesgcm_matches_linux_aad_and_nonzero_iv_selftest_vector() {
        let key = [
            0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f, 0x94, 0x67, 0x30,
            0x83, 0x08,
        ];
        let iv = [
            0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8, 0x88,
        ];
        let assoc = [
            0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad,
            0xbe, 0xef, 0xab, 0xad, 0xda, 0xd2,
        ];
        let plaintext = [
            0xd9, 0x31, 0x32, 0x25, 0xf8, 0x84, 0x06, 0xe5, 0xa5, 0x59, 0x09, 0xc5, 0xaf, 0xf5,
            0x26, 0x9a, 0x86, 0xa7, 0xa9, 0x53, 0x15, 0x34, 0xf7, 0xda, 0x2e, 0x4c, 0x30, 0x3d,
            0x8a, 0x31, 0x8a, 0x72, 0x1c, 0x3c, 0x0c, 0x95, 0x95, 0x68, 0x09, 0x53, 0x2f, 0xcf,
            0x0e, 0x24, 0x49, 0xa6, 0xb5, 0x25, 0xb1, 0x6a, 0xed, 0xf5, 0xaa, 0x0d, 0xe6, 0x57,
            0xba, 0x63, 0x7b, 0x39,
        ];
        let expected_ctext = [
            0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24, 0x4b, 0x72, 0x21, 0xb7, 0x84, 0xd0,
            0xd4, 0x9c, 0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02, 0xa4, 0xe0, 0x35, 0xc1, 0x7e, 0x23,
            0x29, 0xac, 0xa1, 0x2e, 0x21, 0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c, 0x7d, 0x8f,
            0x6a, 0x5a, 0xac, 0x84, 0xaa, 0x05, 0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac, 0x97,
            0x3d, 0x58, 0xe0, 0x91,
        ];
        let expected_tag = [
            0x5b, 0xc9, 0x4f, 0xbc, 0x32, 0x21, 0xa5, 0xdb, 0x94, 0xfa, 0xe9, 0x5a, 0xe7, 0x12,
            0x1a, 0x47,
        ];

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/aesgcm.c"
        ));
        assert!(source.contains(".assoc\t= \"\\xfe\\xed\\xfa\\xce\\xde\\xad\\xbe\\xef\""));
        assert!(source.contains("ghash_update(&ghash, zeroes, -assoc_len &"));
        assert!(source.contains("ctr[3] = cpu_to_be32(1);"));
        assert!(source.contains("ctr[3] = cpu_to_be32(n++);"));

        let mut ctx = AesGcmCtx::default();
        assert_eq!(aesgcm_expandkey(&mut ctx, &key, 16), 0);
        let mut ciphertext = [0u8; 60];
        let mut tag = [0u8; 16];
        aesgcm_encrypt(&ctx, &mut ciphertext, &plaintext, &assoc, &iv, &mut tag);
        assert_eq!(ciphertext, expected_ctext);
        assert_eq!(tag, expected_tag);

        let mut decrypted = [0u8; 60];
        assert!(aesgcm_decrypt(
            &ctx,
            &mut decrypted,
            &ciphertext,
            &assoc,
            &iv,
            &tag
        ));
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aesgcm_authsize_and_raw_exports_match_linux_contract() {
        for authsize in [4usize, 8, 12, 13, 14, 15, 16] {
            assert_eq!(crypto_gcm_check_authsize(authsize), 0);
            let mut ctx = AesGcmCtx::default();
            assert_eq!(aesgcm_expandkey(&mut ctx, &[0u8; 16], authsize), 0);
            assert_eq!(ctx.authsize, authsize as u32);
        }
        for authsize in [0usize, 1, 2, 3, 5, 6, 7, 9, 10, 11, 17] {
            assert_eq!(crypto_gcm_check_authsize(authsize), EINVAL);
        }

        let key = [0x11u8; 16];
        let iv = [0x22u8; GCM_AES_IV_SIZE];
        let assoc = b"raw assoc";
        let plaintext = b"raw exported AES-GCM path";
        let mut ctx = AesGcmCtx::default();
        unsafe {
            assert_eq!(
                aesgcm_expandkey_raw(&mut ctx, key.as_ptr(), key.len() as u32, 12),
                0
            );
        }

        let mut ciphertext = [0u8; 25];
        let mut tag = [0u8; 12];
        unsafe {
            aesgcm_encrypt_raw(
                &ctx,
                ciphertext.as_mut_ptr(),
                plaintext.as_ptr(),
                plaintext.len() as i32,
                assoc.as_ptr(),
                assoc.len() as i32,
                iv.as_ptr(),
                tag.as_mut_ptr(),
            );
        }

        let mut decrypted = [0u8; 25];
        unsafe {
            assert!(aesgcm_decrypt_raw(
                &ctx,
                decrypted.as_mut_ptr(),
                ciphertext.as_ptr(),
                ciphertext.len() as i32,
                assoc.as_ptr(),
                assoc.len() as i32,
                iv.as_ptr(),
                tag.as_ptr(),
            ));
        }
        assert_eq!(&decrypted, plaintext);

        tag[0] ^= 1;
        decrypted.fill(0xa5);
        unsafe {
            assert!(!aesgcm_decrypt_raw(
                &ctx,
                decrypted.as_mut_ptr(),
                ciphertext.as_ptr(),
                ciphertext.len() as i32,
                assoc.as_ptr(),
                assoc.len() as i32,
                iv.as_ptr(),
                tag.as_ptr(),
            ));
        }
        assert_eq!(decrypted, [0xa5; 25]);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("aesgcm_expandkey"),
            Some(aesgcm_expandkey_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("aesgcm_encrypt"),
            Some(aesgcm_encrypt_raw as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("aesgcm_decrypt"),
            Some(aesgcm_decrypt_raw as usize)
        );
    }
}
