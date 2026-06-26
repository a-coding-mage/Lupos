//! linux-parity: complete
//! linux-source: vendor/linux/crypto/crc32c.c
//! test-origin: linux:vendor/linux/crypto/crc32c.c
//! Crypto API shash wrapper for CRC-32C.

use crate::include::uapi::errno::EINVAL;
use crate::lib::crc::crc32_main::crc32c;

pub const CHKSUM_BLOCK_SIZE: usize = 1;
pub const CHKSUM_DIGEST_SIZE: usize = 4;
pub const CRA_NAME: &str = "crc32c";
pub const CRA_DRIVER_NAME: &str = "crc32c-lib";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_AUTHOR: &str = "Clay Haapala <chaapala@cisco.com>";
pub const MODULE_DESCRIPTION: &str = "CRC32c (Castagnoli) calculations wrapper for lib/crc32c";
pub const MODULE_ALIAS_CRYPTO: &str = "crc32c";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChksumCtx {
    pub key: u32,
}

impl Default for ChksumCtx {
    fn default() -> Self {
        Self { key: u32::MAX }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChksumDescCtx {
    pub crc: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShashAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub cra_flags_optional_key: bool,
    pub cra_blocksize: usize,
    pub cra_ctxsize: usize,
    pub descsize: usize,
    pub digestsize: usize,
}

pub const CRC32C_ALG: ShashAlg = ShashAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    cra_flags_optional_key: true,
    cra_blocksize: CHKSUM_BLOCK_SIZE,
    cra_ctxsize: core::mem::size_of::<ChksumCtx>(),
    descsize: core::mem::size_of::<ChksumDescCtx>(),
    digestsize: CHKSUM_DIGEST_SIZE,
};

pub fn crc32c_cra_init(ctx: &mut ChksumCtx) -> i32 {
    ctx.key = u32::MAX;
    0
}

pub fn chksum_setkey(ctx: &mut ChksumCtx, key: &[u8]) -> Result<(), i32> {
    if key.len() != core::mem::size_of::<u32>() {
        return Err(-EINVAL);
    }
    ctx.key = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
    Ok(())
}

pub fn chksum_init(tfm: &ChksumCtx, desc: &mut ChksumDescCtx) -> i32 {
    desc.crc = tfm.key;
    0
}

pub fn chksum_update(desc: &mut ChksumDescCtx, data: &[u8]) -> i32 {
    desc.crc = crc32c(desc.crc, data);
    0
}

pub fn chksum_final(desc: &ChksumDescCtx, out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = (!desc.crc).to_le_bytes();
    0
}

pub fn chksum_finup(desc: &ChksumDescCtx, data: &[u8], out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = (!crc32c(desc.crc, data)).to_le_bytes();
    0
}

pub fn chksum_digest(tfm: &ChksumCtx, data: &[u8], out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = (!crc32c(tfm.key, data)).to_le_bytes();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32c_matches_linux_shash_wrapper_and_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/crc32c.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("struct chksum_ctx"));
        assert!(source.contains("struct chksum_desc_ctx"));
        assert!(source.contains("ctx->crc = crc32c(ctx->crc, data, length);"));
        assert!(source.contains("put_unaligned_le32(~ctx->crc, out);"));
        assert!(source.contains("mctx->key = ~0;"));
        assert!(source.contains(".base.cra_driver_name\t= \"crc32c-lib\""));
        assert!(source.contains(".base.cra_flags\t\t= CRYPTO_ALG_OPTIONAL_KEY"));
        assert!(source.contains("return crypto_register_shash(&alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"crc32c\")"));
        assert!(testmgr.contains("static const struct hash_testvec crc32c_tv_template[]"));
        assert!(testmgr.contains(".digest = \"\\x41\\xf4\\x27\\xe6\""));
        assert!(testmgr.contains(".digest = \"\\x78\\x56\\x34\\x12\""));

        let mut tfm = ChksumCtx { key: 0 };
        assert_eq!(crc32c_cra_init(&mut tfm), 0);
        assert_eq!(tfm.key, u32::MAX);
        assert_eq!(chksum_setkey(&mut tfm, &[0x87, 0xa9, 0xcb, 0xed]), Ok(()));
        assert_eq!(tfm.key, 0xedcb_a987);
        assert_eq!(chksum_setkey(&mut tfm, &[0, 1, 2]), Err(-EINVAL));

        let mut out = [0u8; CHKSUM_DIGEST_SIZE];
        assert_eq!(chksum_digest(&ChksumCtx::default(), b"", &mut out), 0);
        assert_eq!(out, [0, 0, 0, 0]);
        assert_eq!(
            chksum_digest(&ChksumCtx::default(), b"abcdefg", &mut out),
            0
        );
        assert_eq!(out, [0x41, 0xf4, 0x27, 0xe6]);
        assert_eq!(chksum_digest(&tfm, b"", &mut out), 0);
        assert_eq!(out, [0x78, 0x56, 0x34, 0x12]);

        let mut desc = ChksumDescCtx::default();
        assert_eq!(chksum_init(&ChksumCtx::default(), &mut desc), 0);
        assert_eq!(chksum_update(&mut desc, b"abc"), 0);
        assert_eq!(chksum_finup(&desc, b"defg", &mut out), 0);
        assert_eq!(out, [0x41, 0xf4, 0x27, 0xe6]);
        assert_eq!(chksum_update(&mut desc, b"defg"), 0);
        assert_eq!(chksum_final(&desc, &mut out), 0);
        assert_eq!(out, [0x41, 0xf4, 0x27, 0xe6]);
        assert_eq!(CRC32C_ALG.cra_ctxsize, core::mem::size_of::<ChksumCtx>());
        assert!(CRC32C_ALG.cra_flags_optional_key);
    }
}
