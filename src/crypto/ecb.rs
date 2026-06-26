//! linux-parity: complete
//! linux-source: vendor/linux/crypto/ecb.c
//! test-origin: linux:vendor/linux/crypto/ecb.c
//! ECB block cipher mode wrapper from the Linux Crypto API.

use crate::include::uapi::errno::EINVAL;

pub const CRYPTO_LSKCIPHER_FLAG_FINAL: u32 = 0x0000_0002;
pub const MODULE_DESCRIPTION: &str = "ECB block cipher mode of operation";
pub const MODULE_ALIAS_CRYPTO: &str = "ecb";

pub trait EcbBlockCipher {
    fn block_size(&self) -> usize;
    fn encrypt_block(&self, dst: &mut [u8], src: &[u8]);
    fn decrypt_block(&self, dst: &mut [u8], src: &[u8]);
}

pub trait EcbSetkeyCipher {
    fn clear_flags(&mut self, flags: u32);
    fn set_flags(&mut self, flags: u32);
    fn setkey(&mut self, key: &[u8]) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EcbCreatePath {
    Lskcipher,
    CipherFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcbInstance {
    pub path: EcbCreatePath,
    pub ivsize: usize,
    pub blocksize: usize,
    pub ctxsize: usize,
}

pub fn crypto_ecb_create_lskcipher(
    child_ivsize: usize,
    blocksize: usize,
    ctxsize: usize,
) -> Result<EcbInstance, i32> {
    if child_ivsize != 0 {
        return Err(-EINVAL);
    }
    Ok(EcbInstance {
        path: EcbCreatePath::Lskcipher,
        ivsize: 0,
        blocksize,
        ctxsize,
    })
}

pub fn crypto_ecb_create_cipher_fallback(blocksize: usize) -> EcbInstance {
    EcbInstance {
        path: EcbCreatePath::CipherFallback,
        ivsize: 0,
        blocksize,
        ctxsize: core::mem::size_of::<usize>(),
    }
}

pub fn lskcipher_setkey_simple2<C: EcbSetkeyCipher>(
    cipher: &mut C,
    tfm_flags: u32,
    req_mask: u32,
    key: &[u8],
) -> Result<(), i32> {
    cipher.clear_flags(req_mask);
    cipher.set_flags(tfm_flags & req_mask);
    cipher.setkey(key)
}

fn crypto_ecb_crypt<C, F>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    mut nbytes: usize,
    final_block: bool,
    mut crypt: F,
) -> Result<usize, i32>
where
    C: EcbBlockCipher,
    F: FnMut(&C, &mut [u8], &[u8]),
{
    let bsize = cipher.block_size();
    assert!(src.len() >= nbytes);
    assert!(dst.len() >= nbytes);

    let mut offset = 0usize;
    while nbytes >= bsize {
        crypt(
            cipher,
            &mut dst[offset..offset + bsize],
            &src[offset..offset + bsize],
        );
        offset += bsize;
        nbytes -= bsize;
    }

    if nbytes != 0 && final_block {
        Err(-EINVAL)
    } else {
        Ok(nbytes)
    }
}

pub fn crypto_ecb_encrypt<C: EcbBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    len: usize,
    flags: u32,
) -> Result<usize, i32> {
    crypto_ecb_crypt(
        cipher,
        src,
        dst,
        len,
        flags & CRYPTO_LSKCIPHER_FLAG_FINAL != 0,
        |cipher, dst, src| cipher.encrypt_block(dst, src),
    )
}

pub fn crypto_ecb_decrypt<C: EcbBlockCipher>(
    cipher: &C,
    src: &[u8],
    dst: &mut [u8],
    len: usize,
    flags: u32,
) -> Result<usize, i32> {
    crypto_ecb_crypt(
        cipher,
        src,
        dst,
        len,
        flags & CRYPTO_LSKCIPHER_FLAG_FINAL != 0,
        |cipher, dst, src| cipher.decrypt_block(dst, src),
    )
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

    impl EcbBlockCipher for AesCipher {
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

    #[derive(Default)]
    struct FlagCipher {
        flags: u32,
        key_len: usize,
    }

    impl EcbSetkeyCipher for FlagCipher {
        fn clear_flags(&mut self, flags: u32) {
            self.flags &= !flags;
        }

        fn set_flags(&mut self, flags: u32) {
            self.flags |= flags;
        }

        fn setkey(&mut self, key: &[u8]) -> Result<(), i32> {
            self.key_len = key.len();
            Ok(())
        }
    }

    #[test]
    fn ecb_matches_linux_source_and_aes_testmgr_vector() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/ecb.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        assert!(source.contains("static int crypto_ecb_crypt"));
        assert!(source.contains("return nbytes && final ? -EINVAL : nbytes;"));
        assert!(source.contains("lskcipher_setkey_simple2"));
        assert!(source.contains("inst->alg.co.ivsize = 0;"));
        assert!(source.contains("if (cipher_alg->co.ivsize)"));
        assert!(testmgr.contains("static const struct cipher_testvec aes_tv_template[]"));
        assert!(testmgr_c.contains("\"ecb(aes)\""));

        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let ptext = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let ctext = [
            0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
            0xc5, 0x5a,
        ];
        let cipher = AesCipher::new(&key);
        let mut out = [0u8; 16];
        assert_eq!(
            crypto_ecb_encrypt(
                &cipher,
                &ptext,
                &mut out,
                ptext.len(),
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Ok(0)
        );
        assert_eq!(out, ctext);

        let mut plain = [0u8; 16];
        assert_eq!(
            crypto_ecb_decrypt(
                &cipher,
                &ctext,
                &mut plain,
                ctext.len(),
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Ok(0)
        );
        assert_eq!(plain, ptext);
    }

    #[test]
    fn ecb_create_setkey_and_final_remainder_match_linux_contract() {
        assert_eq!(
            crypto_ecb_create_lskcipher(0, 16, 48).unwrap(),
            EcbInstance {
                path: EcbCreatePath::Lskcipher,
                ivsize: 0,
                blocksize: 16,
                ctxsize: 48,
            }
        );
        assert_eq!(crypto_ecb_create_lskcipher(16, 16, 48), Err(-EINVAL));
        assert_eq!(crypto_ecb_create_cipher_fallback(16).ivsize, 0);

        let mut setkey = FlagCipher {
            flags: 0xffff,
            key_len: 0,
        };
        assert_eq!(
            lskcipher_setkey_simple2(&mut setkey, 0xa5, 0x0f, b"key"),
            Ok(())
        );
        assert_eq!(setkey.flags & 0x0f, 0x05);
        assert_eq!(setkey.key_len, 3);

        let cipher = AesCipher::new(&[0u8; 16]);
        let input = [0u8; 17];
        let mut out = [0u8; 17];
        assert_eq!(
            crypto_ecb_encrypt(&cipher, &input, &mut out, input.len(), 0),
            Ok(1)
        );
        assert_eq!(
            crypto_ecb_encrypt(
                &cipher,
                &input,
                &mut out,
                input.len(),
                CRYPTO_LSKCIPHER_FLAG_FINAL
            ),
            Err(-EINVAL)
        );
    }
}
