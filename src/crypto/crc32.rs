//! linux-parity: complete
//! linux-source: vendor/linux/crypto/crc32.c
//! test-origin: linux:vendor/linux/crypto/crc32.c
//! Crypto API shash wrapper for crc32_le.

use crate::include::uapi::errno::EINVAL;
use crate::lib::crc::crc32_main::crc32_le;

pub const CHKSUM_BLOCK_SIZE: usize = 1;
pub const CHKSUM_DIGEST_SIZE: usize = 4;
pub const CRA_NAME: &str = "crc32";
pub const CRA_DRIVER_NAME: &str = "crc32-lib";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_AUTHOR: &str = "Alexander Boyko <alexander_boyko@xyratex.com>";
pub const MODULE_DESCRIPTION: &str = "CRC32 calculations wrapper for lib/crc32";
pub const MODULE_ALIAS_CRYPTO: &str = "crc32";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Crc32TfmCtx {
    pub key: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Crc32DescCtx {
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

pub const CRC32_ALG: ShashAlg = ShashAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    cra_flags_optional_key: true,
    cra_blocksize: CHKSUM_BLOCK_SIZE,
    cra_ctxsize: core::mem::size_of::<Crc32TfmCtx>(),
    descsize: core::mem::size_of::<Crc32DescCtx>(),
    digestsize: CHKSUM_DIGEST_SIZE,
};

pub fn crc32_cra_init(ctx: &mut Crc32TfmCtx) -> i32 {
    ctx.key = 0;
    0
}

pub fn crc32_setkey(ctx: &mut Crc32TfmCtx, key: &[u8]) -> Result<(), i32> {
    if key.len() != core::mem::size_of::<u32>() {
        return Err(-EINVAL);
    }
    ctx.key = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
    Ok(())
}

pub fn crc32_init(tfm: &Crc32TfmCtx, desc: &mut Crc32DescCtx) -> i32 {
    desc.crc = tfm.key;
    0
}

pub fn crc32_update(desc: &mut Crc32DescCtx, data: &[u8]) -> i32 {
    desc.crc = crc32_le(desc.crc, data);
    0
}

pub fn crc32_final(desc: &Crc32DescCtx, out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = desc.crc.to_le_bytes();
    0
}

pub fn crc32_finup(desc: &Crc32DescCtx, data: &[u8], out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = crc32_le(desc.crc, data).to_le_bytes();
    0
}

pub fn crc32_digest(tfm: &Crc32TfmCtx, data: &[u8], out: &mut [u8; CHKSUM_DIGEST_SIZE]) -> i32 {
    *out = crc32_le(tfm.key, data).to_le_bytes();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_linux_shash_wrapper_and_testmgr_vectors() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/crc32.c"
        ));
        let testmgr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("#define CHKSUM_BLOCK_SIZE\t1"));
        assert!(source.contains("#define CHKSUM_DIGEST_SIZE\t4"));
        assert!(source.contains("*key = 0;"));
        assert!(source.contains("if (keylen != sizeof(u32))"));
        assert!(source.contains("*mctx = get_unaligned_le32(key);"));
        assert!(source.contains("*crcp = crc32_le(*crcp, data, len);"));
        assert!(source.contains("put_unaligned_le32(*crcp, out);"));
        assert!(source.contains(".base.cra_driver_name\t= \"crc32-lib\""));
        assert!(source.contains(".base.cra_flags\t\t= CRYPTO_ALG_OPTIONAL_KEY"));
        assert!(source.contains("return crypto_register_shash(&alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"crc32\")"));
        assert!(testmgr.contains("static const struct hash_testvec crc32_tv_template[]"));
        assert!(testmgr.contains(".digest = \"\\xd8\\xb5\\x46\\xac\""));
        assert!(testmgr.contains(".digest = \"\\x87\\xa9\\xcb\\xed\""));

        let mut tfm = Crc32TfmCtx { key: 0xffff_ffff };
        assert_eq!(crc32_cra_init(&mut tfm), 0);
        assert_eq!(tfm.key, 0);
        assert_eq!(crc32_setkey(&mut tfm, &[0x87, 0xa9, 0xcb, 0xed]), Ok(()));
        assert_eq!(tfm.key, 0xedcb_a987);
        assert_eq!(crc32_setkey(&mut tfm, &[0, 1, 2]), Err(-EINVAL));

        let mut out = [0u8; CHKSUM_DIGEST_SIZE];
        assert_eq!(crc32_digest(&Crc32TfmCtx { key: 0 }, b"", &mut out), 0);
        assert_eq!(out, [0, 0, 0, 0]);
        assert_eq!(
            crc32_digest(&Crc32TfmCtx { key: 0 }, b"abcdefg", &mut out),
            0
        );
        assert_eq!(out, [0xd8, 0xb5, 0x46, 0xac]);
        assert_eq!(crc32_digest(&tfm, b"", &mut out), 0);
        assert_eq!(out, [0x87, 0xa9, 0xcb, 0xed]);

        let mut desc = Crc32DescCtx::default();
        assert_eq!(crc32_init(&Crc32TfmCtx { key: 0 }, &mut desc), 0);
        assert_eq!(crc32_update(&mut desc, b"abc"), 0);
        assert_eq!(crc32_finup(&desc, b"defg", &mut out), 0);
        assert_eq!(out, [0xd8, 0xb5, 0x46, 0xac]);
        assert_eq!(crc32_update(&mut desc, b"defg"), 0);
        assert_eq!(crc32_final(&desc, &mut out), 0);
        assert_eq!(out, [0xd8, 0xb5, 0x46, 0xac]);
        assert_eq!(CRC32_ALG.digestsize, 4);
        assert!(CRC32_ALG.cra_flags_optional_key);
    }
}
