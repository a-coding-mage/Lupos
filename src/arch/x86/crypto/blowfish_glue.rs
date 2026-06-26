//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/crypto/blowfish_glue.c
//! test-origin: linux:vendor/linux/arch/x86/crypto/blowfish_glue.c
//! x86 Blowfish assembler glue metadata and CPU blacklist.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const BF_BLOCK_SIZE: usize = 8;
pub const BF_MIN_KEY_SIZE: usize = 4;
pub const BF_MAX_KEY_SIZE: usize = 56;

pub const BLOWFISH_CRA_NAME: &str = "blowfish";
pub const BLOWFISH_DRIVER_NAME: &str = "blowfish-asm";
pub const BLOWFISH_PRIORITY: i32 = 200;
pub const BLOWFISH_DESCRIPTION: &str = "Blowfish Cipher Algorithm, asm optimized";
pub const BLOWFISH_MODULE_ALIASES: [&str; 2] = ["blowfish", "blowfish-asm"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlowfishAlgKind {
    Cipher,
    Ecb,
    Cbc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlowfishAlg {
    pub kind: BlowfishAlgKind,
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: i32,
    pub cra_blocksize: usize,
    pub min_keysize: usize,
    pub max_keysize: usize,
    pub ivsize: usize,
    pub parallel_decrypt_blocks: usize,
    pub has_setkey: bool,
    pub has_encrypt: bool,
    pub has_decrypt: bool,
}

pub const BLOWFISH_CIPHER_ALG: BlowfishAlg = BlowfishAlg {
    kind: BlowfishAlgKind::Cipher,
    cra_name: BLOWFISH_CRA_NAME,
    cra_driver_name: BLOWFISH_DRIVER_NAME,
    cra_priority: BLOWFISH_PRIORITY,
    cra_blocksize: BF_BLOCK_SIZE,
    min_keysize: BF_MIN_KEY_SIZE,
    max_keysize: BF_MAX_KEY_SIZE,
    ivsize: 0,
    parallel_decrypt_blocks: 1,
    has_setkey: true,
    has_encrypt: true,
    has_decrypt: true,
};

pub const BLOWFISH_SKCIPHER_ALGS: [BlowfishAlg; 2] = [
    BlowfishAlg {
        kind: BlowfishAlgKind::Ecb,
        cra_name: "ecb(blowfish)",
        cra_driver_name: "ecb-blowfish-asm",
        cra_priority: 300,
        cra_blocksize: BF_BLOCK_SIZE,
        min_keysize: BF_MIN_KEY_SIZE,
        max_keysize: BF_MAX_KEY_SIZE,
        ivsize: 0,
        parallel_decrypt_blocks: 4,
        has_setkey: true,
        has_encrypt: true,
        has_decrypt: true,
    },
    BlowfishAlg {
        kind: BlowfishAlgKind::Cbc,
        cra_name: "cbc(blowfish)",
        cra_driver_name: "cbc-blowfish-asm",
        cra_priority: 300,
        cra_blocksize: BF_BLOCK_SIZE,
        min_keysize: BF_MIN_KEY_SIZE,
        max_keysize: BF_MAX_KEY_SIZE,
        ivsize: BF_BLOCK_SIZE,
        parallel_decrypt_blocks: 4,
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
pub struct BlowfishCpu {
    pub vendor: X86Vendor,
    pub family: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlowfishRegistration {
    pub cipher_registered: bool,
    pub skcipher_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlowfishBlockFn {
    EncBlk,
    DecBlk,
    EncBlk4Way,
    DecEcb4Way,
    DecCbc4Way,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlowfishSkcipherStep {
    EcbBlock {
        blocks: usize,
        function: BlowfishBlockFn,
    },
    CbcEncBlock {
        function: BlowfishBlockFn,
    },
    CbcDecBlock {
        blocks: usize,
        function: BlowfishBlockFn,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlowfishSkcipherWalk {
    pub mode: BlowfishAlgKind,
    pub encrypt: bool,
    pub block_size: usize,
    pub fpu_blocks: i32,
    pub steps: &'static [BlowfishSkcipherStep],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlowfishInitPlan {
    pub result: Result<BlowfishRegistration, i32>,
    pub unregister_cipher_alg: bool,
}

pub const BLOWFISH_ECB_ENCRYPT_STEPS: [BlowfishSkcipherStep; 2] = [
    BlowfishSkcipherStep::EcbBlock {
        blocks: 4,
        function: BlowfishBlockFn::EncBlk4Way,
    },
    BlowfishSkcipherStep::EcbBlock {
        blocks: 1,
        function: BlowfishBlockFn::EncBlk,
    },
];

pub const BLOWFISH_ECB_DECRYPT_STEPS: [BlowfishSkcipherStep; 2] = [
    BlowfishSkcipherStep::EcbBlock {
        blocks: 4,
        function: BlowfishBlockFn::DecEcb4Way,
    },
    BlowfishSkcipherStep::EcbBlock {
        blocks: 1,
        function: BlowfishBlockFn::DecBlk,
    },
];

pub const BLOWFISH_CBC_ENCRYPT_STEPS: [BlowfishSkcipherStep; 1] =
    [BlowfishSkcipherStep::CbcEncBlock {
        function: BlowfishBlockFn::EncBlk,
    }];

pub const BLOWFISH_CBC_DECRYPT_STEPS: [BlowfishSkcipherStep; 2] = [
    BlowfishSkcipherStep::CbcDecBlock {
        blocks: 4,
        function: BlowfishBlockFn::DecCbc4Way,
    },
    BlowfishSkcipherStep::CbcDecBlock {
        blocks: 1,
        function: BlowfishBlockFn::DecBlk,
    },
];

pub const BLOWFISH_ECB_ENCRYPT_WALK: BlowfishSkcipherWalk = BlowfishSkcipherWalk {
    mode: BlowfishAlgKind::Ecb,
    encrypt: true,
    block_size: BF_BLOCK_SIZE,
    fpu_blocks: -1,
    steps: &BLOWFISH_ECB_ENCRYPT_STEPS,
};

pub const BLOWFISH_ECB_DECRYPT_WALK: BlowfishSkcipherWalk = BlowfishSkcipherWalk {
    mode: BlowfishAlgKind::Ecb,
    encrypt: false,
    block_size: BF_BLOCK_SIZE,
    fpu_blocks: -1,
    steps: &BLOWFISH_ECB_DECRYPT_STEPS,
};

pub const BLOWFISH_CBC_ENCRYPT_WALK: BlowfishSkcipherWalk = BlowfishSkcipherWalk {
    mode: BlowfishAlgKind::Cbc,
    encrypt: true,
    block_size: BF_BLOCK_SIZE,
    fpu_blocks: -1,
    steps: &BLOWFISH_CBC_ENCRYPT_STEPS,
};

pub const BLOWFISH_CBC_DECRYPT_WALK: BlowfishSkcipherWalk = BlowfishSkcipherWalk {
    mode: BlowfishAlgKind::Cbc,
    encrypt: false,
    block_size: BF_BLOCK_SIZE,
    fpu_blocks: -1,
    steps: &BLOWFISH_CBC_DECRYPT_STEPS,
};

pub const fn blowfish_keylen_allowed(keylen: usize) -> bool {
    keylen >= BF_MIN_KEY_SIZE && keylen <= BF_MAX_KEY_SIZE
}

pub const fn blowfish_is_blacklisted_cpu(cpu: BlowfishCpu) -> bool {
    matches!(cpu.vendor, X86Vendor::Intel) && cpu.family == 0x0f
}

pub const fn blowfish_skcipher_walk(
    mode: BlowfishAlgKind,
    encrypt: bool,
) -> Option<BlowfishSkcipherWalk> {
    match (mode, encrypt) {
        (BlowfishAlgKind::Ecb, true) => Some(BLOWFISH_ECB_ENCRYPT_WALK),
        (BlowfishAlgKind::Ecb, false) => Some(BLOWFISH_ECB_DECRYPT_WALK),
        (BlowfishAlgKind::Cbc, true) => Some(BLOWFISH_CBC_ENCRYPT_WALK),
        (BlowfishAlgKind::Cbc, false) => Some(BLOWFISH_CBC_DECRYPT_WALK),
        (BlowfishAlgKind::Cipher, _) => None,
    }
}

pub const fn blowfish_init_plan(
    force: bool,
    cpu: BlowfishCpu,
    cipher_register_errno: i32,
    skcipher_register_errno: i32,
) -> BlowfishInitPlan {
    if !force && blowfish_is_blacklisted_cpu(cpu) {
        return BlowfishInitPlan {
            result: Err(-ENODEV),
            unregister_cipher_alg: false,
        };
    }
    if cipher_register_errno != 0 {
        return BlowfishInitPlan {
            result: Err(cipher_register_errno),
            unregister_cipher_alg: false,
        };
    }
    if skcipher_register_errno != 0 {
        return BlowfishInitPlan {
            result: Err(skcipher_register_errno),
            unregister_cipher_alg: true,
        };
    }

    BlowfishInitPlan {
        result: Ok(BlowfishRegistration {
            cipher_registered: true,
            skcipher_count: BLOWFISH_SKCIPHER_ALGS.len(),
        }),
        unregister_cipher_alg: false,
    }
}

pub const fn blowfish_init(
    force: bool,
    cpu: BlowfishCpu,
    crypto_api_available: bool,
) -> Result<BlowfishRegistration, i32> {
    let crypto_errno = if crypto_api_available { 0 } else { -EOPNOTSUPP };
    blowfish_init_plan(force, cpu, crypto_errno, crypto_errno).result
}

pub const fn blowfish_fini(registered: BlowfishRegistration) -> bool {
    registered.cipher_registered && registered.skcipher_count == BLOWFISH_SKCIPHER_ALGS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blowfish_registration_matches_linux_source_selftest_and_testmgr() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/crypto/blowfish_glue.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/blowfish.h"
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
        let ipsec_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/net/ipsec.c"
        ));

        assert!(source.contains("asmlinkage void blowfish_enc_blk"));
        assert!(source.contains("asmlinkage void blowfish_enc_blk_4way"));
        assert!(source.contains("__blowfish_dec_blk_4way(ctx, dst, src, true);"));
        assert!(source.contains("ECB_BLOCK(4, blowfish_enc_blk_4way);"));
        assert!(source.contains("ECB_BLOCK(1, blowfish_enc_blk);"));
        assert!(source.contains("ECB_BLOCK(4, blowfish_dec_ecb_4way);"));
        assert!(source.contains("CBC_ENC_BLOCK(blowfish_enc_blk);"));
        assert!(source.contains("CBC_DEC_BLOCK(4, blowfish_dec_cbc_4way);"));
        assert!(source.contains("CBC_DEC_BLOCK(1, blowfish_dec_blk);"));
        assert!(source.contains(".cra_name\t\t= \"blowfish\""));
        assert!(source.contains(".cra_driver_name\t= \"blowfish-asm\""));
        assert!(source.contains(".base.cra_driver_name\t= \"ecb-blowfish-asm\""));
        assert!(source.contains(".base.cra_driver_name\t= \"cbc-blowfish-asm\""));
        assert!(source.contains("boot_cpu_data.x86_vendor != X86_VENDOR_INTEL"));
        assert!(source.contains("boot_cpu_data.x86 == 0x0f"));
        assert!(source.contains("module_param(force, int, 0);"));
        assert!(source.contains("crypto_unregister_alg(&bf_cipher_alg);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"blowfish-asm\");"));
        assert!(header.contains("#define BF_BLOCK_SIZE 8"));
        assert!(header.contains("#define BF_MIN_KEY_SIZE 4"));
        assert!(header.contains("#define BF_MAX_KEY_SIZE 56"));
        assert!(helper.contains("#define ECB_BLOCK(blocks, func)"));
        assert!(helper.contains("#define CBC_ENC_BLOCK(func)"));
        assert!(helper.contains("#define CBC_DEC_BLOCK(blocks, func)"));
        assert!(testmgr_c.contains(".alg = \"ecb(blowfish)\""));
        assert!(testmgr_c.contains(".alg = \"cbc(blowfish)\""));
        assert!(testmgr_c.contains(".alg = \"ctr(blowfish)\""));
        assert!(testmgr_h.contains("Blowfish test vectors."));
        assert!(ipsec_selftest.contains("{\"cbc(blowfish)\", 448}"));
        assert!(ipsec_selftest.contains("\"cbc(blowfish)\", \"cbc(aes)\""));

        assert_eq!(BLOWFISH_CIPHER_ALG.cra_driver_name, "blowfish-asm");
        assert_eq!(BLOWFISH_CIPHER_ALG.cra_priority, 200);
        assert_eq!(BLOWFISH_SKCIPHER_ALGS[0].parallel_decrypt_blocks, 4);
        assert_eq!(BLOWFISH_SKCIPHER_ALGS[1].ivsize, 8);
        assert_eq!(
            blowfish_skcipher_walk(BlowfishAlgKind::Ecb, true).unwrap(),
            BLOWFISH_ECB_ENCRYPT_WALK
        );
        assert_eq!(
            blowfish_skcipher_walk(BlowfishAlgKind::Cbc, false).unwrap(),
            BLOWFISH_CBC_DECRYPT_WALK
        );
        assert_eq!(blowfish_skcipher_walk(BlowfishAlgKind::Cipher, true), None);
        assert_eq!(BLOWFISH_MODULE_ALIASES, ["blowfish", "blowfish-asm"]);
    }

    #[test]
    fn blowfish_key_bounds_and_blacklist_track_linux_glue() {
        assert!(!blowfish_keylen_allowed(3));
        assert!(blowfish_keylen_allowed(4));
        assert!(blowfish_keylen_allowed(56));
        assert!(!blowfish_keylen_allowed(57));

        let p4 = BlowfishCpu {
            vendor: X86Vendor::Intel,
            family: 0x0f,
        };
        let other = BlowfishCpu {
            vendor: X86Vendor::Other,
            family: 0x0f,
        };
        assert!(blowfish_is_blacklisted_cpu(p4));
        assert!(!blowfish_is_blacklisted_cpu(other));
        assert_eq!(blowfish_init(false, p4, true), Err(-ENODEV));
        assert_eq!(
            blowfish_init(true, p4, true),
            Ok(BlowfishRegistration {
                cipher_registered: true,
                skcipher_count: 2,
            })
        );
        assert_eq!(blowfish_init(true, p4, false), Err(-EOPNOTSUPP));
        assert_eq!(
            blowfish_init_plan(true, p4, -5, 0),
            BlowfishInitPlan {
                result: Err(-5),
                unregister_cipher_alg: false,
            }
        );
        assert_eq!(
            blowfish_init_plan(true, p4, 0, -7),
            BlowfishInitPlan {
                result: Err(-7),
                unregister_cipher_alg: true,
            }
        );
        assert!(blowfish_fini(BlowfishRegistration {
            cipher_registered: true,
            skcipher_count: 2,
        }));
    }
}
