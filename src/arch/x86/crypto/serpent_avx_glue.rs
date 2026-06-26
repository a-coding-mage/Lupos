//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/serpent_avx_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/serpent_avx_glue.c
//! Serpent AVX skcipher glue registration metadata.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const SERPENT_MIN_KEY_SIZE: usize = 0;
pub const SERPENT_MAX_KEY_SIZE: usize = 32;
pub const SERPENT_BLOCK_SIZE: usize = 16;
pub const SERPENT_PARALLEL_BLOCKS: usize = 8;

pub const SERPENT_AVX_DESCRIPTION: &str = "Serpent Cipher Algorithm, AVX optimized";
pub const SERPENT_AVX_MODULE_ALIASES: [&str; 1] = ["serpent"];
pub const SERPENT_AVX_EXPORTS: [&str; 3] = [
    "serpent_ecb_enc_8way_avx",
    "serpent_ecb_dec_8way_avx",
    "serpent_cbc_dec_8way_avx",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerpentAvxMode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerpentAvxSkcipherAlg {
    pub mode: SerpentAvxMode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub fpu_blocks: usize,
    pub parallel_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const SERPENT_AVX_SKCIPHER_ALGS: [SerpentAvxSkcipherAlg; 2] = [
    SerpentAvxSkcipherAlg {
        mode: SerpentAvxMode::Ecb,
        cra_name: "ecb(serpent)",
        cra_driver_name: "ecb-serpent-avx",
        cra_priority: 500,
        cra_blocksize: SERPENT_BLOCK_SIZE,
        min_keysize: SERPENT_MIN_KEY_SIZE,
        max_keysize: SERPENT_MAX_KEY_SIZE,
        ivsize: 0,
        fpu_blocks: SERPENT_PARALLEL_BLOCKS,
        parallel_blocks: SERPENT_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    SerpentAvxSkcipherAlg {
        mode: SerpentAvxMode::Cbc,
        cra_name: "cbc(serpent)",
        cra_driver_name: "cbc-serpent-avx",
        cra_priority: 500,
        cra_blocksize: SERPENT_BLOCK_SIZE,
        min_keysize: SERPENT_MIN_KEY_SIZE,
        max_keysize: SERPENT_MAX_KEY_SIZE,
        ivsize: SERPENT_BLOCK_SIZE,
        fpu_blocks: SERPENT_PARALLEL_BLOCKS,
        parallel_blocks: SERPENT_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SerpentAvxCpuFeatures {
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerpentAvxRegistration {
    pub alg_count: usize,
    pub export_count: usize,
}

pub const fn serpent_avx_cpu_supported(cpu: SerpentAvxCpuFeatures) -> bool {
    cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn serpent_avx_init(
    cpu: SerpentAvxCpuFeatures,
    crypto_api_available: bool,
) -> Result<SerpentAvxRegistration, i32> {
    if !serpent_avx_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(SerpentAvxRegistration {
        alg_count: SERPENT_AVX_SKCIPHER_ALGS.len(),
        export_count: SERPENT_AVX_EXPORTS.len(),
    })
}

pub const fn serpent_avx_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serpent_avx_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/serpent_avx_glue.c"
        ));
        let avx_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/serpent-avx.h"
        ));
        let serpent_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/serpent.h"
        ));

        assert!(source.contains("asmlinkage void serpent_ecb_enc_8way_avx"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(serpent_ecb_enc_8way_avx);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(serpent_ecb_dec_8way_avx);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(serpent_cbc_dec_8way_avx);"));
        assert!(source.contains("return __serpent_setkey(crypto_skcipher_ctx(tfm), key, keylen);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_ecb_enc_8way_avx);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_ecb_dec_8way_avx);"));
        assert!(source.contains("CBC_ENC_BLOCK(__serpent_encrypt);"));
        assert!(
            source.contains("CBC_DEC_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_cbc_dec_8way_avx);")
        );
        assert!(source.contains(".base.cra_name\t\t= \"ecb(serpent)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-serpent-avx\""));
        assert!(source.contains(".base.cra_name\t\t= \"cbc(serpent)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-serpent-avx\""));
        assert!(source.contains(".base.cra_priority\t= 500"));
        assert!(source.contains("XFEATURE_MASK_SSE | XFEATURE_MASK_YMM"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(serpent_algs"));
        assert!(source.contains("crypto_unregister_skciphers(serpent_algs"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Serpent Cipher Algorithm, AVX optimized\");")
        );
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"serpent\");"));
        assert!(avx_header.contains("#define SERPENT_PARALLEL_BLOCKS 8"));
        assert!(serpent_header.contains("#define SERPENT_BLOCK_SIZE\t\t 16"));

        assert_eq!(SERPENT_AVX_SKCIPHER_ALGS.len(), 2);
        assert_eq!(SERPENT_AVX_SKCIPHER_ALGS[0].cra_priority, 500);
        assert_eq!(SERPENT_AVX_SKCIPHER_ALGS[1].ivsize, SERPENT_BLOCK_SIZE);
        assert_eq!(
            SERPENT_AVX_EXPORTS,
            [
                "serpent_ecb_enc_8way_avx",
                "serpent_ecb_dec_8way_avx",
                "serpent_cbc_dec_8way_avx"
            ]
        );
    }

    #[test]
    fn serpent_avx_init_tracks_linux_feature_gate() {
        let cpu = SerpentAvxCpuFeatures {
            xfeatures_sse: true,
            xfeatures_ymm: true,
        };

        assert!(serpent_avx_cpu_supported(cpu));
        assert_eq!(
            serpent_avx_init(cpu, true),
            Ok(SerpentAvxRegistration {
                alg_count: 2,
                export_count: 3,
            })
        );
        assert_eq!(serpent_avx_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(
            serpent_avx_init(
                SerpentAvxCpuFeatures {
                    xfeatures_sse: true,
                    xfeatures_ymm: false,
                },
                true,
            ),
            Err(-ENODEV)
        );
        assert!(serpent_avx_exit(true));
    }
}
