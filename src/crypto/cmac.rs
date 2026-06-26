//! linux-parity: complete
//! linux-source: vendor/linux/crypto/cmac.c
//! test-origin: linux:vendor/linux/crypto/cmac.c
//! CMAC keyed hash wrapper from the Linux Crypto API.

use crate::include::uapi::errno::EINVAL;

pub const CRYPTO_AHASH_ALG_BLOCK_ONLY: u32 = 0x0100_0000;
pub const CRYPTO_AHASH_ALG_FINAL_NONZERO: u32 = 0x0200_0000;
pub const MODULE_DESCRIPTION: &str = "CMAC keyed hash algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "cmac";

pub trait CmacBlockCipher {
    fn block_size(&self) -> usize;
    fn setkey(&mut self, key: &[u8]) -> Result<(), i32>;
    fn encrypt_block(&self, dst: &mut [u8], src: &[u8]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CmacInstance {
    pub blocksize: usize,
    pub digestsize: usize,
    pub descsize: usize,
    pub cra_flags: u32,
}

#[derive(Clone, Debug)]
pub struct CmacCtx<C> {
    pub child: C,
    pub consts: [u8; 32],
}

pub fn cmac_create(blocksize: usize) -> Result<CmacInstance, i32> {
    match blocksize {
        8 | 16 => Ok(CmacInstance {
            blocksize,
            digestsize: blocksize,
            descsize: blocksize,
            cra_flags: CRYPTO_AHASH_ALG_BLOCK_ONLY | CRYPTO_AHASH_ALG_FINAL_NONZERO,
        }),
        _ => Err(-EINVAL),
    }
}

fn xor_assign(dst: &mut [u8], src: &[u8], len: usize) {
    for i in 0..len {
        dst[i] ^= src[i];
    }
}

fn dbl(block: &mut [u8], gfmask: u8) {
    let carry = block[0] >> 7;
    let mut prev = 0u8;
    for byte in block.iter_mut().rev() {
        let next = *byte >> 7;
        *byte = (*byte << 1) | prev;
        prev = next;
    }
    if carry != 0 {
        let last = block.len() - 1;
        block[last] ^= gfmask;
    }
}

pub fn crypto_cmac_digest_setkey<C: CmacBlockCipher>(
    ctx: &mut CmacCtx<C>,
    inkey: &[u8],
) -> Result<(), i32> {
    let bs = ctx.child.block_size();
    ctx.child.setkey(inkey)?;
    ctx.consts.fill(0);
    let zero = [0u8; 16];
    let mut encrypted = [0u8; 16];
    ctx.child.encrypt_block(&mut encrypted[..bs], &zero[..bs]);

    match bs {
        16 => {
            let mut k = [0u8; 16];
            k.copy_from_slice(&encrypted);
            dbl(&mut k, 0x87);
            ctx.consts[..16].copy_from_slice(&k);
            dbl(&mut k, 0x87);
            ctx.consts[16..32].copy_from_slice(&k);
        }
        8 => {
            let mut k = [0u8; 8];
            k.copy_from_slice(&encrypted[..8]);
            dbl(&mut k, 0x1b);
            ctx.consts[..8].copy_from_slice(&k);
            dbl(&mut k, 0x1b);
            ctx.consts[8..16].copy_from_slice(&k);
        }
        _ => return Err(-EINVAL),
    }
    Ok(())
}

pub fn crypto_cmac_digest_init(prev: &mut [u8], bs: usize) -> Result<(), i32> {
    prev[..bs].fill(0);
    Ok(())
}

pub fn crypto_cmac_digest_update<C: CmacBlockCipher>(
    ctx: &CmacCtx<C>,
    prev: &mut [u8],
    mut p: &[u8],
) -> usize {
    let bs = ctx.child.block_size();
    while p.len() >= bs {
        xor_assign(prev, &p[..bs], bs);
        let mut block = [0u8; 16];
        block[..bs].copy_from_slice(&prev[..bs]);
        ctx.child.encrypt_block(&mut prev[..bs], &block[..bs]);
        p = &p[bs..];
    }
    p.len()
}

pub fn crypto_cmac_digest_finup<C: CmacBlockCipher>(
    ctx: &CmacCtx<C>,
    prev: &mut [u8],
    src: &[u8],
    out: &mut [u8],
) -> Result<(), i32> {
    let bs = ctx.child.block_size();
    xor_assign(prev, src, src.len());
    let mut offset = 0usize;
    if src.len() != bs {
        prev[src.len()] ^= 0x80;
        offset += bs;
    }
    xor_assign(prev, &ctx.consts[offset..offset + bs], bs);
    let mut block = [0u8; 16];
    block[..bs].copy_from_slice(&prev[..bs]);
    ctx.child.encrypt_block(&mut out[..bs], &block[..bs]);
    Ok(())
}

pub fn crypto_cmac_digest<C: CmacBlockCipher>(
    ctx: &CmacCtx<C>,
    data: &[u8],
    out: &mut [u8],
) -> Result<(), i32> {
    let bs = ctx.child.block_size();
    let final_len = if data.is_empty() {
        0
    } else {
        let rem = data.len() % bs;
        if rem == 0 { bs } else { rem }
    };
    let update_len = data.len() - final_len;
    let mut prev = [0u8; 16];
    crypto_cmac_digest_init(&mut prev, bs)?;
    assert_eq!(
        crypto_cmac_digest_update(ctx, &mut prev, &data[..update_len]),
        0
    );
    crypto_cmac_digest_finup(ctx, &mut prev, &data[update_len..], out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::crypto::aes::{AesEncKey, aes_encrypt, aes_prepareenckey};

    struct AesCipher(AesEncKey);

    impl Default for AesCipher {
        fn default() -> Self {
            Self(AesEncKey::default())
        }
    }

    impl CmacBlockCipher for AesCipher {
        fn block_size(&self) -> usize {
            16
        }

        fn setkey(&mut self, key: &[u8]) -> Result<(), i32> {
            let err = aes_prepareenckey(&mut self.0, key);
            if err == 0 { Ok(()) } else { Err(err) }
        }

        fn encrypt_block(&self, dst: &mut [u8], src: &[u8]) {
            let mut input = [0u8; 16];
            let mut output = [0u8; 16];
            input.copy_from_slice(&src[..16]);
            aes_encrypt(&self.0, &mut output, &input);
            dst[..16].copy_from_slice(&output);
        }
    }

    #[test]
    fn cmac_matches_linux_source_and_nist_aes_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/cmac.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        assert!(source.contains("crypto_cmac_digest_setkey"));
        assert!(source.contains("gfmask = 0x87;"));
        assert!(source.contains("gfmask = 0x1B;"));
        assert!(source.contains("prev[len] ^= 0x80;"));
        assert!(source.contains("CRYPTO_AHASH_ALG_BLOCK_ONLY"));
        assert!(source.contains("CRYPTO_AHASH_ALG_FINAL_NONZERO"));
        assert!(testmgr.contains("static const struct hash_testvec aes_cmac128_tv_template[]"));
        assert!(testmgr_c.contains("\"cmac(aes)\""));

        let key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let data = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51, 0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11,
        ];
        let expected = [
            0xdf, 0xa6, 0x67, 0x47, 0xde, 0x9a, 0xe6, 0x30, 0x30, 0xca, 0x32, 0x61, 0x14, 0x97,
            0xc8, 0x27,
        ];
        let mut ctx = CmacCtx {
            child: AesCipher::default(),
            consts: [0; 32],
        };
        crypto_cmac_digest_setkey(&mut ctx, &key).unwrap();
        let mut out = [0u8; 16];
        crypto_cmac_digest(&ctx, &data, &mut out).unwrap();
        assert_eq!(out, expected);

        let mut empty = [0u8; 16];
        crypto_cmac_digest(&ctx, &[], &mut empty).unwrap();
        assert_eq!(
            empty,
            [
                0xbb, 0x1d, 0x69, 0x29, 0xe9, 0x59, 0x37, 0x28, 0x7f, 0xa3, 0x7d, 0x12, 0x9b, 0x75,
                0x67, 0x46,
            ]
        );
    }

    #[test]
    fn cmac_create_rejects_non_linux_block_sizes() {
        assert_eq!(cmac_create(16).unwrap().digestsize, 16);
        assert_eq!(cmac_create(8).unwrap().descsize, 8);
        assert_eq!(cmac_create(4), Err(-EINVAL));
    }
}
