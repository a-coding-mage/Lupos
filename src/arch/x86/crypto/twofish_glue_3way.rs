//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/twofish_glue_3way.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/twofish_glue_3way.c
//! Twofish 3-way skcipher glue registration metadata and CPU blacklist.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const TF_MIN_KEY_SIZE: usize = 16;
pub const TF_MAX_KEY_SIZE: usize = 32;
pub const TF_BLOCK_SIZE: usize = 16;
pub const TWOFISH_THREEWAY_BLOCKS: usize = 3;

pub const TWOFISH_3WAY_DESCRIPTION: &str = "Twofish Cipher Algorithm, 3-way parallel asm optimized";
pub const TWOFISH_3WAY_MODULE_ALIASES: [&str; 2] = ["twofish", "twofish-asm"];
pub const TWOFISH_3WAY_EXPORTS: [&str; 3] = [
    "__twofish_enc_blk_3way",
    "twofish_dec_blk_3way",
    "twofish_dec_blk_cbc_3way",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Twofish3WayMode {
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Twofish3WaySkcipherAlg {
    pub mode: Twofish3WayMode,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub threeway_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const TWOFISH_3WAY_SKCIPHER_ALGS: [Twofish3WaySkcipherAlg; 2] = [
    Twofish3WaySkcipherAlg {
        mode: Twofish3WayMode::Ecb,
        cra_name: "ecb(twofish)",
        cra_driver_name: "ecb-twofish-3way",
        cra_priority: 300,
        cra_blocksize: TF_BLOCK_SIZE,
        min_keysize: TF_MIN_KEY_SIZE,
        max_keysize: TF_MAX_KEY_SIZE,
        ivsize: 0,
        threeway_blocks: TWOFISH_THREEWAY_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    Twofish3WaySkcipherAlg {
        mode: Twofish3WayMode::Cbc,
        cra_name: "cbc(twofish)",
        cra_driver_name: "cbc-twofish-3way",
        cra_priority: 300,
        cra_blocksize: TF_BLOCK_SIZE,
        min_keysize: TF_MIN_KEY_SIZE,
        max_keysize: TF_MAX_KEY_SIZE,
        ivsize: TF_BLOCK_SIZE,
        threeway_blocks: TWOFISH_THREEWAY_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Vendor {
    Intel,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntelVfm {
    AtomBonnell,
    AtomBonnellMid,
    AtomSaltwell,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Twofish3WayCpu {
    pub vendor: X86Vendor,
    pub vfm: IntelVfm,
    pub family: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Twofish3WayRegistration {
    pub alg_count: usize,
    pub export_count: usize,
}

pub const fn twofish_3way_is_blacklisted_cpu(cpu: Twofish3WayCpu) -> bool {
    if !matches!(cpu.vendor, X86Vendor::Intel) {
        return false;
    }

    match cpu.vfm {
        IntelVfm::AtomBonnell | IntelVfm::AtomBonnellMid | IntelVfm::AtomSaltwell => true,
        IntelVfm::Other => cpu.family == 0x0f,
    }
}

pub const fn twofish_3way_init(
    force: bool,
    cpu: Twofish3WayCpu,
    crypto_api_available: bool,
) -> Result<Twofish3WayRegistration, i32> {
    if !force && twofish_3way_is_blacklisted_cpu(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(Twofish3WayRegistration {
        alg_count: TWOFISH_3WAY_SKCIPHER_ALGS.len(),
        export_count: TWOFISH_3WAY_EXPORTS.len(),
    })
}

pub const fn twofish_3way_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twofish_3way_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/twofish_glue_3way.c"
        ));
        let local_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/twofish.h"
        ));
        let twofish_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/twofish.h"
        ));

        assert!(source.contains("EXPORT_SYMBOL_GPL(__twofish_enc_blk_3way);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(twofish_dec_blk_3way);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(twofish_dec_blk_cbc_3way);"));
        assert!(source.contains("return twofish_setkey(&tfm->base, key, keylen);"));
        assert!(source.contains("__twofish_enc_blk_3way(ctx, dst, src, false);"));
        assert!(source.contains("void twofish_dec_blk_cbc_3way"));
        assert!(source.contains("crypto_xor(dst + TF_BLOCK_SIZE, s, sizeof(buf));"));
        assert!(source.contains("ECB_BLOCK(3, twofish_enc_blk_3way);"));
        assert!(source.contains("ECB_BLOCK(3, twofish_dec_blk_3way);"));
        assert!(source.contains("CBC_ENC_BLOCK(twofish_enc_blk);"));
        assert!(source.contains("CBC_DEC_BLOCK(3, twofish_dec_blk_cbc_3way);"));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-twofish-3way\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-twofish-3way\""));
        assert!(source.contains(".base.cra_priority\t= 300"));
        assert!(source.contains("boot_cpu_data.x86_vendor != X86_VENDOR_INTEL"));
        assert!(source.contains("case INTEL_ATOM_BONNELL:"));
        assert!(source.contains("case INTEL_ATOM_BONNELL_MID:"));
        assert!(source.contains("case INTEL_ATOM_SALTWELL:"));
        assert!(source.contains("boot_cpu_data.x86 == 0x0f"));
        assert!(source.contains("module_param(force, int, 0);"));
        assert!(source.contains("return -ENODEV;"));
        assert!(source.contains("crypto_register_skciphers(tf_skciphers"));
        assert!(source.contains("crypto_unregister_skciphers(tf_skciphers"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"Twofish Cipher Algorithm, 3-way parallel asm optimized\");"
        ));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"twofish\");"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"twofish-asm\");"));
        assert!(local_header.contains("extern void twofish_dec_blk_cbc_3way"));
        assert!(twofish_header.contains("#define TF_BLOCK_SIZE 16"));

        assert_eq!(TWOFISH_3WAY_SKCIPHER_ALGS.len(), 2);
        assert_eq!(
            TWOFISH_3WAY_SKCIPHER_ALGS[0].cra_driver_name,
            "ecb-twofish-3way"
        );
        assert_eq!(TWOFISH_3WAY_SKCIPHER_ALGS[1].ivsize, TF_BLOCK_SIZE);
        assert_eq!(TWOFISH_3WAY_MODULE_ALIASES, ["twofish", "twofish-asm"]);
        assert_eq!(
            TWOFISH_3WAY_EXPORTS,
            [
                "__twofish_enc_blk_3way",
                "twofish_dec_blk_3way",
                "twofish_dec_blk_cbc_3way"
            ]
        );
    }

    #[test]
    fn twofish_3way_init_tracks_linux_blacklist_and_force() {
        let p4 = Twofish3WayCpu {
            vendor: X86Vendor::Intel,
            vfm: IntelVfm::Other,
            family: 0x0f,
        };
        let atom = Twofish3WayCpu {
            vendor: X86Vendor::Intel,
            vfm: IntelVfm::AtomBonnell,
            family: 6,
        };
        let other = Twofish3WayCpu {
            vendor: X86Vendor::Other,
            vfm: IntelVfm::AtomBonnell,
            family: 0x0f,
        };

        assert!(twofish_3way_is_blacklisted_cpu(p4));
        assert!(twofish_3way_is_blacklisted_cpu(atom));
        assert!(!twofish_3way_is_blacklisted_cpu(other));
        assert_eq!(twofish_3way_init(false, p4, true), Err(-ENODEV));
        assert_eq!(
            twofish_3way_init(true, p4, true),
            Ok(Twofish3WayRegistration {
                alg_count: 2,
                export_count: 3,
            })
        );
        assert_eq!(twofish_3way_init(true, p4, false), Err(-EOPNOTSUPP));
        assert!(twofish_3way_exit(true));
    }
}
