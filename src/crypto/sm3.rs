//! linux-parity: complete
//! linux-source: vendor/linux/crypto/sm3.c
//! test-origin: linux:vendor/linux/crypto/sm3.c
//! Crypto API registration metadata for SM3.

pub const SM3_DIGEST_SIZE: usize = 32;
pub const SM3_BLOCK_SIZE: usize = 64;
pub const SM3_DESC_SIZE_TYPE: &str = "struct sm3_ctx";
pub const CRA_NAME: &str = "sm3";
pub const CRA_DRIVER_NAME: &str = "sm3-lib";
pub const CRA_PRIORITY: u32 = 300;
pub const MODULE_DESCRIPTION: &str = "Crypto API support for SM3";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["sm3", "sm3-lib"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShashAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub cra_blocksize: usize,
    pub digestsize: usize,
    pub descsize_type: &'static str,
}

pub const SM3_ALG: ShashAlg = ShashAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    cra_blocksize: SM3_BLOCK_SIZE,
    digestsize: SM3_DIGEST_SIZE,
    descsize_type: SM3_DESC_SIZE_TYPE,
};

pub fn crypto_sm3_init<C, F>(ctx: &mut C, sm3_init: F) -> i32
where
    F: FnOnce(&mut C),
{
    sm3_init(ctx);
    0
}

pub fn crypto_sm3_update<C, F>(ctx: &mut C, data: &[u8], sm3_update: F) -> i32
where
    F: FnOnce(&mut C, &[u8]),
{
    sm3_update(ctx, data);
    0
}

pub fn crypto_sm3_final<C, F>(ctx: &mut C, out: &mut [u8; SM3_DIGEST_SIZE], sm3_final: F) -> i32
where
    F: FnOnce(&mut C, &mut [u8; SM3_DIGEST_SIZE]),
{
    sm3_final(ctx, out);
    0
}

pub fn crypto_sm3_digest<F>(data: &[u8], out: &mut [u8; SM3_DIGEST_SIZE], sm3_digest: F) -> i32
where
    F: FnOnce(&[u8], &mut [u8; SM3_DIGEST_SIZE]),
{
    sm3_digest(data, out);
    0
}

pub fn crypto_sm3_export_core<C: Copy>(ctx: &C, out: &mut C) -> i32 {
    *out = *ctx;
    0
}

pub fn crypto_sm3_import_core<C: Copy>(ctx: &mut C, input: &C) -> i32 {
    *ctx = *input;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct TestSm3Ctx {
        init: bool,
        bytes: usize,
        finalized: bool,
    }

    #[test]
    fn sm3_matches_linux_shash_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/sm3.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/sm3.h"
        ));
        assert!(source.contains("#define SM3_CTX(desc) ((struct sm3_ctx *)shash_desc_ctx(desc))"));
        assert!(source.contains("sm3_init(SM3_CTX(desc));"));
        assert!(source.contains("sm3_update(SM3_CTX(desc), data, len);"));
        assert!(source.contains("sm3_final(SM3_CTX(desc), out);"));
        assert!(source.contains("sm3(data, len, out);"));
        assert!(source.contains("memcpy(out, SM3_CTX(desc), sizeof(struct sm3_ctx));"));
        assert!(source.contains(".base.cra_driver_name\t= \"sm3-lib\""));
        assert!(source.contains("return crypto_register_shash(&sm3_alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"sm3-lib\")"));
        assert!(header.contains("#define SM3_DIGEST_SIZE\t32"));
        assert!(header.contains("#define SM3_BLOCK_SIZE\t64"));

        assert_eq!(SM3_ALG.cra_name, "sm3");
        assert_eq!(SM3_ALG.cra_priority, 300);
        assert_eq!(SM3_ALG.digestsize, 32);
        assert_eq!(MODULE_ALIAS_CRYPTO, ["sm3", "sm3-lib"]);

        let mut ctx = TestSm3Ctx::default();
        assert_eq!(
            crypto_sm3_init(&mut ctx, |ctx| {
                ctx.init = true;
            }),
            0
        );
        assert!(ctx.init);
        assert_eq!(
            crypto_sm3_update(&mut ctx, b"abc", |ctx, data| {
                ctx.bytes += data.len();
            }),
            0
        );
        assert_eq!(ctx.bytes, 3);

        let mut digest = [0u8; SM3_DIGEST_SIZE];
        assert_eq!(
            crypto_sm3_final(&mut ctx, &mut digest, |ctx, out| {
                ctx.finalized = true;
                out[0] = ctx.bytes as u8;
            }),
            0
        );
        assert!(ctx.finalized);
        assert_eq!(digest[0], 3);

        assert_eq!(
            crypto_sm3_digest(b"one-shot", &mut digest, |data, out| {
                out[0] = data.len() as u8;
            }),
            0
        );
        assert_eq!(digest[0], 8);

        let mut exported = TestSm3Ctx::default();
        assert_eq!(crypto_sm3_export_core(&ctx, &mut exported), 0);
        assert_eq!(exported, ctx);
        let mut imported = TestSm3Ctx::default();
        assert_eq!(crypto_sm3_import_core(&mut imported, &exported), 0);
        assert_eq!(imported, exported);
    }
}
