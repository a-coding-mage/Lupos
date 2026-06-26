//! linux-parity: complete
//! linux-source: vendor/linux/crypto/sm4_generic.c
//! test-origin: linux:vendor/linux/crypto/sm4_generic.c
//! Crypto API registration metadata for the generic SM4 cipher.

pub const SM4_KEY_SIZE: usize = 16;
pub const SM4_BLOCK_SIZE: usize = 16;
pub const SM4_RKEY_WORDS: usize = 32;
pub const CRA_NAME: &str = "sm4";
pub const CRA_DRIVER_NAME: &str = "sm4-generic";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_DESCRIPTION: &str = "SM4 Cipher Algorithm";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["sm4", "sm4-generic"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoCipherAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
}

pub const SM4_ALG: CryptoCipherAlg = CryptoCipherAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    blocksize: SM4_BLOCK_SIZE,
    min_keysize: SM4_KEY_SIZE,
    max_keysize: SM4_KEY_SIZE,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Sm4Ctx {
    pub rkey_enc: [u32; SM4_RKEY_WORDS],
    pub rkey_dec: [u32; SM4_RKEY_WORDS],
}

pub fn sm4_setkey<F>(ctx: &mut Sm4Ctx, in_key: &[u8], sm4_expandkey: F) -> i32
where
    F: FnOnce(&mut Sm4Ctx, &[u8]) -> i32,
{
    sm4_expandkey(ctx, in_key)
}

pub fn sm4_encrypt<F>(
    ctx: &Sm4Ctx,
    out: &mut [u8; SM4_BLOCK_SIZE],
    input: &[u8; SM4_BLOCK_SIZE],
    sm4_crypt_block: F,
) where
    F: FnOnce(&[u32; SM4_RKEY_WORDS], &mut [u8; SM4_BLOCK_SIZE], &[u8; SM4_BLOCK_SIZE]),
{
    sm4_crypt_block(&ctx.rkey_enc, out, input);
}

pub fn sm4_decrypt<F>(
    ctx: &Sm4Ctx,
    out: &mut [u8; SM4_BLOCK_SIZE],
    input: &[u8; SM4_BLOCK_SIZE],
    sm4_crypt_block: F,
) where
    F: FnOnce(&[u32; SM4_RKEY_WORDS], &mut [u8; SM4_BLOCK_SIZE], &[u8; SM4_BLOCK_SIZE]),
{
    sm4_crypt_block(&ctx.rkey_dec, out, input);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm4_generic_matches_linux_cipher_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/sm4_generic.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/sm4.h"
        ));
        assert!(source.contains("static int sm4_setkey(struct crypto_tfm *tfm"));
        assert!(source.contains("return sm4_expandkey(ctx, in_key, key_len);"));
        assert!(source.contains("sm4_crypt_block(ctx->rkey_enc, out, in);"));
        assert!(source.contains("sm4_crypt_block(ctx->rkey_dec, out, in);"));
        assert!(source.contains(".cra_name\t\t=\t\"sm4\""));
        assert!(source.contains(".cra_driver_name\t=\t\"sm4-generic\""));
        assert!(source.contains(".cia_min_keysize\t=\tSM4_KEY_SIZE"));
        assert!(source.contains("return crypto_register_alg(&sm4_alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"sm4-generic\")"));
        assert!(header.contains("#define SM4_KEY_SIZE\t16"));
        assert!(header.contains("#define SM4_BLOCK_SIZE\t16"));

        assert_eq!(SM4_ALG.cra_name, "sm4");
        assert_eq!(SM4_ALG.blocksize, 16);
        assert_eq!(SM4_ALG.min_keysize, SM4_ALG.max_keysize);
        assert_eq!(MODULE_ALIAS_CRYPTO, ["sm4", "sm4-generic"]);

        let mut ctx = Sm4Ctx::default();
        let key = [0x11u8; SM4_KEY_SIZE];
        assert_eq!(
            sm4_setkey(&mut ctx, &key, |ctx, in_key| {
                assert_eq!(in_key, key);
                ctx.rkey_enc[0] = 0xe;
                ctx.rkey_dec[0] = 0xd;
                5
            }),
            5
        );

        let input = [0x22u8; SM4_BLOCK_SIZE];
        let mut encrypted = [0u8; SM4_BLOCK_SIZE];
        sm4_encrypt(&ctx, &mut encrypted, &input, |rk, out, input| {
            assert_eq!(rk[0], 0xe);
            out.copy_from_slice(input);
            out[0] ^= rk[0] as u8;
        });
        assert_eq!(encrypted[0], 0x2c);

        let mut decrypted = [0u8; SM4_BLOCK_SIZE];
        sm4_decrypt(&ctx, &mut decrypted, &input, |rk, out, input| {
            assert_eq!(rk[0], 0xd);
            out.copy_from_slice(input);
            out[0] ^= rk[0] as u8;
        });
        assert_eq!(decrypted[0], 0x2f);
    }
}
