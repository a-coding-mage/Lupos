//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/crypto/camellia_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/camellia_glue.c
//! x86 Camellia assembler glue metadata, key-shape validation, and CPU blacklist.

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const CAMELLIA_MIN_KEY_SIZE: usize = 16;
pub const CAMELLIA_MAX_KEY_SIZE: usize = 32;
pub const CAMELLIA_BLOCK_SIZE: usize = 16;
pub const CAMELLIA_TABLE_BYTE_LEN: usize = 272;
pub const CAMELLIA_PARALLEL_BLOCKS: usize = 2;
pub const CAMELLIA_KEY_TABLE_U64S: usize = CAMELLIA_TABLE_BYTE_LEN / core::mem::size_of::<u64>();

pub const CAMELLIA_CRA_NAME: &str = "camellia";
pub const CAMELLIA_DRIVER_NAME: &str = "camellia-asm";
pub const CAMELLIA_PRIORITY: i32 = 200;
pub const CAMELLIA_DESCRIPTION: &str = "Camellia Cipher Algorithm, asm optimized";
pub const CAMELLIA_MODULE_ALIASES: [&str; 2] = ["camellia", "camellia-asm"];

pub const CAMELLIA_SIGMA1L: u32 = 0xA09E667F;
pub const CAMELLIA_SIGMA1R: u32 = 0x3BCC908B;
pub const CAMELLIA_SIGMA2L: u32 = 0xB67AE858;
pub const CAMELLIA_SIGMA2R: u32 = 0x4CAA73B2;
pub const CAMELLIA_SIGMA3L: u32 = 0xC6EF372F;
pub const CAMELLIA_SIGMA3R: u32 = 0xE94F82BE;
pub const CAMELLIA_SIGMA4L: u32 = 0x54FF53A5;
pub const CAMELLIA_SIGMA4R: u32 = 0xF1D36F1C;
pub const CAMELLIA_SIGMA5L: u32 = 0x10E527FA;
pub const CAMELLIA_SIGMA5R: u32 = 0xDE682D1D;
pub const CAMELLIA_SIGMA6L: u32 = 0xB05688C2;
pub const CAMELLIA_SIGMA6R: u32 = 0xB3E6C1FD;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CamelliaAlgKind {
    Cipher,
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CamelliaAlg {
    pub kind: CamelliaAlgKind,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub parallel_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const CAMELLIA_CIPHER_ALG: CamelliaAlg = CamelliaAlg {
    kind: CamelliaAlgKind::Cipher,
    cra_name: CAMELLIA_CRA_NAME,
    cra_driver_name: CAMELLIA_DRIVER_NAME,
    cra_priority: CAMELLIA_PRIORITY,
    cra_blocksize: CAMELLIA_BLOCK_SIZE,
    min_keysize: CAMELLIA_MIN_KEY_SIZE,
    max_keysize: CAMELLIA_MAX_KEY_SIZE,
    ivsize: 0,
    parallel_blocks: 1,
    has_setkey: true,
    has_encrypt: true,
    has_decrypt: true,
};

pub const CAMELLIA_SKCIPHER_ALGS: [CamelliaAlg; 2] = [
    CamelliaAlg {
        kind: CamelliaAlgKind::Ecb,
        cra_name: "ecb(camellia)",
        cra_driver_name: "ecb-camellia-asm",
        cra_priority: 300,
        cra_blocksize: CAMELLIA_BLOCK_SIZE,
        min_keysize: CAMELLIA_MIN_KEY_SIZE,
        max_keysize: CAMELLIA_MAX_KEY_SIZE,
        ivsize: 0,
        parallel_blocks: CAMELLIA_PARALLEL_BLOCKS,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    CamelliaAlg {
        kind: CamelliaAlgKind::Cbc,
        cra_name: "cbc(camellia)",
        cra_driver_name: "cbc-camellia-asm",
        cra_priority: 300,
        cra_blocksize: CAMELLIA_BLOCK_SIZE,
        min_keysize: CAMELLIA_MIN_KEY_SIZE,
        max_keysize: CAMELLIA_MAX_KEY_SIZE,
        ivsize: CAMELLIA_BLOCK_SIZE,
        parallel_blocks: CAMELLIA_PARALLEL_BLOCKS,
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
pub struct CamelliaCpu {
    pub vendor: X86Vendor,
    pub family: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CamelliaRegistration {
    pub cipher_registered: bool,
    pub skcipher_count: usize,
}

pub const fn camellia_keylen_valid(key_len: usize) -> Result<(), i32> {
    match key_len {
        16 | 24 | 32 => Ok(()),
        _ => Err(-EINVAL),
    }
}

pub const fn camellia_setup_tail_max(key_len: usize) -> Result<usize, i32> {
    match key_len {
        16 => Ok(24),
        24 | 32 => Ok(32),
        _ => Err(-EINVAL),
    }
}

pub const fn camellia_is_blacklisted_cpu(cpu: CamelliaCpu) -> bool {
    matches!(cpu.vendor, X86Vendor::Intel) && cpu.family == 0x0f
}

pub const fn camellia_init(
    force: bool,
    cpu: CamelliaCpu,
    crypto_api_available: bool,
) -> Result<CamelliaRegistration, i32> {
    if !force && camellia_is_blacklisted_cpu(cpu) {
        return Err(-ENODEV);
    }
    if !crypto_api_available {
        return Err(-EOPNOTSUPP);
    }
    Ok(CamelliaRegistration {
        cipher_registered: true,
        skcipher_count: CAMELLIA_SKCIPHER_ALGS.len(),
    })
}

pub const fn camellia_fini(registered: CamelliaRegistration) -> bool {
    registered.cipher_registered && registered.skcipher_count == CAMELLIA_SKCIPHER_ALGS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camellia_registration_matches_linux_source_selftest_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/camellia_glue.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/camellia.h"
        ));
        let testmgr_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.c"
        ));
        let testmgr_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/testmgr.h"
        ));
        let ipsec_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/net/ipsec.c"
        ));

        assert!(source.contains("asmlinkage void __camellia_enc_blk"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__camellia_setkey);"));
        assert!(source.contains("__visible const u64 camellia_sp10011110[256]"));
        assert!(source.contains("#define CAMELLIA_SIGMA1L (0xA09E667FL)"));
        assert!(source.contains("camellia_setup_tail(subkey, subRL, 24);"));
        assert!(source.contains("camellia_setup_tail(subkey, subRL, 32);"));
        assert!(source.contains(".cra_name\t\t= \"camellia\""));
        assert!(source.contains(".cra_driver_name\t= \"camellia-asm\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-camellia-asm\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-camellia-asm\""));
        assert!(source.contains("boot_cpu_data.x86_vendor != X86_VENDOR_INTEL"));
        assert!(source.contains("boot_cpu_data.x86 == 0x0f"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"camellia-asm\");"));
        assert!(header.contains("#define CAMELLIA_TABLE_BYTE_LEN\t272"));
        assert!(header.contains("#define CAMELLIA_PARALLEL_BLOCKS 2"));
        assert!(testmgr_c.contains(".alg = \"ecb(camellia)\""));
        assert!(testmgr_c.contains(".alg = \"cbc(camellia)\""));
        assert!(testmgr_c.contains(".alg = \"ctr(camellia)\""));
        assert!(testmgr_h.contains("static const struct cipher_testvec camellia_tv_template[]"));
        assert!(
            testmgr_h.contains("static const struct cipher_testvec camellia_cbc_tv_template[]")
        );
        assert!(ipsec_selftest.contains("{\"cbc(camellia)\", 256}"));
        assert!(ipsec_selftest.contains("\"cbc(serpent)\", \"cbc(camellia)\""));

        assert_eq!(CAMELLIA_CIPHER_ALG.cra_driver_name, "camellia-asm");
        assert_eq!(CAMELLIA_CIPHER_ALG.cra_priority, 200);
        assert_eq!(CAMELLIA_SKCIPHER_ALGS[0].parallel_blocks, 2);
        assert_eq!(CAMELLIA_SKCIPHER_ALGS[1].ivsize, 16);
        assert_eq!(CAMELLIA_KEY_TABLE_U64S, 34);
        assert_eq!(CAMELLIA_SIGMA6R, 0xB3E6C1FD);
        assert_eq!(CAMELLIA_MODULE_ALIASES, ["camellia", "camellia-asm"]);
    }

    #[test]
    fn camellia_key_shape_and_blacklist_track_linux_glue() {
        assert_eq!(camellia_keylen_valid(15), Err(-EINVAL));
        assert_eq!(camellia_keylen_valid(16), Ok(()));
        assert_eq!(camellia_keylen_valid(24), Ok(()));
        assert_eq!(camellia_keylen_valid(32), Ok(()));
        assert_eq!(camellia_keylen_valid(33), Err(-EINVAL));
        assert_eq!(camellia_setup_tail_max(16), Ok(24));
        assert_eq!(camellia_setup_tail_max(24), Ok(32));
        assert_eq!(camellia_setup_tail_max(32), Ok(32));
        assert_eq!(camellia_setup_tail_max(8), Err(-EINVAL));

        let p4 = CamelliaCpu {
            vendor: X86Vendor::Intel,
            family: 0x0f,
        };
        let newer_intel = CamelliaCpu {
            vendor: X86Vendor::Intel,
            family: 0x06,
        };
        assert!(camellia_is_blacklisted_cpu(p4));
        assert!(!camellia_is_blacklisted_cpu(newer_intel));
        assert_eq!(camellia_init(false, p4, true), Err(-ENODEV));
        assert_eq!(
            camellia_init(true, p4, true),
            Ok(CamelliaRegistration {
                cipher_registered: true,
                skcipher_count: 2,
            })
        );
        assert_eq!(camellia_init(true, p4, false), Err(-EOPNOTSUPP));
        assert!(camellia_fini(CamelliaRegistration {
            cipher_registered: true,
            skcipher_count: 2,
        }));
    }
}
