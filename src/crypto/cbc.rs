//! linux-parity: complete
//! linux-source: vendor/linux/crypto/cbc.c
//! test-origin: linux:vendor/linux/crypto/cbc.c
//! CBC block cipher mode wrapper from the Linux Crypto API.

use crate::include::uapi::errno::EINVAL;

pub const CRYPTO_LSKCIPHER_FLAG_FINAL: u32 = 0x0000_0002;
pub const MAX_CIPHER_BLOCKSIZE: usize = 16;
pub const MODULE_DESCRIPTION: &str = "CBC block cipher mode of operation";
pub const MODULE_ALIAS_CRYPTO: &str = "cbc";

pub trait LskcipherBlockCipher {
    fn block_size(&self) -> usize;
    fn encrypt_block(&self, dst: &mut [u8], src: &[u8]);
    fn decrypt_block(&self, dst: &mut [u8], src: &[u8]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CbcInstance {
    pub blocksize: usize,
    pub statesize: usize,
    pub ivsize: usize,
}

pub fn crypto_cbc_create(blocksize: usize, statesize: usize) -> Result<CbcInstance, i32> {
    if !blocksize.is_power_of_two() || blocksize == 0 || blocksize > MAX_CIPHER_BLOCKSIZE {
        return Err(-EINVAL);
    }
    if statesize != 0 {
        return Err(-EINVAL);
    }
    Ok(CbcInstance {
        blocksize,
        statesize,
        ivsize: blocksize,
    })
}

fn xor_assign(dst: &mut [u8], src: &[u8], len: usize) {
    for i in 0..len {
        dst[i] ^= src[i];
    }
}

pub fn crypto_cbc_encrypt_segment<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    nbytes: usize,
    iv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(src.len() >= nbytes);
    assert!(dst.len() >= nbytes);
    assert!(iv.len() >= bsize);

    let mut offset = 0usize;
    while nbytes - offset >= bsize {
        xor_assign(iv, &src[offset..offset + bsize], bsize);
        cipher.encrypt_block(&mut dst[offset..offset + bsize], &iv[..bsize]);
        iv[..bsize].copy_from_slice(&dst[offset..offset + bsize]);
        offset += bsize;
    }
    nbytes - offset
}

pub fn crypto_cbc_encrypt_inplace<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &mut [u8],
    nbytes: usize,
    oiv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(src.len() >= nbytes);
    assert!(oiv.len() >= bsize);

    if nbytes < bsize {
        return nbytes;
    }

    let mut iv = [0u8; MAX_CIPHER_BLOCKSIZE];
    iv[..bsize].copy_from_slice(&oiv[..bsize]);
    let mut offset = 0usize;
    while nbytes - offset >= bsize {
        xor_assign(&mut src[offset..offset + bsize], &iv[..bsize], bsize);
        let mut block = [0u8; MAX_CIPHER_BLOCKSIZE];
        block[..bsize].copy_from_slice(&src[offset..offset + bsize]);
        cipher.encrypt_block(&mut src[offset..offset + bsize], &block[..bsize]);
        iv[..bsize].copy_from_slice(&src[offset..offset + bsize]);
        offset += bsize;
    }
    oiv[..bsize].copy_from_slice(&iv[..bsize]);
    nbytes - offset
}

pub fn crypto_cbc_encrypt<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    len: usize,
    iv: &mut [u8],
    flags: u32,
) -> Result<usize, i32> {
    let rem = crypto_cbc_encrypt_segment(cipher, src, dst, len, iv);
    if rem != 0 && flags & CRYPTO_LSKCIPHER_FLAG_FINAL != 0 {
        Err(-EINVAL)
    } else {
        Ok(rem)
    }
}

pub fn crypto_cbc_decrypt_segment<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    nbytes: usize,
    oiv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(src.len() >= nbytes);
    assert!(dst.len() >= nbytes);
    assert!(oiv.len() >= bsize);

    if nbytes < bsize {
        return nbytes;
    }

    let mut iv = [0u8; MAX_CIPHER_BLOCKSIZE];
    iv[..bsize].copy_from_slice(&oiv[..bsize]);
    let mut offset = 0usize;
    while nbytes - offset >= bsize {
        cipher.decrypt_block(
            &mut dst[offset..offset + bsize],
            &src[offset..offset + bsize],
        );
        xor_assign(&mut dst[offset..offset + bsize], &iv[..bsize], bsize);
        iv[..bsize].copy_from_slice(&src[offset..offset + bsize]);
        offset += bsize;
    }
    oiv[..bsize].copy_from_slice(&iv[..bsize]);
    nbytes - offset
}

pub fn crypto_cbc_decrypt_inplace<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &mut [u8],
    nbytes: usize,
    iv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(src.len() >= nbytes);
    assert!(iv.len() >= bsize);

    if nbytes < bsize {
        return nbytes;
    }

    let rem = nbytes & (bsize - 1);
    let mut offset = nbytes - rem - bsize;
    let mut last_iv = [0u8; MAX_CIPHER_BLOCKSIZE];
    last_iv[..bsize].copy_from_slice(&src[offset..offset + bsize]);

    loop {
        let mut block = [0u8; MAX_CIPHER_BLOCKSIZE];
        block[..bsize].copy_from_slice(&src[offset..offset + bsize]);
        cipher.decrypt_block(&mut src[offset..offset + bsize], &block[..bsize]);
        if offset == 0 {
            break;
        }
        let prev = offset - bsize;
        let mut prev_ct = [0u8; MAX_CIPHER_BLOCKSIZE];
        prev_ct[..bsize].copy_from_slice(&src[prev..offset]);
        xor_assign(&mut src[offset..offset + bsize], &prev_ct[..bsize], bsize);
        offset = prev;
    }

    xor_assign(&mut src[..bsize], iv, bsize);
    iv[..bsize].copy_from_slice(&last_iv[..bsize]);
    rem
}

pub fn crypto_cbc_decrypt<C: LskcipherBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    len: usize,
    iv: &mut [u8],
    flags: u32,
) -> Result<usize, i32> {
    let rem = crypto_cbc_decrypt_segment(cipher, src, dst, len, iv);
    if rem != 0 && flags & CRYPTO_LSKCIPHER_FLAG_FINAL != 0 {
        Err(-EINVAL)
    } else {
        Ok(rem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::crypto::aes::{AesKey, aes_decrypt, aes_encrypt, aes_preparekey};

    struct AesCipher(AesKey);

    impl AesCipher {
        fn new(key: &[u8]) -> Self {
            let mut aes = AesKey::default();
            assert_eq!(aes_preparekey(&mut aes, key), 0);
            Self(aes)
        }
    }

    impl LskcipherBlockCipher for AesCipher {
        fn block_size(&self) -> usize {
            16
        }

        fn encrypt_block(&self, dst: &mut [u8], src: &[u8]) {
            let mut input = [0u8; 16];
            let mut output = [0u8; 16];
            input.copy_from_slice(&src[..16]);
            aes_encrypt(&self.0.enc_key, &mut output, &input);
            dst[..16].copy_from_slice(&output);
        }

        fn decrypt_block(&self, dst: &mut [u8], src: &[u8]) {
            let mut input = [0u8; 16];
            let mut output = [0u8; 16];
            input.copy_from_slice(&src[..16]);
            aes_decrypt(&self.0, &mut output, &input);
            dst[..16].copy_from_slice(&output);
        }
    }

    #[test]
    fn cbc_matches_linux_source_and_aes_cbc_testmgr_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/cbc.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        assert!(source.contains("crypto_cbc_encrypt_segment"));
        assert!(source.contains("crypto_xor(iv, src, bsize);"));
        assert!(source.contains("memcpy(iv, dst, bsize);"));
        assert!(source.contains("crypto_cbc_decrypt_inplace"));
        assert!(source.contains("return rem && final ? -EINVAL : rem;"));
        assert!(source.contains("if (!is_power_of_2(inst->alg.co.base.cra_blocksize))"));
        assert!(source.contains("if (inst->alg.co.statesize)"));
        assert!(testmgr.contains("static const struct cipher_testvec aes_cbc_tv_template[]"));
        assert!(testmgr_c.contains("\"cbc(aes)\""));

        let key = [
            0x8e, 0x73, 0xb0, 0xf7, 0xda, 0x0e, 0x64, 0x52, 0xc8, 0x10, 0xf3, 0x2b, 0x80, 0x90,
            0x79, 0xe5, 0x62, 0xf8, 0xea, 0xd2, 0x52, 0x2c, 0x6b, 0x7b,
        ];
        let mut iv = [
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
            0x4f, 0x02, 0x1d, 0xb2, 0x43, 0xbc, 0x63, 0x3d, 0x71, 0x78, 0x18, 0x3a, 0x9f, 0xa0,
            0x71, 0xe8, 0xb4, 0xd9, 0xad, 0xa9, 0xad, 0x7d, 0xed, 0xf4, 0xe5, 0xe7, 0x38, 0x76,
            0x3f, 0x69, 0x14, 0x5a, 0x57, 0x1b, 0x24, 0x20, 0x12, 0xfb, 0x7a, 0xe0, 0x7f, 0xa9,
            0xba, 0xac, 0x3d, 0xf1, 0x02, 0xe0, 0x08, 0xb0, 0xe2, 0x79, 0x88, 0x59, 0x88, 0x81,
            0xd9, 0x20, 0xa9, 0xe6, 0x4f, 0x56, 0x15, 0xcd,
        ];
        let cipher = AesCipher::new(&key);
        let mut out = [0u8; 64];
        assert_eq!(
            crypto_cbc_encrypt(
                &cipher,
                &ptext,
                &mut out,
                ptext.len(),
                &mut iv,
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Ok(0)
        );
        assert_eq!(out, ctext);
        assert_eq!(
            iv,
            [
                0x08, 0xb0, 0xe2, 0x79, 0x88, 0x59, 0x88, 0x81, 0xd9, 0x20, 0xa9, 0xe6, 0x4f, 0x56,
                0x15, 0xcd,
            ]
        );

        let mut dec_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut plain = [0u8; 64];
        assert_eq!(
            crypto_cbc_decrypt(
                &cipher,
                &ctext,
                &mut plain,
                ctext.len(),
                &mut dec_iv,
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Ok(0)
        );
        assert_eq!(plain, ptext);
        assert_eq!(dec_iv, iv);
    }

    #[test]
    fn cbc_inplace_and_final_remainder_match_linux_contract() {
        let cipher = AesCipher::new(&[0u8; 16]);
        assert_eq!(crypto_cbc_create(16, 0).unwrap().ivsize, 16);
        assert_eq!(crypto_cbc_create(12, 0), Err(-EINVAL));
        assert_eq!(crypto_cbc_create(16, 1), Err(-EINVAL));

        let mut iv = [0u8; 16];
        let input = [0x11u8; 17];
        let mut out = [0u8; 17];
        assert_eq!(
            crypto_cbc_encrypt(&cipher, &input, &mut out, input.len(), &mut iv, 0),
            Ok(1)
        );
        let mut final_iv = [0u8; 16];
        assert_eq!(
            crypto_cbc_encrypt(
                &cipher,
                &input,
                &mut out,
                input.len(),
                &mut final_iv,
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Err(-EINVAL)
        );

        let mut segment_iv = [0x33u8; 16];
        let mut segment = [0u8; 32];
        crypto_cbc_encrypt(&cipher, &[0x44u8; 32], &mut segment, 32, &mut segment_iv, 0).unwrap();
        let mut inplace_iv = [0x33u8; 16];
        let mut inplace = [0x44u8; 32];
        assert_eq!(
            crypto_cbc_encrypt_inplace(&cipher, &mut inplace, 32, &mut inplace_iv),
            0
        );
        assert_eq!(inplace, segment);
        assert_eq!(inplace_iv, segment_iv);

        let mut decrypt_inplace = inplace;
        let mut decrypt_iv = [0x33u8; 16];
        assert_eq!(
            crypto_cbc_decrypt_inplace(&cipher, &mut decrypt_inplace, 32, &mut decrypt_iv),
            0
        );
        assert_eq!(decrypt_inplace, [0x44u8; 32]);
        assert_eq!(decrypt_iv, segment_iv);
    }
}
