//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/aescfb.c
//! test-origin: linux:vendor/linux/lib/crypto/aescfb.c
//! AES-CFB encryption and decryption helpers.

use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::crypto::aes::{AES_BLOCK_SIZE, AesEncKey, aes_encrypt};
use crate::lib::crypto::utils::crypto_xor_cpy;

pub const AESCFB_DESCRIPTION: &str = "Generic AES-CFB library";
pub const AESCFB_AUTHOR: &str = "Ard Biesheuvel <ardb@kernel.org>";
pub const AESCFB_LICENSE: &str = "GPL";

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("aescfb_encrypt", aescfb_encrypt_raw as usize, false);
    export_symbol_once("aescfb_decrypt", aescfb_decrypt_raw as usize, false);
}

pub fn aescfb_encrypt(key: &AesEncKey, dst: &mut [u8], src: &[u8], iv: &[u8; AES_BLOCK_SIZE]) {
    assert!(dst.len() >= src.len());
    let mut ks = [0u8; AES_BLOCK_SIZE];
    let mut feedback = *iv;
    let mut offset = 0usize;
    while offset < src.len() {
        aes_encrypt(key, &mut ks, &feedback);
        let take = core::cmp::min(AES_BLOCK_SIZE, src.len() - offset);
        crypto_xor_cpy(
            &mut dst[offset..offset + take],
            &src[offset..offset + take],
            &ks[..take],
        );
        feedback[..take].copy_from_slice(&dst[offset..offset + take]);
        if take < AES_BLOCK_SIZE {
            break;
        }
        offset += AES_BLOCK_SIZE;
    }
    ks.fill(0);
}

pub fn aescfb_decrypt(key: &AesEncKey, dst: &mut [u8], src: &[u8], iv: &[u8; AES_BLOCK_SIZE]) {
    assert!(dst.len() >= src.len());
    let mut ks = [0u8; AES_BLOCK_SIZE];
    let mut feedback = *iv;
    let mut offset = 0usize;
    while offset < src.len() {
        aes_encrypt(key, &mut ks, &feedback);
        let take = core::cmp::min(AES_BLOCK_SIZE, src.len() - offset);
        let mut next_feedback = feedback;
        next_feedback[..take].copy_from_slice(&src[offset..offset + take]);
        crypto_xor_cpy(
            &mut dst[offset..offset + take],
            &src[offset..offset + take],
            &ks[..take],
        );
        feedback = next_feedback;
        if take < AES_BLOCK_SIZE {
            break;
        }
        offset += AES_BLOCK_SIZE;
    }
    ks.fill(0);
}

pub fn aescfb_encrypt_in_place(key: &AesEncKey, buf: &mut [u8], iv: &[u8; AES_BLOCK_SIZE]) {
    let mut ks = [0u8; AES_BLOCK_SIZE];
    let mut feedback = *iv;
    let mut offset = 0usize;
    while offset < buf.len() {
        aes_encrypt(key, &mut ks, &feedback);
        let take = core::cmp::min(AES_BLOCK_SIZE, buf.len() - offset);
        for i in 0..take {
            buf[offset + i] ^= ks[i];
        }
        feedback[..take].copy_from_slice(&buf[offset..offset + take]);
        if take < AES_BLOCK_SIZE {
            break;
        }
        offset += AES_BLOCK_SIZE;
    }
    ks.fill(0);
}

pub fn aescfb_decrypt_in_place(key: &AesEncKey, buf: &mut [u8], iv: &[u8; AES_BLOCK_SIZE]) {
    let mut ks = [0u8; AES_BLOCK_SIZE];
    let mut feedback = *iv;
    let mut offset = 0usize;
    while offset < buf.len() {
        aes_encrypt(key, &mut ks, &feedback);
        let take = core::cmp::min(AES_BLOCK_SIZE, buf.len() - offset);
        let mut next_feedback = feedback;
        next_feedback[..take].copy_from_slice(&buf[offset..offset + take]);
        for i in 0..take {
            buf[offset + i] ^= ks[i];
        }
        feedback = next_feedback;
        if take < AES_BLOCK_SIZE {
            break;
        }
        offset += AES_BLOCK_SIZE;
    }
    ks.fill(0);
}

pub unsafe extern "C" fn aescfb_encrypt_raw(
    key: *const AesEncKey,
    dst: *mut u8,
    src: *const u8,
    len: i32,
    iv: *const u8,
) {
    if key.is_null() || dst.is_null() || src.is_null() || iv.is_null() || len <= 0 {
        return;
    }
    let len = len as usize;
    let iv = unsafe { &*(iv as *const [u8; AES_BLOCK_SIZE]) };
    if core::ptr::eq(dst, src as *mut u8) {
        let buf = unsafe { core::slice::from_raw_parts_mut(dst, len) };
        unsafe { aescfb_encrypt_in_place(&*key, buf, iv) };
    } else {
        let dst = unsafe { core::slice::from_raw_parts_mut(dst, len) };
        let src = unsafe { core::slice::from_raw_parts(src, len) };
        unsafe { aescfb_encrypt(&*key, dst, src, iv) };
    }
}

pub unsafe extern "C" fn aescfb_decrypt_raw(
    key: *const AesEncKey,
    dst: *mut u8,
    src: *const u8,
    len: i32,
    iv: *const u8,
) {
    if key.is_null() || dst.is_null() || src.is_null() || iv.is_null() || len <= 0 {
        return;
    }
    let len = len as usize;
    let iv = unsafe { &*(iv as *const [u8; AES_BLOCK_SIZE]) };
    if core::ptr::eq(dst, src as *mut u8) {
        let buf = unsafe { core::slice::from_raw_parts_mut(dst, len) };
        unsafe { aescfb_decrypt_in_place(&*key, buf, iv) };
    } else {
        let dst = unsafe { core::slice::from_raw_parts_mut(dst, len) };
        let src = unsafe { core::slice::from_raw_parts(src, len) };
        unsafe { aescfb_decrypt(&*key, dst, src, iv) };
    }
}

pub fn libaescfb_init() -> Result<(), i32> {
    #[cfg(test)]
    {
        aescfb_selftest()
    }
    #[cfg(not(test))]
    {
        Ok(())
    }
}

pub const fn libaescfb_exit() {}

#[cfg(test)]
#[derive(Clone, Copy)]
struct AescfbTestVector {
    key: [u8; 32],
    iv: [u8; AES_BLOCK_SIZE],
    ptext: [u8; 64],
    ctext: [u8; 64],
    klen: usize,
    len: usize,
}

#[cfg(test)]
const fn padded_key(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < bytes.len() {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[cfg(test)]
const fn padded_block(bytes: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < bytes.len() {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[cfg(test)]
const AESCFB_TEST_VECTORS: &[AescfbTestVector] = &[
    AescfbTestVector {
        key: padded_key(&[
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ]),
        iv: [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
        ptext: padded_block(&[
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae,
        ]),
        ctext: padded_block(&[
            0x3b, 0x3f, 0xd9, 0x2e, 0xb7, 0x2d, 0xad, 0x20, 0x33, 0x34, 0x49, 0xf8, 0xe8, 0x3c,
            0xfb, 0x4a, 0xc8,
        ]),
        klen: 16,
        len: 17,
    },
    AescfbTestVector {
        key: padded_key(&[
            0x8e, 0x73, 0xb0, 0xf7, 0xda, 0x0e, 0x64, 0x52, 0xc8, 0x10, 0xf3, 0x2b, 0x80, 0x90,
            0x79, 0xe5, 0x62, 0xf8, 0xea, 0xd2, 0x52, 0x2c, 0x6b, 0x7b,
        ]),
        iv: [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
        ptext: padded_block(&[
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ]),
        ctext: padded_block(&[
            0xcd, 0xc8, 0x0d, 0x6f, 0xdd, 0xf1, 0x8c, 0xab, 0x34, 0xc2, 0x59, 0x09, 0xc9, 0x9a,
            0x41, 0x74,
        ]),
        klen: 24,
        len: 16,
    },
    AescfbTestVector {
        key: padded_key(&[
            0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe, 0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d,
            0x77, 0x81, 0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7, 0x2d, 0x98, 0x10, 0xa3,
            0x09, 0x14, 0xdf, 0xf4,
        ]),
        iv: [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
        ptext: padded_block(&[0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f]),
        ctext: padded_block(&[0xdc, 0x7e, 0x84, 0xbf, 0xda, 0x79, 0x16]),
        klen: 32,
        len: 7,
    },
];

#[cfg(test)]
fn aescfb_selftest() -> Result<(), i32> {
    use crate::include::uapi::errno::ENODEV;
    use crate::lib::crypto::aes::aes_prepareenckey;

    for tv in AESCFB_TEST_VECTORS {
        let mut key = AesEncKey::default();
        if aes_prepareenckey(&mut key, &tv.key[..tv.klen]) != 0 {
            return Err(-ENODEV);
        }

        let mut buf = [0u8; 64];
        aescfb_encrypt(&key, &mut buf[..tv.len], &tv.ptext[..tv.len], &tv.iv);
        if buf[..tv.len] != tv.ctext[..tv.len] {
            return Err(-ENODEV);
        }

        aescfb_decrypt_in_place(&key, &mut buf[..tv.len], &tv.iv);
        if buf[..tv.len] != tv.ptext[..tv.len] {
            return Err(-ENODEV);
        }

        buf[..tv.len].copy_from_slice(&tv.ptext[..tv.len]);
        aescfb_encrypt_in_place(&key, &mut buf[..tv.len], &tv.iv);
        if buf[..tv.len] != tv.ctext[..tv.len] {
            return Err(-ENODEV);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::crypto::aes::aes_prepareenckey;

    #[test]
    fn aescfb_matches_linux_selftest_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/aescfb.c"
        ));
        assert!(source.contains("Test code below. Vectors taken from crypto/testmgr.h"));
        assert!(source.contains("aescfb_encrypt(&key, buf, aescfb_tv[i].ptext"));
        assert!(source.contains("aescfb_decrypt(&key, buf, buf, aescfb_tv[i].len"));
        assert!(source.contains("EXPORT_SYMBOL(aescfb_encrypt);"));
        assert!(source.contains("EXPORT_SYMBOL(aescfb_decrypt);"));
        assert!(source.contains("memzero_explicit(ks, sizeof(ks));"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Generic AES-CFB library\");"));
        assert!(source.contains("MODULE_AUTHOR(\"Ard Biesheuvel <ardb@kernel.org>\");"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));
        assert!(source.contains("module_init(libaescfb_init);"));
        assert!(source.contains("module_exit(libaescfb_exit);"));
        assert_eq!(AESCFB_DESCRIPTION, "Generic AES-CFB library");
        assert_eq!(AESCFB_AUTHOR, "Ard Biesheuvel <ardb@kernel.org>");
        assert_eq!(AESCFB_LICENSE, "GPL");
        assert_eq!(libaescfb_init(), Ok(()));
        libaescfb_exit();

        let iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let ptext = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51, 0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11, 0xe5, 0xfb,
            0xc1, 0x19, 0x1a, 0x0a, 0x52, 0xef, 0xf6, 0x9f, 0x24, 0x45, 0xdf, 0x4f, 0x9b, 0x17,
            0xad, 0x2b, 0x41, 0x7b, 0xe6, 0x6c, 0x37, 0x10,
        ];
        let ctext = [
            0x3b, 0x3f, 0xd9, 0x2e, 0xb7, 0x2d, 0xad, 0x20, 0x33, 0x34, 0x49, 0xf8, 0xe8, 0x3c,
            0xfb, 0x4a, 0xc8, 0xa6, 0x45, 0x37, 0xa0, 0xb3, 0xa9, 0x3f, 0xcd, 0xe3, 0xcd, 0xad,
            0x9f, 0x1c, 0xe5, 0x8b, 0x26, 0x75, 0x1f, 0x67, 0xa3, 0xcb, 0xb1, 0x40, 0xb1, 0x80,
            0x8c, 0xf1, 0x87, 0xa4, 0xf4, 0xdf, 0xc0, 0x4b, 0x05, 0x35, 0x7c, 0x5d, 0x1c, 0x0e,
            0xea, 0xc4, 0xc6, 0x6f, 0x9f, 0xf7, 0xf2, 0xe6,
        ];
        let key_bytes = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let mut key = AesEncKey::default();
        assert_eq!(aes_prepareenckey(&mut key, &key_bytes), 0);
        let mut out = [0u8; 64];
        aescfb_encrypt(&key, &mut out, &ptext, &iv);
        assert_eq!(out, ctext);
        let mut plain = [0u8; 64];
        aescfb_decrypt(&key, &mut plain, &out, &iv);
        assert_eq!(plain, ptext);

        let mut short = [0u8; 17];
        aescfb_encrypt(&key, &mut short, &ptext[..17], &iv);
        assert_eq!(&short, &ctext[..17]);

        let mut inplace = ptext;
        aescfb_encrypt_in_place(&key, &mut inplace, &iv);
        assert_eq!(inplace, ctext);
        unsafe {
            aescfb_decrypt_raw(
                &key,
                inplace.as_mut_ptr(),
                inplace.as_mut_ptr(),
                inplace.len() as i32,
                iv.as_ptr(),
            );
        }
        assert_eq!(inplace, ptext);
    }
}
