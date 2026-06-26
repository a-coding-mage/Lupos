//! linux-parity: complete
//! linux-source: vendor/linux/crypto/xcbc.c
//! test-origin: linux:vendor/linux/crypto/xcbc.c
//! XCBC keyed hash wrapper from the Linux Crypto API.

use crate::include::uapi::errno::EINVAL;

pub const XCBC_BLOCKSIZE: usize = 16;
pub const CRYPTO_AHASH_ALG_BLOCK_ONLY: u32 = 0x0100_0000;
pub const CRYPTO_AHASH_ALG_FINAL_NONZERO: u32 = 0x0200_0000;
pub const MODULE_DESCRIPTION: &str = "XCBC keyed hash algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "xcbc";
pub const KS: [u8; 48] = [
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
];

pub trait XcbcBlockCipher {
    fn block_size(&self) -> usize;
    fn setkey(&mut self, key: &[u8]) -> Result<(), i32>;
    fn encrypt_block(&self, dst: &mut [u8], src: &[u8]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XcbcInstance {
    pub blocksize: usize,
    pub digestsize: usize,
    pub descsize: usize,
    pub cra_flags: u32,
}

#[derive(Clone, Debug)]
pub struct XcbcCtx<C> {
    pub child: C,
    pub consts: [u8; XCBC_BLOCKSIZE * 2],
}

pub fn xcbc_create(blocksize: usize) -> Result<XcbcInstance, i32> {
    if blocksize != XCBC_BLOCKSIZE {
        return Err(-EINVAL);
    }
    Ok(XcbcInstance {
        blocksize,
        digestsize: blocksize,
        descsize: blocksize,
        cra_flags: CRYPTO_AHASH_ALG_BLOCK_ONLY | CRYPTO_AHASH_ALG_FINAL_NONZERO,
    })
}

fn xor_assign(dst: &mut [u8], src: &[u8], len: usize) {
    for i in 0..len {
        dst[i] ^= src[i];
    }
}

pub fn crypto_xcbc_digest_setkey<C: XcbcBlockCipher>(
    ctx: &mut XcbcCtx<C>,
    inkey: &[u8],
) -> Result<(), i32> {
    ctx.child.setkey(inkey)?;
    let mut key1 = [0u8; XCBC_BLOCKSIZE];
    ctx.child.encrypt_block(
        &mut ctx.consts[..XCBC_BLOCKSIZE],
        &KS[XCBC_BLOCKSIZE..XCBC_BLOCKSIZE * 2],
    );
    ctx.child.encrypt_block(
        &mut ctx.consts[XCBC_BLOCKSIZE..XCBC_BLOCKSIZE * 2],
        &KS[XCBC_BLOCKSIZE * 2..XCBC_BLOCKSIZE * 3],
    );
    ctx.child.encrypt_block(&mut key1, &KS[..XCBC_BLOCKSIZE]);
    ctx.child.setkey(&key1)
}

pub fn crypto_xcbc_digest_init(prev: &mut [u8]) -> Result<(), i32> {
    prev[..XCBC_BLOCKSIZE].fill(0);
    Ok(())
}

pub fn crypto_xcbc_digest_update<C: XcbcBlockCipher>(
    ctx: &XcbcCtx<C>,
    prev: &mut [u8],
    mut p: &[u8],
) -> usize {
    while p.len() >= XCBC_BLOCKSIZE {
        xor_assign(prev, &p[..XCBC_BLOCKSIZE], XCBC_BLOCKSIZE);
        let mut block = [0u8; XCBC_BLOCKSIZE];
        block.copy_from_slice(&prev[..XCBC_BLOCKSIZE]);
        ctx.child.encrypt_block(&mut prev[..XCBC_BLOCKSIZE], &block);
        p = &p[XCBC_BLOCKSIZE..];
    }
    p.len()
}

pub fn crypto_xcbc_digest_finup<C: XcbcBlockCipher>(
    ctx: &XcbcCtx<C>,
    prev: &mut [u8],
    src: &[u8],
    out: &mut [u8],
) -> Result<(), i32> {
    xor_assign(prev, src, src.len());
    let mut offset = 0usize;
    if src.len() != XCBC_BLOCKSIZE {
        prev[src.len()] ^= 0x80;
        offset += XCBC_BLOCKSIZE;
    }
    xor_assign(
        prev,
        &ctx.consts[offset..offset + XCBC_BLOCKSIZE],
        XCBC_BLOCKSIZE,
    );
    let mut block = [0u8; XCBC_BLOCKSIZE];
    block.copy_from_slice(&prev[..XCBC_BLOCKSIZE]);
    ctx.child.encrypt_block(&mut out[..XCBC_BLOCKSIZE], &block);
    Ok(())
}

pub fn crypto_xcbc_digest<C: XcbcBlockCipher>(
    ctx: &XcbcCtx<C>,
    data: &[u8],
    out: &mut [u8],
) -> Result<(), i32> {
    let final_len = if data.is_empty() {
        0
    } else {
        let rem = data.len() % XCBC_BLOCKSIZE;
        if rem == 0 { XCBC_BLOCKSIZE } else { rem }
    };
    let update_len = data.len() - final_len;
    let mut prev = [0u8; XCBC_BLOCKSIZE];
    crypto_xcbc_digest_init(&mut prev)?;
    assert_eq!(
        crypto_xcbc_digest_update(ctx, &mut prev, &data[..update_len]),
        0
    );
    crypto_xcbc_digest_finup(ctx, &mut prev, &data[update_len..], out)
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

    impl XcbcBlockCipher for AesCipher {
        fn block_size(&self) -> usize {
            XCBC_BLOCKSIZE
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
    fn xcbc_matches_linux_source_and_aes_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/xcbc.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        assert!(source.contains("static u_int32_t ks[12]"));
        assert!(source.contains("crypto_xcbc_digest_setkey"));
        assert!(source.contains("crypto_cipher_encrypt_one(ctx->child, consts, (u8 *)ks + bs);"));
        assert!(source.contains("return crypto_cipher_setkey(ctx->child, key1, bs);"));
        assert!(source.contains("prev[len] ^= 0x80;"));
        assert!(source.contains("CRYPTO_AHASH_ALG_FINAL_NONZERO"));
        assert!(testmgr.contains("static const struct hash_testvec aes_xcbc128_tv_template[]"));
        assert!(testmgr_c.contains("\"xcbc(aes)\""));

        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let data = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
        ];
        let expected = [
            0x47, 0xf5, 0x1b, 0x45, 0x64, 0x96, 0x62, 0x15, 0xb8, 0x98, 0x5c, 0x63, 0x05, 0x5e,
            0xd3, 0x08,
        ];
        let mut ctx = XcbcCtx {
            child: AesCipher::default(),
            consts: [0; 32],
        };
        crypto_xcbc_digest_setkey(&mut ctx, &key).unwrap();
        let mut out = [0u8; 16];
        crypto_xcbc_digest(&ctx, &data, &mut out).unwrap();
        assert_eq!(out, expected);

        let mut empty = [0u8; 16];
        crypto_xcbc_digest(&ctx, &[], &mut empty).unwrap();
        assert_eq!(
            empty,
            [
                0x75, 0xf0, 0x25, 0x1d, 0x52, 0x8a, 0xc0, 0x1c, 0x45, 0x73, 0xdf, 0xd5, 0x84, 0xd7,
                0x9f, 0x29,
            ]
        );
    }

    #[test]
    fn xcbc_create_rejects_non_linux_block_sizes() {
        assert_eq!(xcbc_create(16).unwrap().digestsize, 16);
        assert_eq!(xcbc_create(8), Err(-EINVAL));
    }
}
