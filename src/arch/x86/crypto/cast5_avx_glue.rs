//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/cast5_avx_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/cast5_avx_glue.c
//! CAST5 AVX skcipher glue registration metadata.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const CAST5_BLOCK_SIZE: usize = 8;
pub const CAST5_MIN_KEY_SIZE: usize = 5;
pub const CAST5_MAX_KEY_SIZE: usize = 16;
pub const CAST5_PARALLEL_BLOCKS: usize = 16;

pub const CAST5_AVX_DESCRIPTION: &str = "Cast5 Cipher Algorithm, AVX optimized";
pub const CAST5_AVX_MODULE_ALIASES: [&str; 1] = ["cast5"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cast5AvxMode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cast5AvxSkcipherAlg {
    pub mode: Cast5AvxMode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub fpu_blocks: i32,
    pub parallel_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const CAST5_AVX_SKCIPHER_ALGS: [Cast5AvxSkcipherAlg; 2] = [
    Cast5AvxSkcipherAlg {
        mode: Cast5AvxMode::Ecb,
        cra_name: "ecb(cast5)",
        cra_driver_name: "ecb-cast5-avx",
        cra_priority: 200,
        cra_blocksize: CAST5_BLOCK_SIZE,
        min_keysize: CAST5_MIN_KEY_SIZE,
        max_keysize: CAST5_MAX_KEY_SIZE,
        ivsize: 0,
        fpu_blocks: CAST5_PARALLEL_BLOCKS as i32,
        parallel_blocks: CAST5_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    Cast5AvxSkcipherAlg {
        mode: Cast5AvxMode::Cbc,
        cra_name: "cbc(cast5)",
        cra_driver_name: "cbc-cast5-avx",
        cra_priority: 200,
        cra_blocksize: CAST5_BLOCK_SIZE,
        min_keysize: CAST5_MIN_KEY_SIZE,
        max_keysize: CAST5_MAX_KEY_SIZE,
        ivsize: CAST5_BLOCK_SIZE,
        fpu_blocks: CAST5_PARALLEL_BLOCKS as i32,
        parallel_blocks: CAST5_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cast5AvxCpuFeatures {
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cast5AvxRegistration {
    pub alg_count: usize,
}

pub const fn cast5_avx_cpu_supported(cpu: Cast5AvxCpuFeatures) -> bool {
    cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn cast5_avx_init(
    cpu: Cast5AvxCpuFeatures,
    crypto_api_available: bool,
) -> Result<Cast5AvxRegistration, i32> {
    if !cast5_avx_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(Cast5AvxRegistration {
        alg_count: CAST5_AVX_SKCIPHER_ALGS.len(),
    })
}

pub const fn cast5_avx_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast5_avx_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/cast5_avx_glue.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/cast5.h"
        ));

        assert!(source.contains("#define CAST5_PARALLEL_BLOCKS 16"));
        assert!(source.contains("asmlinkage void cast5_ecb_enc_16way"));
        assert!(source.contains("asmlinkage void cast5_ecb_dec_16way"));
        assert!(source.contains("asmlinkage void cast5_cbc_dec_16way"));
        assert!(source.contains("return cast5_setkey(&tfm->base, key, keylen);"));
        assert!(source.contains("ECB_BLOCK(CAST5_PARALLEL_BLOCKS, cast5_ecb_enc_16way);"));
        assert!(source.contains("ECB_BLOCK(CAST5_PARALLEL_BLOCKS, cast5_ecb_dec_16way);"));
        assert!(source.contains("CBC_ENC_BLOCK(__cast5_encrypt);"));
        assert!(source.contains("CBC_DEC_BLOCK(CAST5_PARALLEL_BLOCKS, cast5_cbc_dec_16way);"));
        assert!(source.contains(".base.cra_name\t\t= \"ecb(cast5)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-cast5-avx\""));
        assert!(source.contains(".base.cra_name\t\t= \"cbc(cast5)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-cast5-avx\""));
        assert!(source.contains(".base.cra_priority\t= 200"));
        assert!(source.contains(".ivsize\t\t\t= CAST5_BLOCK_SIZE"));
        assert!(source.contains("XFEATURE_MASK_SSE | XFEATURE_MASK_YMM"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(cast5_algs"));
        assert!(source.contains("crypto_unregister_skciphers(cast5_algs"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Cast5 Cipher Algorithm, AVX optimized\");"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"cast5\");"));
        assert!(header.contains("#define CAST5_BLOCK_SIZE 8"));
        assert!(header.contains("#define CAST5_MIN_KEY_SIZE 5"));
        assert!(header.contains("#define CAST5_MAX_KEY_SIZE 16"));

        assert_eq!(CAST5_AVX_SKCIPHER_ALGS.len(), 2);
        assert_eq!(CAST5_AVX_SKCIPHER_ALGS[0].cra_driver_name, "ecb-cast5-avx");
        assert_eq!(CAST5_AVX_SKCIPHER_ALGS[1].ivsize, CAST5_BLOCK_SIZE);
        assert_eq!(CAST5_AVX_MODULE_ALIASES, ["cast5"]);
    }

    #[test]
    fn cast5_avx_init_tracks_linux_feature_gate() {
        let cpu = Cast5AvxCpuFeatures {
            xfeatures_sse: true,
            xfeatures_ymm: true,
        };

        assert!(cast5_avx_cpu_supported(cpu));
        assert_eq!(
            cast5_avx_init(cpu, true),
            Ok(Cast5AvxRegistration { alg_count: 2 })
        );
        assert_eq!(cast5_avx_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(
            cast5_avx_init(
                Cast5AvxCpuFeatures {
                    xfeatures_sse: true,
                    xfeatures_ymm: false,
                },
                true,
            ),
            Err(-ENODEV)
        );
        assert!(cast5_avx_exit(true));
    }
}
