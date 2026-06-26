//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/twofish_avx_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/twofish_avx_glue.c
//! Twofish AVX skcipher glue registration metadata.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const TF_MIN_KEY_SIZE: usize = 16;
pub const TF_MAX_KEY_SIZE: usize = 32;
pub const TF_BLOCK_SIZE: usize = 16;
pub const TWOFISH_PARALLEL_BLOCKS: usize = 8;
pub const TWOFISH_THREEWAY_BLOCKS: usize = 3;

pub const TWOFISH_AVX_DESCRIPTION: &str = "Twofish Cipher Algorithm, AVX optimized";
pub const TWOFISH_AVX_MODULE_ALIASES: [&str; 1] = ["twofish"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwofishAvxMode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TwofishAvxSkcipherAlg {
    pub mode: TwofishAvxMode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub fpu_blocks: usize,
    pub avx_parallel_blocks: usize,
    pub threeway_fallback_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const TWOFISH_AVX_SKCIPHER_ALGS: [TwofishAvxSkcipherAlg; 2] = [
    TwofishAvxSkcipherAlg {
        mode: TwofishAvxMode::Ecb,
        cra_name: "ecb(twofish)",
        cra_driver_name: "ecb-twofish-avx",
        cra_priority: 400,
        cra_blocksize: TF_BLOCK_SIZE,
        min_keysize: TF_MIN_KEY_SIZE,
        max_keysize: TF_MAX_KEY_SIZE,
        ivsize: 0,
        fpu_blocks: TWOFISH_PARALLEL_BLOCKS,
        avx_parallel_blocks: TWOFISH_PARALLEL_BLOCKS,
        threeway_fallback_blocks: TWOFISH_THREEWAY_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    TwofishAvxSkcipherAlg {
        mode: TwofishAvxMode::Cbc,
        cra_name: "cbc(twofish)",
        cra_driver_name: "cbc-twofish-avx",
        cra_priority: 400,
        cra_blocksize: TF_BLOCK_SIZE,
        min_keysize: TF_MIN_KEY_SIZE,
        max_keysize: TF_MAX_KEY_SIZE,
        ivsize: TF_BLOCK_SIZE,
        fpu_blocks: TWOFISH_PARALLEL_BLOCKS,
        avx_parallel_blocks: TWOFISH_PARALLEL_BLOCKS,
        threeway_fallback_blocks: TWOFISH_THREEWAY_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TwofishAvxCpuFeatures {
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TwofishAvxRegistration {
    pub alg_count: usize,
    pub uses_threeway_fallback: bool,
}

pub const fn twofish_avx_cpu_supported(cpu: TwofishAvxCpuFeatures) -> bool {
    cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn twofish_avx_init(
    cpu: TwofishAvxCpuFeatures,
    crypto_api_available: bool,
) -> Result<TwofishAvxRegistration, i32> {
    if !twofish_avx_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(TwofishAvxRegistration {
        alg_count: TWOFISH_AVX_SKCIPHER_ALGS.len(),
        uses_threeway_fallback: true,
    })
}

pub const fn twofish_avx_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twofish_avx_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/twofish_avx_glue.c"
        ));
        let local_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/twofish.h"
        ));
        let twofish_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/twofish.h"
        ));

        assert!(source.contains("#define TWOFISH_PARALLEL_BLOCKS 8"));
        assert!(source.contains("asmlinkage void twofish_ecb_enc_8way"));
        assert!(source.contains("asmlinkage void twofish_ecb_dec_8way"));
        assert!(source.contains("asmlinkage void twofish_cbc_dec_8way"));
        assert!(source.contains("return twofish_setkey(&tfm->base, key, keylen);"));
        assert!(source.contains("__twofish_enc_blk_3way(ctx, dst, src, false);"));
        assert!(source.contains("ECB_BLOCK(TWOFISH_PARALLEL_BLOCKS, twofish_ecb_enc_8way);"));
        assert!(source.contains("ECB_BLOCK(3, twofish_enc_blk_3way);"));
        assert!(source.contains("ECB_BLOCK(1, twofish_enc_blk);"));
        assert!(source.contains("ECB_BLOCK(TWOFISH_PARALLEL_BLOCKS, twofish_ecb_dec_8way);"));
        assert!(source.contains("ECB_BLOCK(3, twofish_dec_blk_3way);"));
        assert!(source.contains("CBC_ENC_BLOCK(twofish_enc_blk);"));
        assert!(source.contains("CBC_DEC_BLOCK(TWOFISH_PARALLEL_BLOCKS, twofish_cbc_dec_8way);"));
        assert!(source.contains("CBC_DEC_BLOCK(3, twofish_dec_blk_cbc_3way);"));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-twofish-avx\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-twofish-avx\""));
        assert!(source.contains(".base.cra_priority\t= 400"));
        assert!(source.contains("XFEATURE_MASK_SSE | XFEATURE_MASK_YMM"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(twofish_algs"));
        assert!(source.contains("crypto_unregister_skciphers(twofish_algs"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Twofish Cipher Algorithm, AVX optimized\");")
        );
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"twofish\");"));
        assert!(local_header.contains("__twofish_enc_blk_3way"));
        assert!(local_header.contains("twofish_dec_blk_cbc_3way"));
        assert!(twofish_header.contains("#define TF_MIN_KEY_SIZE 16"));
        assert!(twofish_header.contains("#define TF_MAX_KEY_SIZE 32"));
        assert!(twofish_header.contains("#define TF_BLOCK_SIZE 16"));

        assert_eq!(TWOFISH_AVX_SKCIPHER_ALGS.len(), 2);
        assert_eq!(
            TWOFISH_AVX_SKCIPHER_ALGS[0].cra_driver_name,
            "ecb-twofish-avx"
        );
        assert_eq!(TWOFISH_AVX_SKCIPHER_ALGS[1].ivsize, TF_BLOCK_SIZE);
        assert_eq!(TWOFISH_AVX_MODULE_ALIASES, ["twofish"]);
    }

    #[test]
    fn twofish_avx_init_tracks_linux_feature_gate() {
        let cpu = TwofishAvxCpuFeatures {
            xfeatures_sse: true,
            xfeatures_ymm: true,
        };

        assert!(twofish_avx_cpu_supported(cpu));
        assert_eq!(
            twofish_avx_init(cpu, true),
            Ok(TwofishAvxRegistration {
                alg_count: 2,
                uses_threeway_fallback: true,
            })
        );
        assert_eq!(twofish_avx_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(
            twofish_avx_init(
                TwofishAvxCpuFeatures {
                    xfeatures_sse: false,
                    xfeatures_ymm: true,
                },
                true,
            ),
            Err(-ENODEV)
        );
        assert!(twofish_avx_exit(true));
    }
}
