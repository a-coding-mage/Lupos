//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/aegis128-aesni-glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/aegis128-aesni-glue.c
//! AEGIS-128 AES-NI/SSE4.1 glue metadata and validation.

use crate::include::uapi::errno::{EBADMSG, EINVAL, ENODEV, EOPNOTSUPP};

pub const AEGIS128_BLOCK_ALIGN: usize = 16;
pub const AEGIS128_BLOCK_SIZE: usize = 16;
pub const AEGIS128_NONCE_SIZE: usize = 16;
pub const AEGIS128_STATE_BLOCKS: usize = 5;
pub const AEGIS128_KEY_SIZE: usize = 16;
pub const AEGIS128_MIN_AUTH_SIZE: usize = 8;
pub const AEGIS128_MAX_AUTH_SIZE: usize = 16;

pub const AEGIS128_CRA_NAME: &str = "aegis128";
pub const AEGIS128_DRIVER_NAME: &str = "aegis128-aesni";
pub const AEGIS128_PRIORITY: i32 = 400;
pub const AEGIS128_CRA_BLOCKSIZE: usize = 1;
pub const AEGIS128_DESCRIPTION: &str = "AEGIS-128 AEAD algorithm -- AESNI+SSE4.1 implementation";
pub const AEGIS128_MODULE_ALIASES: [&str; 2] = ["aegis128", "aegis128-aesni"];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AegisBlock {
    pub bytes: [u8; AEGIS128_BLOCK_SIZE],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AegisState {
    pub blocks: [AegisBlock; AEGIS128_STATE_BLOCKS],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AegisCtx {
    pub key: AegisBlock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Aegis128AesniAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub ivsize: usize,
    pub maxauthsize: usize,
    pub chunksize: usize,
    pub has_setkey: bool,
    pub has_setauthsize: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const AEGIS128_AESNI_ALG: Aegis128AesniAlg = Aegis128AesniAlg {
    cra_name: AEGIS128_CRA_NAME,
    cra_driver_name: AEGIS128_DRIVER_NAME,
    cra_priority: AEGIS128_PRIORITY,
    cra_blocksize: AEGIS128_CRA_BLOCKSIZE,
    ivsize: AEGIS128_NONCE_SIZE,
    maxauthsize: AEGIS128_MAX_AUTH_SIZE,
    chunksize: AEGIS128_BLOCK_SIZE,
    has_setkey: true,
    has_setauthsize: true,
    has_encrypt: true,
    has_decrypt: true,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Aegis128AesniCpuFeatures {
    pub xmm4_1: bool,
    pub aes: bool,
    pub xfeatures_sse: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Aegis128AesniCryptMode {
    Encrypt,
    Decrypt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Aegis128AesniRequestPlan {
    pub mode: Aegis128AesniCryptMode,
    pub assoclen: usize,
    pub input_cryptlen: usize,
    pub payload_cryptlen: usize,
    pub authsize: usize,
    pub ad_blocks: usize,
    pub crypt_full_bytes: usize,
    pub crypt_tail_bytes: usize,
    pub tag_offset: usize,
    pub writes_tag: bool,
    pub verifies_tag: bool,
    pub auth_failure_errno: Option<i32>,
}

pub fn crypto_aegis128_aesni_setkey(key: &[u8]) -> Result<AegisCtx, i32> {
    if key.len() != AEGIS128_KEY_SIZE {
        return Err(-EINVAL);
    }

    let mut ctx = AegisCtx::default();
    ctx.key.bytes.copy_from_slice(key);
    Ok(ctx)
}

pub const fn crypto_aegis128_aesni_setauthsize(authsize: usize) -> Result<(), i32> {
    if authsize > AEGIS128_MAX_AUTH_SIZE || authsize < AEGIS128_MIN_AUTH_SIZE {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn aegis128_ad_blocks_for_len(assoclen: usize) -> usize {
    if assoclen == 0 {
        0
    } else {
        (assoclen + AEGIS128_BLOCK_SIZE - 1) / AEGIS128_BLOCK_SIZE
    }
}

pub const fn aegis128_crypt_partition(nbytes: usize) -> (usize, usize) {
    (
        nbytes & !(AEGIS128_BLOCK_SIZE - 1),
        nbytes & (AEGIS128_BLOCK_SIZE - 1),
    )
}

pub const fn crypto_aegis128_aesni_request_plan(
    mode: Aegis128AesniCryptMode,
    assoclen: usize,
    cryptlen: usize,
    authsize: usize,
) -> Result<Aegis128AesniRequestPlan, i32> {
    if authsize > AEGIS128_MAX_AUTH_SIZE || authsize < AEGIS128_MIN_AUTH_SIZE {
        return Err(-EINVAL);
    }

    let payload_cryptlen = match mode {
        Aegis128AesniCryptMode::Encrypt => cryptlen,
        Aegis128AesniCryptMode::Decrypt => {
            if cryptlen < authsize {
                return Err(-EINVAL);
            }
            cryptlen - authsize
        }
    };
    let (crypt_full_bytes, crypt_tail_bytes) = aegis128_crypt_partition(payload_cryptlen);
    Ok(Aegis128AesniRequestPlan {
        mode,
        assoclen,
        input_cryptlen: cryptlen,
        payload_cryptlen,
        authsize,
        ad_blocks: aegis128_ad_blocks_for_len(assoclen),
        crypt_full_bytes,
        crypt_tail_bytes,
        tag_offset: assoclen + payload_cryptlen,
        writes_tag: matches!(mode, Aegis128AesniCryptMode::Encrypt),
        verifies_tag: matches!(mode, Aegis128AesniCryptMode::Decrypt),
        auth_failure_errno: match mode {
            Aegis128AesniCryptMode::Encrypt => None,
            Aegis128AesniCryptMode::Decrypt => Some(-EBADMSG),
        },
    })
}

pub const fn crypto_aegis128_aesni_cpu_supported(cpu: Aegis128AesniCpuFeatures) -> bool {
    cpu.xmm4_1 && cpu.aes && cpu.xfeatures_sse
}

pub const fn crypto_aegis128_aesni_module_init_plan(
    cpu: Aegis128AesniCpuFeatures,
    crypto_register_errno: i32,
) -> Result<Aegis128AesniAlg, i32> {
    if !crypto_aegis128_aesni_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if crypto_register_errno != 0 {
        return Err(crypto_register_errno);
    }
    Ok(AEGIS128_AESNI_ALG)
}

pub const fn crypto_aegis128_aesni_module_init(
    cpu: Aegis128AesniCpuFeatures,
    crypto_api_available: bool,
) -> Result<Aegis128AesniAlg, i32> {
    let crypto_errno = if crypto_api_available { 0 } else { -EOPNOTSUPP };
    crypto_aegis128_aesni_module_init_plan(cpu, crypto_errno)
}

pub const fn crypto_aegis128_aesni_module_exit(registered: bool) -> bool {
    !registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aegis128_aesni_registration_matches_linux_source_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/aegis128-aesni-glue.c"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        let testmgr_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));

        assert!(source.contains("#define AEGIS128_BLOCK_ALIGN 16"));
        assert!(source.contains("#define AEGIS128_STATE_BLOCKS 5"));
        assert!(source.contains("#define AEGIS128_MIN_AUTH_SIZE 8"));
        assert!(source.contains("asmlinkage void aegis128_aesni_init"));
        assert!(source.contains("scatterwalk_start(&walk, sg_src);"));
        assert!(source.contains("aegis128_aesni_ad(state, buf.bytes"));
        assert!(source.contains("aegis128_aesni_enc(state, walk->src.virt.addr"));
        assert!(source.contains("aegis128_aesni_dec_tail(state, walk->src.virt.addr"));
        assert!(source.contains("aegis128_aesni_final(&state, tag_xor"));
        assert!(source.contains("scatterwalk_map_and_copy(tag.bytes, req->dst"));
        assert!(
            source
                .contains("return crypto_memneq(tag.bytes, zeros.bytes, authsize) ? -EBADMSG : 0;")
        );
        assert!(source.contains(".cra_name = \"aegis128\""));
        assert!(source.contains(".cra_driver_name = \"aegis128-aesni\""));
        assert!(source.contains(".cra_priority = 400"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_XMM4_1)"));
        assert!(source.contains("!boot_cpu_has(X86_FEATURE_AES)"));
        assert!(source.contains("!cpu_has_xfeatures(XFEATURE_MASK_SSE, NULL)"));
        assert!(source.contains("return crypto_register_aead(&crypto_aegis128_aesni_alg);"));
        assert!(source.contains("crypto_unregister_aead(&crypto_aegis128_aesni_alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"aegis128-aesni\");"));
        assert!(testmgr_c.contains(".alg = \"aegis128\""));
        assert!(testmgr_c.contains(".aead = __VECS(aegis128_tv_template)"));
        assert!(testmgr_h.contains("static const struct aead_testvec aegis128_tv_template[]"));

        assert_eq!(AEGIS128_AESNI_ALG.cra_name, "aegis128");
        assert_eq!(AEGIS128_AESNI_ALG.cra_driver_name, "aegis128-aesni");
        assert_eq!(AEGIS128_AESNI_ALG.cra_priority, 400);
        assert_eq!(AEGIS128_AESNI_ALG.ivsize, 16);
        assert_eq!(AEGIS128_AESNI_ALG.maxauthsize, 16);
        assert_eq!(AEGIS128_AESNI_ALG.chunksize, 16);
        assert_eq!(AEGIS128_MODULE_ALIASES, ["aegis128", "aegis128-aesni"]);
    }

    #[test]
    fn aegis128_validation_and_cpu_gate_match_glue_logic() {
        assert_eq!(crypto_aegis128_aesni_setkey(&[0u8; 15]), Err(-EINVAL));
        let ctx = crypto_aegis128_aesni_setkey(&[0x42u8; 16]).expect("valid key");
        assert_eq!(ctx.key.bytes, [0x42u8; 16]);

        assert_eq!(crypto_aegis128_aesni_setauthsize(7), Err(-EINVAL));
        assert_eq!(crypto_aegis128_aesni_setauthsize(8), Ok(()));
        assert_eq!(crypto_aegis128_aesni_setauthsize(16), Ok(()));
        assert_eq!(crypto_aegis128_aesni_setauthsize(17), Err(-EINVAL));
        assert_eq!(aegis128_ad_blocks_for_len(0), 0);
        assert_eq!(aegis128_ad_blocks_for_len(1), 1);
        assert_eq!(aegis128_ad_blocks_for_len(17), 2);
        assert_eq!(aegis128_crypt_partition(31), (16, 15));
        assert_eq!(
            crypto_aegis128_aesni_request_plan(Aegis128AesniCryptMode::Encrypt, 5, 31, 12),
            Ok(Aegis128AesniRequestPlan {
                mode: Aegis128AesniCryptMode::Encrypt,
                assoclen: 5,
                input_cryptlen: 31,
                payload_cryptlen: 31,
                authsize: 12,
                ad_blocks: 1,
                crypt_full_bytes: 16,
                crypt_tail_bytes: 15,
                tag_offset: 36,
                writes_tag: true,
                verifies_tag: false,
                auth_failure_errno: None,
            })
        );
        assert_eq!(
            crypto_aegis128_aesni_request_plan(Aegis128AesniCryptMode::Decrypt, 5, 31, 12),
            Ok(Aegis128AesniRequestPlan {
                mode: Aegis128AesniCryptMode::Decrypt,
                assoclen: 5,
                input_cryptlen: 31,
                payload_cryptlen: 19,
                authsize: 12,
                ad_blocks: 1,
                crypt_full_bytes: 16,
                crypt_tail_bytes: 3,
                tag_offset: 24,
                writes_tag: false,
                verifies_tag: true,
                auth_failure_errno: Some(-EBADMSG),
            })
        );

        let supported = Aegis128AesniCpuFeatures {
            xmm4_1: true,
            aes: true,
            xfeatures_sse: true,
        };
        assert_eq!(
            crypto_aegis128_aesni_module_init(supported, true),
            Ok(AEGIS128_AESNI_ALG)
        );
        assert_eq!(
            crypto_aegis128_aesni_module_init(
                Aegis128AesniCpuFeatures {
                    aes: false,
                    ..supported
                },
                true
            ),
            Err(-ENODEV)
        );
        assert_eq!(
            crypto_aegis128_aesni_module_init(supported, false),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(
            crypto_aegis128_aesni_module_init_plan(supported, -5),
            Err(-5)
        );
        assert!(crypto_aegis128_aesni_module_exit(false));
    }
}
