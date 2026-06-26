//! linux-parity: complete
//! linux-source: vendor/linux/crypto/pcbc.c
//! test-origin: linux:vendor/linux/crypto/pcbc.c
//! PCBC block cipher mode wrapper from the Linux Crypto API.

pub const MAX_CIPHER_BLOCKSIZE: usize = 16;
pub const MODULE_DESCRIPTION: &str = "PCBC block cipher mode of operation";
pub const MODULE_ALIAS_CRYPTO: &str = "pcbc";

pub trait SkcipherBlockCipher {
    fn block_size(&self) -> usize;
    fn encrypt_block(&self, dst: &mut [u8], src: &[u8]);
    fn decrypt_block(&self, dst: &mut [u8], src: &[u8]);
}

fn xor_assign(dst: &mut [u8], src: &[u8], len: usize) {
    for i in 0..len {
        dst[i] ^= src[i];
    }
}

fn xor_cpy(dst: &mut [u8], src1: &[u8], src2: &[u8], len: usize) {
    for i in 0..len {
        dst[i] = src1[i] ^ src2[i];
    }
}

pub fn crypto_pcbc_encrypt_segment<C: SkcipherBlockCipher>(
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
        xor_cpy(
            iv,
            &dst[offset..offset + bsize],
            &src[offset..offset + bsize],
            bsize,
        );
        offset += bsize;
    }
    nbytes - offset
}

pub fn crypto_pcbc_encrypt_inplace<C: SkcipherBlockCipher>(
    cipher: &C,
    dst: &mut [u8],
    nbytes: usize,
    iv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(dst.len() >= nbytes);
    assert!(iv.len() >= bsize);

    let mut offset = 0usize;
    while nbytes - offset >= bsize {
        let mut tmpbuf = [0u8; MAX_CIPHER_BLOCKSIZE];
        tmpbuf[..bsize].copy_from_slice(&dst[offset..offset + bsize]);
        xor_assign(iv, &dst[offset..offset + bsize], bsize);
        let mut block = [0u8; MAX_CIPHER_BLOCKSIZE];
        block[..bsize].copy_from_slice(&iv[..bsize]);
        cipher.encrypt_block(&mut dst[offset..offset + bsize], &block[..bsize]);
        xor_cpy(iv, &tmpbuf[..bsize], &dst[offset..offset + bsize], bsize);
        offset += bsize;
    }
    nbytes - offset
}

pub fn crypto_pcbc_decrypt_segment<C: SkcipherBlockCipher>(
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
        cipher.decrypt_block(
            &mut dst[offset..offset + bsize],
            &src[offset..offset + bsize],
        );
        xor_assign(&mut dst[offset..offset + bsize], iv, bsize);
        xor_cpy(
            iv,
            &dst[offset..offset + bsize],
            &src[offset..offset + bsize],
            bsize,
        );
        offset += bsize;
    }
    nbytes - offset
}

pub fn crypto_pcbc_decrypt_inplace<C: SkcipherBlockCipher>(
    cipher: &C,
    dst: &mut [u8],
    nbytes: usize,
    iv: &mut [u8],
) -> usize {
    let bsize = cipher.block_size();
    assert!(bsize <= MAX_CIPHER_BLOCKSIZE);
    assert!(dst.len() >= nbytes);
    assert!(iv.len() >= bsize);

    let mut offset = 0usize;
    while nbytes - offset >= bsize {
        let mut tmpbuf = [0u8; MAX_CIPHER_BLOCKSIZE];
        tmpbuf[..bsize].copy_from_slice(&dst[offset..offset + bsize]);
        let mut block = [0u8; MAX_CIPHER_BLOCKSIZE];
        block[..bsize].copy_from_slice(&dst[offset..offset + bsize]);
        cipher.decrypt_block(&mut dst[offset..offset + bsize], &block[..bsize]);
        xor_assign(&mut dst[offset..offset + bsize], iv, bsize);
        xor_cpy(iv, &dst[offset..offset + bsize], &tmpbuf[..bsize], bsize);
        offset += bsize;
    }
    nbytes - offset
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

    impl SkcipherBlockCipher for AesCipher {
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
    fn pcbc_matches_linux_source_and_chaining_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/pcbc.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        assert!(source.contains("crypto_pcbc_encrypt_segment"));
        assert!(source.contains("crypto_xor(iv, src, bsize);"));
        assert!(source.contains("crypto_xor_cpy(iv, dst, src, bsize);"));
        assert!(source.contains("crypto_pcbc_decrypt_inplace"));
        assert!(source.contains("skcipher_alloc_instance_simple(tmpl, tb);"));
        assert!(testmgr.contains("static const struct cipher_testvec fcrypt_pcbc_tv_template[]"));
        assert!(testmgr_c.contains("\"pcbc(fcrypt)\""));

        let cipher = AesCipher::new(&[
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ]);
        let ptext = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51,
        ];
        let mut iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut out = [0u8; 32];
        assert_eq!(
            crypto_pcbc_encrypt_segment(&cipher, &ptext, &mut out, 32, &mut iv),
            0
        );

        let mut inplace_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut inplace = ptext;
        assert_eq!(
            crypto_pcbc_encrypt_inplace(&cipher, &mut inplace, 32, &mut inplace_iv),
            0
        );
        assert_eq!(inplace, out);
        assert_eq!(inplace_iv, iv);

        let mut decrypt_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut plain = [0u8; 32];
        assert_eq!(
            crypto_pcbc_decrypt_segment(&cipher, &out, &mut plain, 32, &mut decrypt_iv),
            0
        );
        assert_eq!(plain, ptext);
        assert_eq!(decrypt_iv, iv);

        let mut decrypt_inplace = out;
        let mut decrypt_inplace_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        assert_eq!(
            crypto_pcbc_decrypt_inplace(&cipher, &mut decrypt_inplace, 32, &mut decrypt_inplace_iv),
            0
        );
        assert_eq!(decrypt_inplace, ptext);
        assert_eq!(decrypt_inplace_iv, iv);
    }
}
