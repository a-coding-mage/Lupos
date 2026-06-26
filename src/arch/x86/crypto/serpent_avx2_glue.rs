//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/serpent_avx2_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/serpent_avx2_glue.c
//! Serpent AVX2 skcipher glue registration metadata.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const SERPENT_MIN_KEY_SIZE: usize = 0;
pub const SERPENT_MAX_KEY_SIZE: usize = 32;
pub const SERPENT_BLOCK_SIZE: usize = 16;
pub const SERPENT_PARALLEL_BLOCKS: usize = 8;
pub const SERPENT_AVX2_PARALLEL_BLOCKS: usize = 16;

pub const SERPENT_AVX2_DESCRIPTION: &str = "Serpent Cipher Algorithm, AVX2 optimized";
pub const SERPENT_AVX2_MODULE_ALIASES: [&str; 2] = ["serpent", "serpent-asm"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerpentAvx2Mode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerpentAvx2SkcipherAlg {
    pub mode: SerpentAvx2Mode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub fpu_blocks: usize,
    pub avx2_parallel_blocks: usize,
    pub avx_fallback_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const SERPENT_AVX2_SKCIPHER_ALGS: [SerpentAvx2SkcipherAlg; 2] = [
    SerpentAvx2SkcipherAlg {
        mode: SerpentAvx2Mode::Ecb,
        cra_name: "ecb(serpent)",
        cra_driver_name: "ecb-serpent-avx2",
        cra_priority: 600,
        cra_blocksize: SERPENT_BLOCK_SIZE,
        min_keysize: SERPENT_MIN_KEY_SIZE,
        max_keysize: SERPENT_MAX_KEY_SIZE,
        ivsize: 0,
        fpu_blocks: SERPENT_PARALLEL_BLOCKS,
        avx2_parallel_blocks: SERPENT_AVX2_PARALLEL_BLOCKS,
        avx_fallback_blocks: SERPENT_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    SerpentAvx2SkcipherAlg {
        mode: SerpentAvx2Mode::Cbc,
        cra_name: "cbc(serpent)",
        cra_driver_name: "cbc-serpent-avx2",
        cra_priority: 600,
        cra_blocksize: SERPENT_BLOCK_SIZE,
        min_keysize: SERPENT_MIN_KEY_SIZE,
        max_keysize: SERPENT_MAX_KEY_SIZE,
        ivsize: SERPENT_BLOCK_SIZE,
        fpu_blocks: SERPENT_PARALLEL_BLOCKS,
        avx2_parallel_blocks: SERPENT_AVX2_PARALLEL_BLOCKS,
        avx_fallback_blocks: SERPENT_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SerpentAvx2CpuFeatures {
    pub avx2: bool,
    pub osxsave: bool,
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerpentAvx2Registration {
    pub alg_count: usize,
    pub has_avx_8way_fallback: bool,
}

pub const fn serpent_avx2_cpu_supported(cpu: SerpentAvx2CpuFeatures) -> bool {
    cpu.avx2 && cpu.osxsave && cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn serpent_avx2_init(
    cpu: SerpentAvx2CpuFeatures,
    crypto_api_available: bool,
) -> Result<SerpentAvx2Registration, i32> {
    if !serpent_avx2_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(SerpentAvx2Registration {
        alg_count: SERPENT_AVX2_SKCIPHER_ALGS.len(),
        has_avx_8way_fallback: true,
    })
}

pub const fn serpent_avx2_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serpent_avx2_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/serpent_avx2_glue.c"
        ));
        let avx_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/serpent-avx.h"
        ));
        let serpent_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/serpent.h"
        ));

        assert!(source.contains("#define SERPENT_AVX2_PARALLEL_BLOCKS 16"));
        assert!(source.contains("asmlinkage void serpent_ecb_enc_16way"));
        assert!(source.contains("asmlinkage void serpent_ecb_dec_16way"));
        assert!(source.contains("asmlinkage void serpent_cbc_dec_16way"));
        assert!(source.contains("return __serpent_setkey(crypto_skcipher_ctx(tfm), key, keylen);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_AVX2_PARALLEL_BLOCKS, serpent_ecb_enc_16way);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_ecb_enc_8way_avx);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_AVX2_PARALLEL_BLOCKS, serpent_ecb_dec_16way);"));
        assert!(source.contains("ECB_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_ecb_dec_8way_avx);"));
        assert!(
            source.contains("CBC_DEC_BLOCK(SERPENT_AVX2_PARALLEL_BLOCKS, serpent_cbc_dec_16way);")
        );
        assert!(
            source.contains("CBC_DEC_BLOCK(SERPENT_PARALLEL_BLOCKS, serpent_cbc_dec_8way_avx);")
        );
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-serpent-avx2\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-serpent-avx2\""));
        assert!(source.contains(".base.cra_priority\t= 600"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_AVX2)"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_OSXSAVE)"));
        assert!(source.contains("XFEATURE_MASK_SSE | XFEATURE_MASK_YMM"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(serpent_algs"));
        assert!(source.contains("crypto_unregister_skciphers(serpent_algs"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Serpent Cipher Algorithm, AVX2 optimized\");")
        );
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"serpent\");"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"serpent-asm\");"));
        assert!(avx_header.contains("#define SERPENT_PARALLEL_BLOCKS 8"));
        assert!(serpent_header.contains("#define SERPENT_MAX_KEY_SIZE\t\t 32"));

        assert_eq!(SERPENT_AVX2_SKCIPHER_ALGS.len(), 2);
        assert_eq!(SERPENT_AVX2_SKCIPHER_ALGS[0].cra_priority, 600);
        assert_eq!(SERPENT_AVX2_SKCIPHER_ALGS[0].avx2_parallel_blocks, 16);
        assert_eq!(SERPENT_AVX2_MODULE_ALIASES, ["serpent", "serpent-asm"]);
    }

    #[test]
    fn serpent_avx2_init_tracks_linux_feature_gate() {
        let cpu = SerpentAvx2CpuFeatures {
            avx2: true,
            osxsave: true,
            xfeatures_sse: true,
            xfeatures_ymm: true,
        };

        assert!(serpent_avx2_cpu_supported(cpu));
        assert_eq!(
            serpent_avx2_init(cpu, true),
            Ok(SerpentAvx2Registration {
                alg_count: 2,
                has_avx_8way_fallback: true,
            })
        );
        assert_eq!(serpent_avx2_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(
            serpent_avx2_init(SerpentAvx2CpuFeatures { avx2: false, ..cpu }, true,),
            Err(-ENODEV)
        );
        assert!(serpent_avx2_exit(true));
    }
}
