//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/twofish_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/twofish_glue.c
//! x86 Twofish assembler glue registration metadata.

use crate::include::uapi::errno::EOPNOTSUPP;

pub const TF_MIN_KEY_SIZE: usize = 16;
pub const TF_MAX_KEY_SIZE: usize = 32;
pub const TF_BLOCK_SIZE: usize = 16;

pub const TWOFISH_CRA_NAME: &str = "twofish";
pub const TWOFISH_DRIVER_NAME: &str = "twofish-asm";
pub const TWOFISH_PRIORITY: i32 = 200;
pub const TWOFISH_DESCRIPTION: &str = "Twofish Cipher Algorithm, asm optimized";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TwofishGlueAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const fn twofish_glue_alg() -> TwofishGlueAlg {
    TwofishGlueAlg {
        cra_name: TWOFISH_CRA_NAME,
        cra_driver_name: TWOFISH_DRIVER_NAME,
        cra_priority: TWOFISH_PRIORITY,
        cra_blocksize: TF_BLOCK_SIZE,
        min_keysize: TF_MIN_KEY_SIZE,
        max_keysize: TF_MAX_KEY_SIZE,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    }
}

pub const fn twofish_glue_init(crypto_api_available: bool) -> Result<TwofishGlueAlg, i32> {
    if crypto_api_available {
        Ok(twofish_glue_alg())
    } else {
        Err(-EOPNOTSUPP)
    }
}

pub const fn twofish_glue_fini(registered: bool) -> bool {
    !registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twofish_glue_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/twofish_glue.c"
        ));
        assert!(source.contains("asmlinkage void twofish_enc_blk"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(twofish_enc_blk);"));
        assert!(source.contains("asmlinkage void twofish_dec_blk"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(twofish_dec_blk);"));
        assert!(source.contains("twofish_enc_blk(crypto_tfm_ctx(tfm), dst, src);"));
        assert!(source.contains("twofish_dec_blk(crypto_tfm_ctx(tfm), dst, src);"));
        assert!(source.contains(".cra_name\t\t=\t\"twofish\""));
        assert!(source.contains(".cra_driver_name\t=\t\"twofish-asm\""));
        assert!(source.contains(".cra_priority\t\t=\t200"));
        assert!(source.contains(".cra_blocksize\t\t=\tTF_BLOCK_SIZE"));
        assert!(source.contains(".cia_min_keysize\t=\tTF_MIN_KEY_SIZE"));
        assert!(source.contains(".cia_max_keysize\t=\tTF_MAX_KEY_SIZE"));
        assert!(source.contains(".cia_setkey\t\t=\ttwofish_setkey"));
        assert!(source.contains("return crypto_register_alg(&alg);"));
        assert!(source.contains("crypto_unregister_alg(&alg);"));
        assert!(
            source.contains("MODULE_DESCRIPTION (\"Twofish Cipher Algorithm, asm optimized\");")
        );
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"twofish-asm\");"));

        let alg = twofish_glue_alg();
        assert_eq!(alg.cra_name, TWOFISH_CRA_NAME);
        assert_eq!(alg.cra_driver_name, TWOFISH_DRIVER_NAME);
        assert_eq!(alg.cra_priority, 200);
        assert_eq!(alg.cra_blocksize, 16);
        assert_eq!(alg.min_keysize, 16);
        assert_eq!(alg.max_keysize, 32);
        assert!(alg.has_setkey);
        assert!(alg.has_encrypt);
        assert!(alg.has_decrypt);
        assert_eq!(twofish_glue_init(false), Err(-EOPNOTSUPP));
        assert_eq!(twofish_glue_init(true), Ok(alg));
        assert!(twofish_glue_fini(false));
    }
}
