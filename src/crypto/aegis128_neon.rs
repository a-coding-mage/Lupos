//! linux-parity: complete
//! linux-source: vendor/linux/crypto/aegis128-neon.c
//! test-origin: linux:vendor/linux/crypto/aegis128-neon.c
//! AEGIS-128 ARM NEON SIMD glue.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Aegis128SimdAvailability {
    pub have_simd: bool,
    pub sets_aes_insn_flag: bool,
}

pub const SIMD_WRAPPERS: &[&str] = &[
    "crypto_aegis128_init_neon",
    "crypto_aegis128_update_neon",
    "crypto_aegis128_encrypt_chunk_neon",
    "crypto_aegis128_decrypt_chunk_neon",
    "crypto_aegis128_final_neon",
];

pub const fn crypto_aegis128_have_simd(
    cpu_has_aes_feature: bool,
    config_arm64: bool,
) -> Aegis128SimdAvailability {
    if cpu_has_aes_feature {
        Aegis128SimdAvailability {
            have_simd: true,
            sets_aes_insn_flag: true,
        }
    } else {
        Aegis128SimdAvailability {
            have_simd: config_arm64,
            sets_aes_insn_flag: false,
        }
    }
}

pub const fn simd_wrapper_uses_scoped_ksimd(wrapper: &str) -> bool {
    let mut i = 0;
    while i < SIMD_WRAPPERS.len() {
        if str_eq(SIMD_WRAPPERS[i], wrapper) {
            return true;
        }
        i += 1;
    }
    false
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut i = 0;
    while i < left.len() {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aegis128_neon_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/aegis128-neon.c"
        ));
        assert!(source.contains("#include <asm/cpufeature.h>"));
        assert!(source.contains("#include <asm/simd.h>"));
        assert!(source.contains("#include \"aegis.h\""));
        assert!(source.contains("#include \"aegis-neon.h\""));
        assert!(source.contains("int aegis128_have_aes_insn __ro_after_init;"));
        assert!(source.contains("cpu_have_feature(cpu_feature(AES))"));
        assert!(source.contains("aegis128_have_aes_insn = 1;"));
        assert!(source.contains("return IS_ENABLED(CONFIG_ARM64);"));
        for wrapper in SIMD_WRAPPERS {
            assert!(source.contains(wrapper));
        }
        assert_eq!(
            crypto_aegis128_have_simd(true, false),
            Aegis128SimdAvailability {
                have_simd: true,
                sets_aes_insn_flag: true,
            }
        );
        assert_eq!(
            crypto_aegis128_have_simd(false, true),
            Aegis128SimdAvailability {
                have_simd: true,
                sets_aes_insn_flag: false,
            }
        );
        assert!(!crypto_aegis128_have_simd(false, false).have_simd);
        assert!(simd_wrapper_uses_scoped_ksimd("crypto_aegis128_final_neon"));
    }
}
