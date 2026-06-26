//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/aria_aesni_avx2_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/aria_aesni_avx2_glue.c
//! ARIA AVX2/AES-NI/GFNI glue metadata and validation.

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const ARIA_MIN_KEY_SIZE: usize = 16;
pub const ARIA_MAX_KEY_SIZE: usize = 32;
pub const ARIA_BLOCK_SIZE: usize = 16;
pub const ARIA_AESNI_PARALLEL_BLOCKS: usize = 16;
pub const ARIA_AESNI_PARALLEL_BLOCK_SIZE: usize = ARIA_BLOCK_SIZE * ARIA_AESNI_PARALLEL_BLOCKS;
pub const ARIA_AESNI_AVX2_PARALLEL_BLOCKS: usize = 32;
pub const ARIA_AESNI_AVX2_PARALLEL_BLOCK_SIZE: usize =
    ARIA_BLOCK_SIZE * ARIA_AESNI_AVX2_PARALLEL_BLOCKS;
pub const CRYPTO_ALG_SKCIPHER_REQSIZE_LARGE: u32 = 0x0000_4000;

pub const ARIA_AVX2_DESCRIPTION: &str = "ARIA Cipher Algorithm, AVX2/AES-NI/GFNI optimized";
pub const ARIA_AVX2_MODULE_ALIASES: [&str; 2] = ["aria", "aria-aesni-avx2"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvx2Mode {
    Ecb,
    Ctr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvx2Backend {
    Aesni,
    Gfni,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvx2SkcipherAlg {
    pub mode: AriaAvx2Mode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_flags: u32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub chunksize: usize,
    pub request_ctx_size: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
    pub has_init: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvx2BackendOps {
    pub encrypt_16way: &'static str,
    pub decrypt_16way: &'static str,
    pub ctr_crypt_16way: &'static str,
    pub encrypt_32way: &'static str,
    pub decrypt_32way: &'static str,
    pub ctr_crypt_32way: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvx2CtrStep {
    Parallel32Way,
    Parallel16Way,
    FullBlock,
    FirstWalkTail,
}

pub const ARIA_AVX2_SKCIPHER_ALGS: [AriaAvx2SkcipherAlg; 2] = [
    AriaAvx2SkcipherAlg {
        mode: AriaAvx2Mode::Ecb,
        cra_name: "ecb(aria)",
        cra_driver_name: "ecb-aria-avx2",
        cra_priority: 500,
        cra_flags: 0,
        cra_blocksize: ARIA_BLOCK_SIZE,
        min_keysize: ARIA_MIN_KEY_SIZE,
        max_keysize: ARIA_MAX_KEY_SIZE,
        ivsize: 0,
        chunksize: 0,
        request_ctx_size: 0,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
        has_init: false,
    },
    AriaAvx2SkcipherAlg {
        mode: AriaAvx2Mode::Ctr,
        cra_name: "ctr(aria)",
        cra_driver_name: "ctr-aria-avx2",
        cra_priority: 500,
        cra_flags: CRYPTO_ALG_SKCIPHER_REQSIZE_LARGE,
        cra_blocksize: 1,
        min_keysize: ARIA_MIN_KEY_SIZE,
        max_keysize: ARIA_MAX_KEY_SIZE,
        ivsize: ARIA_BLOCK_SIZE,
        chunksize: ARIA_BLOCK_SIZE,
        request_ctx_size: ARIA_AESNI_AVX2_PARALLEL_BLOCK_SIZE,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
        has_init: true,
    },
];

pub const ARIA_AVX2_CTR_STEPS: [AriaAvx2CtrStep; 4] = [
    AriaAvx2CtrStep::Parallel32Way,
    AriaAvx2CtrStep::Parallel16Way,
    AriaAvx2CtrStep::FullBlock,
    AriaAvx2CtrStep::FirstWalkTail,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AriaAvx2CpuFeatures {
    pub avx: bool,
    pub avx2: bool,
    pub aes: bool,
    pub osxsave: bool,
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
    pub gfni: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvx2Registration {
    pub alg_count: usize,
    pub backend: AriaAvx2Backend,
    pub has_16way_fallback: bool,
    pub has_32way_parallel: bool,
}

pub const fn aria_avx2_keylen_valid(keylen: usize) -> Result<(), i32> {
    match keylen {
        16 | 24 | 32 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn aria_avx2_cpu_supported(cpu: AriaAvx2CpuFeatures) -> bool {
    cpu.avx && cpu.avx2 && cpu.aes && cpu.osxsave && cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn aria_avx2_backend(cpu: AriaAvx2CpuFeatures) -> Option<AriaAvx2Backend> {
    if !aria_avx2_cpu_supported(cpu) {
        None
    } else if cpu.gfni {
        Some(AriaAvx2Backend::Gfni)
    } else {
        Some(AriaAvx2Backend::Aesni)
    }
}

pub const fn aria_avx2_backend_ops(backend: AriaAvx2Backend) -> AriaAvx2BackendOps {
    match backend {
        AriaAvx2Backend::Aesni => AriaAvx2BackendOps {
            encrypt_16way: "aria_aesni_avx_encrypt_16way",
            decrypt_16way: "aria_aesni_avx_decrypt_16way",
            ctr_crypt_16way: "aria_aesni_avx_ctr_crypt_16way",
            encrypt_32way: "aria_aesni_avx2_encrypt_32way",
            decrypt_32way: "aria_aesni_avx2_decrypt_32way",
            ctr_crypt_32way: "aria_aesni_avx2_ctr_crypt_32way",
        },
        AriaAvx2Backend::Gfni => AriaAvx2BackendOps {
            encrypt_16way: "aria_aesni_avx_gfni_encrypt_16way",
            decrypt_16way: "aria_aesni_avx_gfni_decrypt_16way",
            ctr_crypt_16way: "aria_aesni_avx_gfni_ctr_crypt_16way",
            encrypt_32way: "aria_aesni_avx2_gfni_encrypt_32way",
            decrypt_32way: "aria_aesni_avx2_gfni_decrypt_32way",
            ctr_crypt_32way: "aria_aesni_avx2_gfni_ctr_crypt_32way",
        },
    }
}

pub const fn aria_avx2_init_plan(
    cpu: AriaAvx2CpuFeatures,
    crypto_register_errno: i32,
) -> Result<AriaAvx2Registration, i32> {
    let Some(backend) = aria_avx2_backend(cpu) else {
        return Err(-ENODEV);
    };
    if crypto_register_errno != 0 {
        return Err(crypto_register_errno);
    }
    Ok(AriaAvx2Registration {
        alg_count: ARIA_AVX2_SKCIPHER_ALGS.len(),
        backend,
        has_16way_fallback: true,
        has_32way_parallel: true,
    })
}

pub const fn aria_avx2_init(
    cpu: AriaAvx2CpuFeatures,
    crypto_api_available: bool,
) -> Result<AriaAvx2Registration, i32> {
    let crypto_errno = if crypto_api_available { 0 } else { -EOPNOTSUPP };
    aria_avx2_init_plan(cpu, crypto_errno)
}

pub const fn aria_avx2_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aria_avx2_registration_matches_linux_source_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/aria_aesni_avx2_glue.c"
        ));
        let avx_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/aria-avx.h"
        ));
        let aria_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/aria.h"
        ));
        let helper = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/ecb_cbc_helpers.h"
        ));
        let skcipher_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/internal/skcipher.h"
        ));
        let crypto_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/crypto.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        let testmgr_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("aria_aesni_avx2_encrypt_32way"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(aria_aesni_avx2_gfni_ctr_crypt_32way);"));
        assert!(source.contains("ECB_BLOCK(ARIA_AESNI_AVX2_PARALLEL_BLOCKS"));
        assert!(source.contains("ECB_BLOCK(ARIA_AESNI_PARALLEL_BLOCKS"));
        assert!(source.contains("ECB_BLOCK(1, aria_encrypt);"));
        assert!(source.contains("ECB_BLOCK(1, aria_decrypt);"));
        assert!(source.contains("while (nbytes >= ARIA_AESNI_AVX2_PARALLEL_BLOCK_SIZE)"));
        assert!(source.contains("while (nbytes >= ARIA_AESNI_PARALLEL_BLOCK_SIZE)"));
        assert!(source.contains("crypto_inc(walk.iv, ARIA_BLOCK_SIZE);"));
        assert!(source.contains("crypto_xor_cpy(dst, src, &req_ctx->keystream[0]"));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-aria-avx2\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ctr-aria-avx2\""));
        assert!(source.contains(".base.cra_priority\t= 500"));
        assert!(source.contains(".base.cra_flags\t\t= CRYPTO_ALG_SKCIPHER_REQSIZE_LARGE"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_AVX2)"));
        assert!(source.contains("boot_cpu_has(X86_FEATURE_GFNI)"));
        assert!(source.contains("crypto_register_skciphers(aria_algs"));
        assert!(source.contains("crypto_unregister_skciphers(aria_algs"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"aria-aesni-avx2\");"));
        assert!(avx_header.contains("#define ARIA_AESNI_AVX2_PARALLEL_BLOCKS 32"));
        assert!(aria_header.contains("#define ARIA_MAX_KEY_SIZE\t32"));
        assert!(helper.contains("#define ECB_BLOCK(blocks, func)"));
        assert!(
            skcipher_header
                .contains("#define CRYPTO_ALG_SKCIPHER_REQSIZE_LARGE CRYPTO_ALG_OPTIONAL_KEY")
        );
        assert!(crypto_header.contains("#define CRYPTO_ALG_OPTIONAL_KEY\t\t0x00004000"));
        assert!(testmgr_c.contains(".alg = \"ecb(aria)\""));
        assert!(testmgr_c.contains(".alg = \"ctr(aria)\""));
        assert!(testmgr_h.contains("static const struct cipher_testvec aria_tv_template[]"));
        assert!(testmgr_h.contains("static const struct cipher_testvec aria_ctr_tv_template[]"));

        assert_eq!(ARIA_AVX2_SKCIPHER_ALGS[0].cra_driver_name, "ecb-aria-avx2");
        assert_eq!(
            ARIA_AVX2_SKCIPHER_ALGS[1].cra_flags,
            CRYPTO_ALG_SKCIPHER_REQSIZE_LARGE
        );
        assert_eq!(ARIA_AVX2_SKCIPHER_ALGS[1].request_ctx_size, 32 * 16);
        assert!(ARIA_AVX2_SKCIPHER_ALGS[1].has_init);
        assert_eq!(
            aria_avx2_backend_ops(AriaAvx2Backend::Gfni).ctr_crypt_32way,
            "aria_aesni_avx2_gfni_ctr_crypt_32way"
        );
        assert_eq!(
            ARIA_AVX2_CTR_STEPS,
            [
                AriaAvx2CtrStep::Parallel32Way,
                AriaAvx2CtrStep::Parallel16Way,
                AriaAvx2CtrStep::FullBlock,
                AriaAvx2CtrStep::FirstWalkTail
            ]
        );
        assert_eq!(ARIA_AVX2_MODULE_ALIASES, ["aria", "aria-aesni-avx2"]);
    }

    #[test]
    fn aria_avx2_validation_and_dispatch_track_linux_branches() {
        assert_eq!(aria_avx2_keylen_valid(16), Ok(()));
        assert_eq!(aria_avx2_keylen_valid(24), Ok(()));
        assert_eq!(aria_avx2_keylen_valid(32), Ok(()));
        assert_eq!(aria_avx2_keylen_valid(40), Err(-EINVAL));

        let cpu = AriaAvx2CpuFeatures {
            avx: true,
            avx2: true,
            aes: true,
            osxsave: true,
            xfeatures_sse: true,
            xfeatures_ymm: true,
            gfni: false,
        };
        assert_eq!(aria_avx2_backend(cpu), Some(AriaAvx2Backend::Aesni));
        assert_eq!(
            aria_avx2_backend(AriaAvx2CpuFeatures { gfni: true, ..cpu }),
            Some(AriaAvx2Backend::Gfni)
        );
        assert_eq!(
            aria_avx2_init(cpu, true),
            Ok(AriaAvx2Registration {
                alg_count: 2,
                backend: AriaAvx2Backend::Aesni,
                has_16way_fallback: true,
                has_32way_parallel: true,
            })
        );
        assert_eq!(aria_avx2_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(aria_avx2_init_plan(cpu, -5), Err(-5));
        assert_eq!(
            aria_avx2_init(AriaAvx2CpuFeatures { avx2: false, ..cpu }, true,),
            Err(-ENODEV)
        );
        assert!(aria_avx2_exit(true));
    }
}
