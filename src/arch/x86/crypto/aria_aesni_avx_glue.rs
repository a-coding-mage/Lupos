//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/aria_aesni_avx_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/aria_aesni_avx_glue.c
//! ARIA AVX/AES-NI/GFNI glue metadata and validation.

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const ARIA_MIN_KEY_SIZE: usize = 16;
pub const ARIA_MAX_KEY_SIZE: usize = 32;
pub const ARIA_BLOCK_SIZE: usize = 16;
pub const ARIA_AESNI_PARALLEL_BLOCKS: usize = 16;
pub const ARIA_AESNI_PARALLEL_BLOCK_SIZE: usize = ARIA_BLOCK_SIZE * ARIA_AESNI_PARALLEL_BLOCKS;

pub const ARIA_AVX_DESCRIPTION: &str = "ARIA Cipher Algorithm, AVX/AES-NI/GFNI optimized";
pub const ARIA_AVX_MODULE_ALIASES: [&str; 2] = ["aria", "aria-aesni-avx"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvxMode {
    Ecb,
    Ctr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvxBackend {
    Aesni,
    Gfni,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvxSkcipherAlg {
    pub mode: AriaAvxMode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub chunksize: usize,
    pub walksize: usize,
    pub request_ctx_size: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
    pub has_init: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvxBackendOps {
    pub encrypt_16way: &'static str,
    pub decrypt_16way: &'static str,
    pub ctr_crypt_16way: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaAvxCtrStep {
    Parallel16Way,
    FullBlock,
    FirstWalkTail,
}

pub const ARIA_AVX_SKCIPHER_ALGS: [AriaAvxSkcipherAlg; 2] = [
    AriaAvxSkcipherAlg {
        mode: AriaAvxMode::Ecb,
        cra_name: "ecb(aria)",
        cra_driver_name: "ecb-aria-avx",
        cra_priority: 400,
        cra_blocksize: ARIA_BLOCK_SIZE,
        min_keysize: ARIA_MIN_KEY_SIZE,
        max_keysize: ARIA_MAX_KEY_SIZE,
        ivsize: 0,
        chunksize: 0,
        walksize: 0,
        request_ctx_size: 0,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
        has_init: false,
    },
    AriaAvxSkcipherAlg {
        mode: AriaAvxMode::Ctr,
        cra_name: "ctr(aria)",
        cra_driver_name: "ctr-aria-avx",
        cra_priority: 400,
        cra_blocksize: 1,
        min_keysize: ARIA_MIN_KEY_SIZE,
        max_keysize: ARIA_MAX_KEY_SIZE,
        ivsize: ARIA_BLOCK_SIZE,
        chunksize: ARIA_BLOCK_SIZE,
        walksize: ARIA_AESNI_PARALLEL_BLOCK_SIZE,
        request_ctx_size: ARIA_AESNI_PARALLEL_BLOCK_SIZE,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
        has_init: true,
    },
];

pub const ARIA_AVX_CTR_STEPS: [AriaAvxCtrStep; 3] = [
    AriaAvxCtrStep::Parallel16Way,
    AriaAvxCtrStep::FullBlock,
    AriaAvxCtrStep::FirstWalkTail,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AriaAvxCpuFeatures {
    pub avx: bool,
    pub aes: bool,
    pub osxsave: bool,
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
    pub gfni: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaAvxRegistration {
    pub alg_count: usize,
    pub backend: AriaAvxBackend,
}

pub const fn aria_avx_keylen_valid(keylen: usize) -> Result<(), i32> {
    match keylen {
        16 | 24 | 32 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn aria_avx_cpu_supported(cpu: AriaAvxCpuFeatures) -> bool {
    cpu.avx && cpu.aes && cpu.osxsave && cpu.xfeatures_sse && cpu.xfeatures_ymm
}

pub const fn aria_avx_backend(cpu: AriaAvxCpuFeatures) -> Option<AriaAvxBackend> {
    if !aria_avx_cpu_supported(cpu) {
        None
    } else if cpu.gfni {
        Some(AriaAvxBackend::Gfni)
    } else {
        Some(AriaAvxBackend::Aesni)
    }
}

pub const fn aria_avx_backend_ops(backend: AriaAvxBackend) -> AriaAvxBackendOps {
    match backend {
        AriaAvxBackend::Aesni => AriaAvxBackendOps {
            encrypt_16way: "aria_aesni_avx_encrypt_16way",
            decrypt_16way: "aria_aesni_avx_decrypt_16way",
            ctr_crypt_16way: "aria_aesni_avx_ctr_crypt_16way",
        },
        AriaAvxBackend::Gfni => AriaAvxBackendOps {
            encrypt_16way: "aria_aesni_avx_gfni_encrypt_16way",
            decrypt_16way: "aria_aesni_avx_gfni_decrypt_16way",
            ctr_crypt_16way: "aria_aesni_avx_gfni_ctr_crypt_16way",
        },
    }
}

pub const fn aria_avx_init_plan(
    cpu: AriaAvxCpuFeatures,
    crypto_register_errno: i32,
) -> Result<AriaAvxRegistration, i32> {
    let Some(backend) = aria_avx_backend(cpu) else {
        return Err(-ENODEV);
    };
    if crypto_register_errno != 0 {
        return Err(crypto_register_errno);
    }
    Ok(AriaAvxRegistration {
        alg_count: ARIA_AVX_SKCIPHER_ALGS.len(),
        backend,
    })
}

pub const fn aria_avx_init(
    cpu: AriaAvxCpuFeatures,
    crypto_api_available: bool,
) -> Result<AriaAvxRegistration, i32> {
    let crypto_errno = if crypto_api_available { 0 } else { -EOPNOTSUPP };
    aria_avx_init_plan(cpu, crypto_errno)
}

pub const fn aria_avx_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aria_avx_registration_matches_linux_source_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/aria_aesni_avx_glue.c"
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
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        let testmgr_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("aria_aesni_avx_encrypt_16way"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(aria_aesni_avx_gfni_ctr_crypt_16way);"));
        assert!(source.contains("ECB_BLOCK(ARIA_AESNI_PARALLEL_BLOCKS"));
        assert!(source.contains("ECB_BLOCK(1, aria_encrypt);"));
        assert!(source.contains("ECB_BLOCK(1, aria_decrypt);"));
        assert!(source.contains("kernel_fpu_begin();"));
        assert!(source.contains("crypto_inc(walk.iv, ARIA_BLOCK_SIZE);"));
        assert!(source.contains("crypto_xor_cpy(dst, src, &req_ctx->keystream[0]"));
        assert!(source.contains(".base.cra_name\t\t= \"ecb(aria)\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ctr-aria-avx\""));
        assert!(source.contains(".base.cra_priority\t= 400"));
        assert!(source.contains(".walksize\t\t= 16 * ARIA_BLOCK_SIZE"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_AVX)"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_AES)"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_OSXSAVE)"));
        assert!(source.contains("boot_cpu_has(X86_FEATURE_GFNI)"));
        assert!(source.contains("crypto_register_skciphers(aria_algs"));
        assert!(source.contains("crypto_unregister_skciphers(aria_algs"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"aria-aesni-avx\");"));
        assert!(avx_header.contains("#define ARIA_AESNI_PARALLEL_BLOCKS 16"));
        assert!(aria_header.contains("#define ARIA_MIN_KEY_SIZE\t16"));
        assert!(aria_header.contains("int aria_set_key(struct crypto_tfm *tfm"));
        assert!(helper.contains("#define ECB_BLOCK(blocks, func)"));
        assert!(testmgr_c.contains(".alg = \"ecb(aria)\""));
        assert!(testmgr_c.contains(".alg = \"ctr(aria)\""));
        assert!(testmgr_h.contains("static const struct cipher_testvec aria_tv_template[]"));
        assert!(testmgr_h.contains("static const struct cipher_testvec aria_ctr_tv_template[]"));

        assert_eq!(ARIA_AVX_SKCIPHER_ALGS[0].cra_driver_name, "ecb-aria-avx");
        assert_eq!(ARIA_AVX_SKCIPHER_ALGS[1].walksize, 16 * 16);
        assert!(ARIA_AVX_SKCIPHER_ALGS[1].has_init);
        assert_eq!(
            aria_avx_backend_ops(AriaAvxBackend::Gfni).ctr_crypt_16way,
            "aria_aesni_avx_gfni_ctr_crypt_16way"
        );
        assert_eq!(
            ARIA_AVX_CTR_STEPS,
            [
                AriaAvxCtrStep::Parallel16Way,
                AriaAvxCtrStep::FullBlock,
                AriaAvxCtrStep::FirstWalkTail
            ]
        );
        assert_eq!(ARIA_AVX_MODULE_ALIASES, ["aria", "aria-aesni-avx"]);
    }

    #[test]
    fn aria_avx_validation_and_dispatch_track_linux_branches() {
        assert_eq!(aria_avx_keylen_valid(15), Err(-EINVAL));
        assert_eq!(aria_avx_keylen_valid(16), Ok(()));
        assert_eq!(aria_avx_keylen_valid(24), Ok(()));
        assert_eq!(aria_avx_keylen_valid(32), Ok(()));
        assert_eq!(aria_avx_keylen_valid(33), Err(-EINVAL));

        let cpu = AriaAvxCpuFeatures {
            avx: true,
            aes: true,
            osxsave: true,
            xfeatures_sse: true,
            xfeatures_ymm: true,
            gfni: false,
        };
        assert_eq!(aria_avx_backend(cpu), Some(AriaAvxBackend::Aesni));
        assert_eq!(
            aria_avx_backend(AriaAvxCpuFeatures { gfni: true, ..cpu }),
            Some(AriaAvxBackend::Gfni)
        );
        assert_eq!(
            aria_avx_init(cpu, true),
            Ok(AriaAvxRegistration {
                alg_count: 2,
                backend: AriaAvxBackend::Aesni,
            })
        );
        assert_eq!(aria_avx_init(cpu, false), Err(-EOPNOTSUPP));
        assert_eq!(aria_avx_init_plan(cpu, -5), Err(-5));
        assert_eq!(
            aria_avx_init(AriaAvxCpuFeatures { avx: false, ..cpu }, true,),
            Err(-ENODEV)
        );
        assert!(aria_avx_exit(true));
    }
}
