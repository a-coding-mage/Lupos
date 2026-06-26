//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto
//! test-origin: linux:vendor/linux/arch/x86/crypto
//! x86 accelerated crypto feature gates.
//!
//! Linux's files here register architecture-specific glue modules with the
//! crypto API. Lupos currently has no in-kernel crypto API registration path,
//! so accelerated algorithms are exposed as explicit feature decisions and
//! fail with `-EOPNOTSUPP` when requested through arch code.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/crypto/aegis128-aesni-glue.c
//! - vendor/linux/arch/x86/crypto/aesni-intel_glue.c
//! - vendor/linux/arch/x86/crypto/aria_aesni_avx2_glue.c
//! - vendor/linux/arch/x86/crypto/aria_aesni_avx_glue.c
//! - vendor/linux/arch/x86/crypto/aria_gfni_avx512_glue.c
//! - vendor/linux/arch/x86/crypto/blowfish_glue.c
//! - vendor/linux/arch/x86/crypto/camellia_aesni_avx2_glue.c
//! - vendor/linux/arch/x86/crypto/camellia_aesni_avx_glue.c
//! - vendor/linux/arch/x86/crypto/camellia_glue.c
//! - vendor/linux/arch/x86/crypto/cast5_avx_glue.c
//! - vendor/linux/arch/x86/crypto/cast6_avx_glue.c
//! - vendor/linux/arch/x86/crypto/serpent_avx2_glue.c
//! - vendor/linux/arch/x86/crypto/serpent_avx_glue.c
//! - vendor/linux/arch/x86/crypto/serpent_sse2_glue.c
//! - vendor/linux/arch/x86/crypto/sm4_aesni_avx2_glue.c
//! - vendor/linux/arch/x86/crypto/sm4_aesni_avx_glue.c
//! - vendor/linux/arch/x86/crypto/twofish_avx_glue.c
//! - vendor/linux/arch/x86/crypto/twofish_glue.c
//! - vendor/linux/arch/x86/crypto/twofish_glue_3way.c

use crate::include::uapi::errno::EOPNOTSUPP;

pub mod aegis128_aesni_glue;
pub mod aesni_intel_glue;
pub mod aria_aesni_avx2_glue;
pub mod aria_aesni_avx_glue;
pub mod blowfish_glue;
pub mod camellia_glue;
pub mod cast5_avx_glue;
pub mod cast6_avx_glue;
pub mod serpent_avx2_glue;
pub mod serpent_avx_glue;
pub mod serpent_sse2_glue;
pub mod twofish_avx_glue;
pub mod twofish_glue;
pub mod twofish_glue_3way;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86CryptoAlgorithm {
    Aesni,
    Aegis128,
    Aria,
    Blowfish,
    Camellia,
    Cast5,
    Cast6,
    Serpent,
    Sm4,
    Twofish,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct X86CryptoCpuFeatures {
    pub aesni: bool,
    pub sse2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512: bool,
    pub gfni: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86CryptoGlue {
    Aegis128Aesni,
    AesniIntel,
    AriaAesniAvx2,
    AriaAesniAvx,
    AriaGfniAvx512,
    Blowfish,
    CamelliaAesniAvx2,
    CamelliaAesniAvx,
    Camellia,
    Cast5Avx,
    Cast6Avx,
    SerpentAvx2,
    SerpentAvx,
    SerpentSse2,
    Sm4AesniAvx2,
    Sm4AesniAvx,
    TwofishAvx,
    Twofish,
    Twofish3Way,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86CryptoGlueInfo {
    pub algorithm: X86CryptoAlgorithm,
    pub needs_aesni: bool,
    pub needs_sse2: bool,
    pub needs_avx: bool,
    pub needs_avx2: bool,
    pub needs_avx512: bool,
    pub needs_gfni: bool,
    pub parallel_blocks: u8,
}

pub const fn accelerated_crypto_registered(_algorithm: X86CryptoAlgorithm) -> bool {
    false
}

pub const fn accelerated_crypto_errno(_algorithm: X86CryptoAlgorithm) -> i32 {
    EOPNOTSUPP
}

pub fn algorithm_requires_aesni(algorithm: X86CryptoAlgorithm) -> bool {
    matches!(
        algorithm,
        X86CryptoAlgorithm::Aesni
            | X86CryptoAlgorithm::Aegis128
            | X86CryptoAlgorithm::Aria
            | X86CryptoAlgorithm::Camellia
            | X86CryptoAlgorithm::Sm4
    )
}

pub const fn crypto_glue_info(glue: X86CryptoGlue) -> X86CryptoGlueInfo {
    match glue {
        X86CryptoGlue::Aegis128Aesni => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Aegis128,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 1,
        },
        X86CryptoGlue::AesniIntel => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Aesni,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 1,
        },
        X86CryptoGlue::AriaAesniAvx2 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Aria,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: true,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 32,
        },
        X86CryptoGlue::AriaAesniAvx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Aria,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 16,
        },
        X86CryptoGlue::AriaGfniAvx512 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Aria,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: true,
            needs_avx512: true,
            needs_gfni: true,
            parallel_blocks: 64,
        },
        X86CryptoGlue::Blowfish => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Blowfish,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 1,
        },
        X86CryptoGlue::CamelliaAesniAvx2 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Camellia,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: true,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 32,
        },
        X86CryptoGlue::CamelliaAesniAvx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Camellia,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 16,
        },
        X86CryptoGlue::Camellia => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Camellia,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 1,
        },
        X86CryptoGlue::Cast5Avx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Cast5,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 16,
        },
        X86CryptoGlue::Cast6Avx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Cast6,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 8,
        },
        X86CryptoGlue::SerpentAvx2 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Serpent,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: true,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 16,
        },
        X86CryptoGlue::SerpentAvx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Serpent,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 8,
        },
        X86CryptoGlue::SerpentSse2 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Serpent,
            needs_aesni: false,
            needs_sse2: true,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 8,
        },
        X86CryptoGlue::Sm4AesniAvx2 => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Sm4,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: true,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 16,
        },
        X86CryptoGlue::Sm4AesniAvx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Sm4,
            needs_aesni: true,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 8,
        },
        X86CryptoGlue::TwofishAvx => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Twofish,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: true,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 8,
        },
        X86CryptoGlue::Twofish => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Twofish,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 1,
        },
        X86CryptoGlue::Twofish3Way => X86CryptoGlueInfo {
            algorithm: X86CryptoAlgorithm::Twofish,
            needs_aesni: false,
            needs_sse2: false,
            needs_avx: false,
            needs_avx2: false,
            needs_avx512: false,
            needs_gfni: false,
            parallel_blocks: 3,
        },
    }
}

pub const fn crypto_glue_cpu_supported(glue: X86CryptoGlue, cpu: X86CryptoCpuFeatures) -> bool {
    let info = crypto_glue_info(glue);
    (!info.needs_aesni || cpu.aesni)
        && (!info.needs_sse2 || cpu.sse2)
        && (!info.needs_avx || cpu.avx)
        && (!info.needs_avx2 || cpu.avx2)
        && (!info.needs_avx512 || cpu.avx512)
        && (!info.needs_gfni || cpu.gfni)
}

pub const fn crypto_glue_registration_errno(
    glue: X86CryptoGlue,
    cpu: X86CryptoCpuFeatures,
) -> Option<i32> {
    if crypto_glue_cpu_supported(glue, cpu)
        && accelerated_crypto_registered(crypto_glue_info(glue).algorithm)
    {
        None
    } else {
        Some(EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerated_crypto_is_not_registered_without_crypto_api() {
        assert!(!accelerated_crypto_registered(X86CryptoAlgorithm::Aesni));
        assert_eq!(
            accelerated_crypto_errno(X86CryptoAlgorithm::Twofish),
            EOPNOTSUPP
        );
    }

    #[test]
    fn aesni_requirement_is_exposed() {
        assert!(algorithm_requires_aesni(X86CryptoAlgorithm::Sm4));
        assert!(!algorithm_requires_aesni(X86CryptoAlgorithm::Blowfish));
    }

    #[test]
    fn glue_info_maps_files_to_linux_algorithm_families() {
        assert_eq!(
            crypto_glue_info(X86CryptoGlue::Aegis128Aesni).algorithm,
            X86CryptoAlgorithm::Aegis128
        );
        assert_eq!(
            crypto_glue_info(X86CryptoGlue::AriaGfniAvx512).algorithm,
            X86CryptoAlgorithm::Aria
        );
        assert_eq!(
            crypto_glue_info(X86CryptoGlue::Twofish3Way).parallel_blocks,
            3
        );
    }

    #[test]
    fn glue_cpu_requirements_track_vector_feature_names() {
        let avx2_aes = X86CryptoCpuFeatures {
            aesni: true,
            sse2: true,
            avx: true,
            avx2: true,
            avx512: false,
            gfni: false,
        };
        assert!(crypto_glue_cpu_supported(
            X86CryptoGlue::CamelliaAesniAvx2,
            avx2_aes
        ));
        assert!(!crypto_glue_cpu_supported(
            X86CryptoGlue::AriaGfniAvx512,
            avx2_aes
        ));
        assert!(crypto_glue_cpu_supported(
            X86CryptoGlue::SerpentSse2,
            avx2_aes
        ));
        assert!(crypto_glue_cpu_supported(
            X86CryptoGlue::Blowfish,
            X86CryptoCpuFeatures::default()
        ));
    }

    #[test]
    fn crypto_glue_still_fails_closed_without_crypto_api_registration() {
        let full = X86CryptoCpuFeatures {
            aesni: true,
            sse2: true,
            avx: true,
            avx2: true,
            avx512: true,
            gfni: true,
        };
        assert_eq!(
            crypto_glue_registration_errno(X86CryptoGlue::AriaGfniAvx512, full),
            Some(EOPNOTSUPP)
        );
    }
}
