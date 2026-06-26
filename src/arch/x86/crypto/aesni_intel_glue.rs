//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/crypto/aesni-intel_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/aesni-intel_glue.c
//! AES-NI/VAES glue registration metadata and key/auth validation.

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const AESNI_ALIGN: usize = 16;
pub const AES_BLOCK_SIZE: usize = 16;
pub const AES_MIN_KEY_SIZE: usize = 16;
pub const AES_MAX_KEY_SIZE: usize = 32;
pub const AES_KEYSIZE_128: usize = 16;
pub const AES_KEYSIZE_192: usize = 24;
pub const AES_KEYSIZE_256: usize = 32;
pub const AES_BLOCK_MASK: usize = !(AES_BLOCK_SIZE - 1);

pub const GCM_AES_IV_SIZE: usize = 12;
pub const GCM_RFC4106_IV_SIZE: usize = 8;
pub const AES_GCM_MAX_AUTH_SIZE: usize = 16;

pub const FLAG_RFC4106: u32 = 1 << 0;
pub const FLAG_ENC: u32 = 1 << 1;
pub const FLAG_AVX: u32 = 1 << 2;
pub const FLAG_VAES_AVX2: u32 = 1 << 3;
pub const FLAG_VAES_AVX512: u32 = 1 << 4;

pub const AESNI_DESCRIPTION: &str =
    "AES cipher and modes, optimized with AES-NI or VAES instructions";
pub const AESNI_MODULE_ALIAS: &str = "aes";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AesniSkcipherMode {
    Ecb,
    Cbc,
    CtsCbc,
    Ctr,
    Xts,
    Xctr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AesniImplementation {
    Aesni,
    AesniAvx,
    VaesAvx2,
    VaesAvx512,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesniSkcipherAlg {
    pub mode: AesniSkcipherMode,
    pub implementation: AesniImplementation,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub chunksize: usize,
    pub walksize: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesGcmAlg {
    pub implementation: AesniImplementation,
    pub rfc4106: bool,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub ivsize: usize,
    pub chunksize: usize,
    pub maxauthsize: usize,
}

pub const AESNI_SKCIPHERS: [AesniSkcipherAlg; 5] = [
    AesniSkcipherAlg {
        mode: AesniSkcipherMode::Ecb,
        implementation: AesniImplementation::Aesni,
        cra_name: "ecb(aes)",
        cra_driver_name: "ecb-aes-aesni",
        cra_priority: 400,
        cra_blocksize: AES_BLOCK_SIZE,
        min_keysize: AES_MIN_KEY_SIZE,
        max_keysize: AES_MAX_KEY_SIZE,
        ivsize: 0,
        chunksize: 0,
        walksize: 0,
    },
    AesniSkcipherAlg {
        mode: AesniSkcipherMode::Cbc,
        implementation: AesniImplementation::Aesni,
        cra_name: "cbc(aes)",
        cra_driver_name: "cbc-aes-aesni",
        cra_priority: 400,
        cra_blocksize: AES_BLOCK_SIZE,
        min_keysize: AES_MIN_KEY_SIZE,
        max_keysize: AES_MAX_KEY_SIZE,
        ivsize: AES_BLOCK_SIZE,
        chunksize: 0,
        walksize: 0,
    },
    AesniSkcipherAlg {
        mode: AesniSkcipherMode::CtsCbc,
        implementation: AesniImplementation::Aesni,
        cra_name: "cts(cbc(aes))",
        cra_driver_name: "cts-cbc-aes-aesni",
        cra_priority: 400,
        cra_blocksize: AES_BLOCK_SIZE,
        min_keysize: AES_MIN_KEY_SIZE,
        max_keysize: AES_MAX_KEY_SIZE,
        ivsize: AES_BLOCK_SIZE,
        chunksize: 0,
        walksize: 2 * AES_BLOCK_SIZE,
    },
    AesniSkcipherAlg {
        mode: AesniSkcipherMode::Ctr,
        implementation: AesniImplementation::Aesni,
        cra_name: "ctr(aes)",
        cra_driver_name: "ctr-aes-aesni",
        cra_priority: 400,
        cra_blocksize: 1,
        min_keysize: AES_MIN_KEY_SIZE,
        max_keysize: AES_MAX_KEY_SIZE,
        ivsize: AES_BLOCK_SIZE,
        chunksize: AES_BLOCK_SIZE,
        walksize: 0,
    },
    AesniSkcipherAlg {
        mode: AesniSkcipherMode::Xts,
        implementation: AesniImplementation::Aesni,
        cra_name: "xts(aes)",
        cra_driver_name: "xts-aes-aesni",
        cra_priority: 401,
        cra_blocksize: AES_BLOCK_SIZE,
        min_keysize: 2 * AES_MIN_KEY_SIZE,
        max_keysize: 2 * AES_MAX_KEY_SIZE,
        ivsize: AES_BLOCK_SIZE,
        chunksize: 0,
        walksize: 2 * AES_BLOCK_SIZE,
    },
];

pub const AESNI_AVX_SKCIPHERS: [AesniSkcipherAlg; 3] = avx_skciphers(
    AesniImplementation::AesniAvx,
    "xts-aes-aesni-avx",
    "ctr-aes-aesni-avx",
    "xctr-aes-aesni-avx",
    500,
);
pub const VAES_AVX2_SKCIPHERS: [AesniSkcipherAlg; 3] = avx_skciphers(
    AesniImplementation::VaesAvx2,
    "xts-aes-vaes-avx2",
    "ctr-aes-vaes-avx2",
    "xctr-aes-vaes-avx2",
    600,
);
pub const VAES_AVX512_SKCIPHERS: [AesniSkcipherAlg; 3] = avx_skciphers(
    AesniImplementation::VaesAvx512,
    "xts-aes-vaes-avx512",
    "ctr-aes-vaes-avx512",
    "xctr-aes-vaes-avx512",
    800,
);

pub const AES_GCM_ALGS_AESNI: [AesGcmAlg; 2] = gcm_algs(
    AesniImplementation::Aesni,
    "generic-gcm-aesni",
    "rfc4106-gcm-aesni",
    400,
);
pub const AES_GCM_ALGS_AESNI_AVX: [AesGcmAlg; 2] = gcm_algs(
    AesniImplementation::AesniAvx,
    "generic-gcm-aesni-avx",
    "rfc4106-gcm-aesni-avx",
    500,
);
pub const AES_GCM_ALGS_VAES_AVX2: [AesGcmAlg; 2] = gcm_algs(
    AesniImplementation::VaesAvx2,
    "generic-gcm-vaes-avx2",
    "rfc4106-gcm-vaes-avx2",
    600,
);
pub const AES_GCM_ALGS_VAES_AVX512: [AesGcmAlg; 2] = gcm_algs(
    AesniImplementation::VaesAvx512,
    "generic-gcm-vaes-avx512",
    "rfc4106-gcm-vaes-avx512",
    800,
);

const fn avx_skciphers(
    implementation: AesniImplementation,
    xts_driver_name: &'static str,
    ctr_driver_name: &'static str,
    xctr_driver_name: &'static str,
    priority: i32,
) -> [AesniSkcipherAlg; 3] {
    [
        AesniSkcipherAlg {
            mode: AesniSkcipherMode::Xts,
            implementation,
            cra_name: "xts(aes)",
            cra_driver_name: xts_driver_name,
            cra_priority: priority,
            cra_blocksize: AES_BLOCK_SIZE,
            min_keysize: 2 * AES_MIN_KEY_SIZE,
            max_keysize: 2 * AES_MAX_KEY_SIZE,
            ivsize: AES_BLOCK_SIZE,
            chunksize: 0,
            walksize: 2 * AES_BLOCK_SIZE,
        },
        AesniSkcipherAlg {
            mode: AesniSkcipherMode::Ctr,
            implementation,
            cra_name: "ctr(aes)",
            cra_driver_name: ctr_driver_name,
            cra_priority: priority,
            cra_blocksize: 1,
            min_keysize: AES_MIN_KEY_SIZE,
            max_keysize: AES_MAX_KEY_SIZE,
            ivsize: AES_BLOCK_SIZE,
            chunksize: AES_BLOCK_SIZE,
            walksize: 0,
        },
        AesniSkcipherAlg {
            mode: AesniSkcipherMode::Xctr,
            implementation,
            cra_name: "xctr(aes)",
            cra_driver_name: xctr_driver_name,
            cra_priority: priority,
            cra_blocksize: 1,
            min_keysize: AES_MIN_KEY_SIZE,
            max_keysize: AES_MAX_KEY_SIZE,
            ivsize: AES_BLOCK_SIZE,
            chunksize: AES_BLOCK_SIZE,
            walksize: 0,
        },
    ]
}

const fn gcm_algs(
    implementation: AesniImplementation,
    generic_driver_name: &'static str,
    rfc_driver_name: &'static str,
    priority: i32,
) -> [AesGcmAlg; 2] {
    [
        AesGcmAlg {
            implementation,
            rfc4106: false,
            cra_name: "gcm(aes)",
            cra_driver_name: generic_driver_name,
            cra_priority: priority,
            cra_blocksize: 1,
            ivsize: GCM_AES_IV_SIZE,
            chunksize: AES_BLOCK_SIZE,
            maxauthsize: AES_GCM_MAX_AUTH_SIZE,
        },
        AesGcmAlg {
            implementation,
            rfc4106: true,
            cra_name: "rfc4106(gcm(aes))",
            cra_driver_name: rfc_driver_name,
            cra_priority: priority,
            cra_blocksize: 1,
            ivsize: GCM_RFC4106_IV_SIZE,
            chunksize: AES_BLOCK_SIZE,
            maxauthsize: AES_GCM_MAX_AUTH_SIZE,
        },
    ]
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AesniCpuFeatures {
    pub aes: bool,
    pub avx: bool,
    pub avx2: bool,
    pub vaes: bool,
    pub vpclmulqdq: bool,
    pub pclmulqdq: bool,
    pub avx512bw: bool,
    pub avx512vl: bool,
    pub bmi2: bool,
    pub xfeatures_sse: bool,
    pub xfeatures_ymm: bool,
    pub xfeatures_avx512: bool,
    pub prefer_ymm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AesniRegistration {
    pub base_skciphers: usize,
    pub base_gcm_aeads: usize,
    pub avx_skciphers: usize,
    pub avx_gcm_aeads: usize,
    pub vaes_avx2_skciphers: usize,
    pub vaes_avx2_gcm_aeads: usize,
    pub vaes_avx512_skciphers: usize,
    pub vaes_avx512_gcm_aeads: usize,
    pub vaes_avx512_priority: i32,
}

pub const fn aes_check_keylen(keylen: usize) -> Result<(), i32> {
    match keylen {
        AES_KEYSIZE_128 | AES_KEYSIZE_192 | AES_KEYSIZE_256 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn generic_gcmaes_set_authsize(authsize: usize) -> Result<(), i32> {
    match authsize {
        4 | 8 | 12 | 13 | 14 | 15 | 16 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn common_rfc4106_set_authsize(authsize: usize) -> Result<(), i32> {
    match authsize {
        8 | 12 | 16 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn aes_gcm_keylen_valid(keylen: usize) -> Result<(), i32> {
    aes_check_keylen(keylen)
}

pub const fn aes_rfc4106_keylen_valid(raw_keylen: usize) -> Result<(), i32> {
    if raw_keylen < 4 {
        Err(-EINVAL)
    } else {
        aes_check_keylen(raw_keylen - 4)
    }
}

pub fn xts_verify_key(key: &[u8], fips_enabled: bool, forbid_weak_keys: bool) -> Result<(), i32> {
    if key.len() % 2 != 0 {
        return Err(-EINVAL);
    }
    if fips_enabled && key.len() != 32 && key.len() != 64 {
        return Err(-EINVAL);
    }
    let half = key.len() / 2;
    aes_check_keylen(half)?;
    if (fips_enabled || forbid_weak_keys) && key[..half] == key[half..] {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn aesni_base_cpu_supported(cpu: AesniCpuFeatures) -> bool {
    cpu.aes
}

pub const fn aesni_avx_supported(cpu: AesniCpuFeatures, x86_64: bool) -> bool {
    x86_64 && cpu.avx
}

pub const fn aesni_vaes_avx2_supported(cpu: AesniCpuFeatures, x86_64: bool) -> bool {
    aesni_avx_supported(cpu, x86_64)
        && cpu.avx2
        && cpu.vaes
        && cpu.vpclmulqdq
        && cpu.pclmulqdq
        && cpu.xfeatures_sse
        && cpu.xfeatures_ymm
}

pub const fn aesni_vaes_avx512_supported(cpu: AesniCpuFeatures, x86_64: bool) -> bool {
    aesni_vaes_avx2_supported(cpu, x86_64)
        && cpu.avx512bw
        && cpu.avx512vl
        && cpu.bmi2
        && cpu.xfeatures_avx512
}

pub const fn aesni_effective_priority(
    implementation: AesniImplementation,
    prefer_ymm: bool,
) -> i32 {
    match implementation {
        AesniImplementation::Aesni => 400,
        AesniImplementation::AesniAvx => 500,
        AesniImplementation::VaesAvx2 => 600,
        AesniImplementation::VaesAvx512 => {
            if prefer_ymm {
                1
            } else {
                800
            }
        }
    }
}

pub const fn aesni_module_init(
    cpu: AesniCpuFeatures,
    x86_64: bool,
    crypto_api_available: bool,
) -> Result<AesniRegistration, i32> {
    if !aesni_base_cpu_supported(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }

    let avx = aesni_avx_supported(cpu, x86_64);
    let vaes_avx2 = aesni_vaes_avx2_supported(cpu, x86_64);
    let vaes_avx512 = aesni_vaes_avx512_supported(cpu, x86_64);
    Ok(AesniRegistration {
        base_skciphers: AESNI_SKCIPHERS.len(),
        base_gcm_aeads: if x86_64 { AES_GCM_ALGS_AESNI.len() } else { 0 },
        avx_skciphers: if avx { AESNI_AVX_SKCIPHERS.len() } else { 0 },
        avx_gcm_aeads: if avx { AES_GCM_ALGS_AESNI_AVX.len() } else { 0 },
        vaes_avx2_skciphers: if vaes_avx2 {
            VAES_AVX2_SKCIPHERS.len()
        } else {
            0
        },
        vaes_avx2_gcm_aeads: if vaes_avx2 {
            AES_GCM_ALGS_VAES_AVX2.len()
        } else {
            0
        },
        vaes_avx512_skciphers: if vaes_avx512 {
            VAES_AVX512_SKCIPHERS.len()
        } else {
            0
        },
        vaes_avx512_gcm_aeads: if vaes_avx512 {
            AES_GCM_ALGS_VAES_AVX512.len()
        } else {
            0
        },
        vaes_avx512_priority: aesni_effective_priority(
            AesniImplementation::VaesAvx512,
            cpu.prefer_ymm,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTMGR_C: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/crypto/testmgr.c"
    ));
    const TESTMGR_H: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/crypto/testmgr.h"
    ));

    #[test]
    fn aesni_registration_tables_match_linux_source_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/aesni-intel_glue.c"
        ));
        let aes_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/aes.h"
        ));
        let gcm_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/gcm.h"
        ));

        assert!(source.contains("#define AESNI_ALIGN\t16"));
        assert!(source.contains("#define AES_BLOCK_MASK\t(~(AES_BLOCK_SIZE - 1))"));
        assert!(source.contains("static struct skcipher_alg aesni_skciphers[]"));
        assert!(source.contains(".cra_driver_name\t= \"ecb-aes-aesni\""));
        assert!(source.contains(".cra_driver_name\t= \"xts-aes-aesni\""));
        assert!(source.contains("DEFINE_AVX_SKCIPHER_ALGS(aesni_avx, \"aesni-avx\", 500);"));
        assert!(source.contains("DEFINE_AVX_SKCIPHER_ALGS(vaes_avx2, \"vaes-avx2\", 600);"));
        assert!(source.contains("DEFINE_AVX_SKCIPHER_ALGS(vaes_avx512, \"vaes-avx512\", 800);"));
        assert!(source.contains("DEFINE_GCM_ALGS(aesni, /* no flags */ 0,"));
        assert!(source.contains("generic_gcmaes_set_authsize"));
        assert!(source.contains("common_rfc4106_set_authsize"));
        assert!(source.contains("X86_MATCH_FEATURE(X86_FEATURE_AES, NULL)"));
        assert!(source.contains("X86_FEATURE_VAES"));
        assert!(source.contains("X86_FEATURE_PREFER_YMM"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"aes\");"));
        assert!(aes_header.contains("#define AES_MIN_KEY_SIZE\t16"));
        assert!(aes_header.contains("static inline int aes_check_keylen(size_t keylen)"));
        assert!(gcm_header.contains("#define GCM_AES_IV_SIZE 12"));
        assert!(gcm_header.contains("#define GCM_RFC4106_IV_SIZE 8"));

        assert_eq!(AESNI_SKCIPHERS.len(), 5);
        assert_eq!(AESNI_SKCIPHERS[0].cra_driver_name, "ecb-aes-aesni");
        assert_eq!(AESNI_SKCIPHERS[4].cra_priority, 401);
        assert_eq!(AESNI_AVX_SKCIPHERS[0].cra_driver_name, "xts-aes-aesni-avx");
        assert_eq!(VAES_AVX2_SKCIPHERS[1].cra_driver_name, "ctr-aes-vaes-avx2");
        assert_eq!(
            VAES_AVX512_SKCIPHERS[2].cra_driver_name,
            "xctr-aes-vaes-avx512"
        );
        assert_eq!(AES_GCM_ALGS_AESNI[0].cra_driver_name, "generic-gcm-aesni");
        assert_eq!(AES_GCM_ALGS_VAES_AVX512[1].cra_priority, 800);

        for needle in [
            ".alg = \"ecb(aes)\"",
            ".alg = \"cbc(aes)\"",
            ".alg = \"cts(cbc(aes))\"",
            ".alg = \"ctr(aes)\"",
            ".alg = \"xts(aes)\"",
            ".alg = \"gcm(aes)\"",
            ".alg = \"rfc4106(gcm(aes))\"",
        ] {
            assert!(TESTMGR_C.contains(needle), "missing testmgr entry {needle}");
        }
        for template in [
            "aes_tv_template",
            "aes_cbc_tv_template",
            "cts_mode_tv_template",
            "aes_ctr_tv_template",
            "aes_xts_tv_template",
            "aes_gcm_tv_template",
            "aes_gcm_rfc4106_tv_template",
        ] {
            assert!(
                TESTMGR_H.contains(template),
                "missing test vectors {template}"
            );
        }
    }

    #[test]
    fn aesni_key_auth_and_cpu_gates_track_linux_branches() {
        assert_eq!(aes_check_keylen(15), Err(-EINVAL));
        assert_eq!(aes_check_keylen(16), Ok(()));
        assert_eq!(aes_check_keylen(24), Ok(()));
        assert_eq!(aes_check_keylen(32), Ok(()));
        assert_eq!(aes_gcm_keylen_valid(20), Err(-EINVAL));
        assert_eq!(aes_rfc4106_keylen_valid(20), Ok(()));
        assert_eq!(aes_rfc4106_keylen_valid(36), Ok(()));
        assert_eq!(aes_rfc4106_keylen_valid(19), Err(-EINVAL));

        assert_eq!(generic_gcmaes_set_authsize(4), Ok(()));
        assert_eq!(generic_gcmaes_set_authsize(13), Ok(()));
        assert_eq!(generic_gcmaes_set_authsize(7), Err(-EINVAL));
        assert_eq!(common_rfc4106_set_authsize(8), Ok(()));
        assert_eq!(common_rfc4106_set_authsize(12), Ok(()));
        assert_eq!(common_rfc4106_set_authsize(13), Err(-EINVAL));

        assert_eq!(xts_verify_key(&[0u8; 31], false, false), Err(-EINVAL));
        assert_eq!(xts_verify_key(&[0u8; 40], true, false), Err(-EINVAL));
        assert_eq!(xts_verify_key(&[0u8; 32], true, false), Err(-EINVAL));
        let mut xts = [0u8; 32];
        xts[16] = 1;
        assert_eq!(xts_verify_key(&xts, true, false), Ok(()));

        let full = AesniCpuFeatures {
            aes: true,
            avx: true,
            avx2: true,
            vaes: true,
            vpclmulqdq: true,
            pclmulqdq: true,
            avx512bw: true,
            avx512vl: true,
            bmi2: true,
            xfeatures_sse: true,
            xfeatures_ymm: true,
            xfeatures_avx512: true,
            prefer_ymm: true,
        };
        let reg = aesni_module_init(full, true, true).expect("full registration");
        assert_eq!(reg.base_skciphers, 5);
        assert_eq!(reg.base_gcm_aeads, 2);
        assert_eq!(reg.avx_skciphers, 3);
        assert_eq!(reg.vaes_avx2_gcm_aeads, 2);
        assert_eq!(reg.vaes_avx512_skciphers, 3);
        assert_eq!(reg.vaes_avx512_priority, 1);

        let base_only = AesniCpuFeatures {
            aes: true,
            ..AesniCpuFeatures::default()
        };
        let reg = aesni_module_init(base_only, true, true).expect("base registration");
        assert_eq!(reg.base_skciphers, 5);
        assert_eq!(reg.avx_skciphers, 0);
        assert_eq!(aesni_module_init(base_only, true, false), Err(-EOPNOTSUPP));
        assert_eq!(
            aesni_module_init(AesniCpuFeatures::default(), true, true),
            Err(-ENODEV)
        );
    }
}
