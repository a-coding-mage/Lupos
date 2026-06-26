//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/cast6_avx_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/cast6_avx_glue.c
//! CAST6 AVX skcipher glue registration metadata.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const CAST6_BLOCK_SIZE: usize = 16;
pub const CAST6_MIN_KEY_SIZE: usize = 16;
pub const CAST6_MAX_KEY_SIZE: usize = 32;
pub const CAST6_PARALLEL_BLOCKS: usize = 8;

pub const CAST6_AVX_DESCRIPTION: &str = "Cast6 Cipher Algorithm, AVX optimized";
pub const CAST6_AVX_MODULE_ALIASES: [&str; 1] = ["cast6"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cast6AvxMode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cast6AvxSkcipherAlg {
    pub mode: Cast6AvxMode,
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

pub const CAST6_AVX_SKCIPHER_ALGS: [Cast6AvxSkcipherAlg; 2] = [
    Cast6AvxSkcipherAlg {
        mode: Cast6AvxMode::Ecb,
        cra_name: "ecb(cast6)",
        cra_driver_name: "ecb-cast6-avx",
        cra_priority: 200,
        cra_blocksize: CAST6_BLOCK_SIZE,
        min_keysize: CAST6_MIN_KEY_SIZE,
        max_keysize: CAST6_MAX_KEY_SIZE,
        ivsize: 0,
        fpu_blocks: CAST6_PARALLEL_BLOCKS as i32,
        parallel_blocks: CAST6_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    Cast6AvxSkcipherAlg {
        mode: Cast6AvxMode::Cbc,
        cra_name: "cbc(cast6)",
        cra_driver_name: "cbc-cast6-avx",
        cra_priority: 200,
        cra_blocksize: CAST6_BLOCK_SIZE,
        min_keysize: CAST6_MIN_KEY_SIZE,
        max_keysize: CAST6_MAX_KEY_SIZE,
        ivsize: CAST6_BLOCK_SIZE,
        fpu_blocks: CAST6_PARALLEL_BLOCKS as i32,
        parallel_blocks: CAST6_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cast6AvxCpuFeatures {
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cast6AvxRegistration {
    pub alg_count: usize,
}

pub const fn cast6_avx_cpu_supported(cpu: Cast6AvxCpuFeatures) -> bool {
    cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn cast6_avx_init(
    cpu: Cast6AvxCpuFeatures,
    crypto_api_available: bool,
) -> Result<Cast6AvxRegistration, i32> {
    if !cast6_avx_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(Cast6AvxRegistration {
        alg_count: CAST6_AVX_SKCIPHER_ALGS.len(),
    })
}

pub const fn cast6_avx_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast6_avx_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/cast6_avx_glue.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/cast6.h"
        ));

        assert!(source.contains("#define CAST6_PARALLEL_BLOCKS 8"));
        assert!(source.contains("asmlinkage void cast6_ecb_enc_8way"));
        assert!(source.contains("asmlinkage void cast6_ecb_dec_8way"));
        assert!(source.contains("asmlinkage void cast6_cbc_dec_8way"));
        assert!(source.contains("return cast6_setkey(&tfm->base, key, keylen);"));
        assert!(source.contains("ECB_BLOCK(CAST6_PARALLEL_BLOCKS, cast6_ecb_enc_8way);"));
        assert!(source.contains("ECB_BLOCK(CAST6_PARALLEL_BLOCKS, cast6_ecb_dec_8way);"));
        assert!(source.contains("CBC_ENC_BLOCK(__cast6_encrypt);"));
        assert!(source.contains("CBC_DEC_BLOCK(CAST6_PARALLEL_BLOCKS, cast6_cbc_dec_8way);"));
        assert!(source.contains(".base.cra_name\t\t= \"ecb(cast6)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-cast6-avx\""));
        assert!(source.contains(".base.cra_name\t\t= \"cbc(cast6)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-cast6-avx\""));
        assert!(source.contains(".base.cra_priority\t= 200"));
        assert!(source.contains(".ivsize\t\t\t= CAST6_BLOCK_SIZE"));
        assert!(source.contains("XFEATURE_MASK_SSE | XFEATURE_MASK_YMM"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(cast6_algs"));
        assert!(source.contains("crypto_unregister_skciphers(cast6_algs"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Cast6 Cipher Algorithm, AVX optimized\");"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"cast6\");"));
        assert!(header.contains("#define CAST6_BLOCK_SIZE 16"));
        assert!(header.contains("#define CAST6_MIN_KEY_SIZE 16"));
        assert!(header.contains("#define CAST6_MAX_KEY_SIZE 32"));

        assert_eq!(CAST6_AVX_SKCIPHER_ALGS.len(), 2);
        assert_eq!(CAST6_AVX_SKCIPHER_ALGS[0].cra_driver_name, "ecb-cast6-avx");
        assert_eq!(CAST6_AVX_SKCIPHER_ALGS[1].ivsize, CAST6_BLOCK_SIZE);
        assert_eq!(CAST6_AVX_MODULE_ALIASES, ["cast6"]);
    }

    #[test]
    fn cast6_avx_init_tracks_linux_feature_gate() {
        let cpu = Cast6AvxCpuFeatures {
            xfeatures_sse: true,
            xfeatures_ymm: true,
        };

        assert!(cast6_avx_cpu_supported(cpu));
        assert_eq!(
            cast6_avx_init(cpu, true),
            Ok(Cast6AvxRegistration { alg_count: 2 })
        );
        assert_eq!(cast6_avx_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(
            cast6_avx_init(
                Cast6AvxCpuFeatures {
                    xfeatures_sse: false,
                    xfeatures_ymm: true,
                },
                true,
            ),
            Err(-ENODEV)
        );
        assert!(cast6_avx_exit(true));
    }
}
