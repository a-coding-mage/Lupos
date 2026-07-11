//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module/signing.c
//! test-origin: linux:vendor/linux/kernel/module/signing.c
//! Module signature marker handling and signature-enforcement decisions.
//!
//! The parsing/policy helper mirrors `signing.c`, but the production
//! `init_module`/`finit_module` path does not yet connect it to a keyring and
//! lockdown policy.  It must not be described as complete until signature
//! stripping and verification run before ELF parsing in the live loader.

extern crate alloc;

use crate::include::uapi::errno::{EBADMSG, EKEYREJECTED, ENODATA, ENOKEY, ENOPKG};
use crate::kernel::module_signature::{
    MODULE_SIGNATURE_ENCODED_SIZE, MODULE_SIGNATURE_MARKER, ModuleSignature, mod_check_sig,
};

pub const MODULE_INIT_IGNORE_MODVERSIONS: u32 = 1;
pub const MODULE_INIT_IGNORE_VERMAGIC: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSigPolicy {
    pub sig_enforced: bool,
    pub lockdown_result: Result<(), i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadInfo<'a> {
    pub module: &'a [u8],
    pub len: usize,
    pub sig_ok: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleSigFailureReason {
    UnsignedModule,
    UnsupportedCrypto,
    UnavailableKey,
}

impl<'a> LoadInfo<'a> {
    pub const fn new(module: &'a [u8]) -> Self {
        Self {
            module,
            len: module.len(),
            sig_ok: false,
        }
    }
}

pub fn is_module_sig_enforced(policy: ModuleSigPolicy) -> bool {
    policy.sig_enforced
}

pub fn set_module_sig_enforced(policy: &mut ModuleSigPolicy) {
    policy.sig_enforced = true;
}

pub fn mod_verify_sig<F>(info: &mut LoadInfo<'_>, mut verify_pkcs7: F) -> Result<(), i32>
where
    F: FnMut(&[u8], &[u8]) -> Result<(), i32>,
{
    if info.len > info.module.len() || info.len <= MODULE_SIGNATURE_ENCODED_SIZE {
        return Err(-EBADMSG);
    }

    let ms_offset = info.len - MODULE_SIGNATURE_ENCODED_SIZE;
    let ms = parse_module_signature(&info.module[ms_offset..info.len])?;
    mod_check_sig(ms, info.len)?;

    let sig_len = ms.sig_len();
    let unsigned_len = info
        .len
        .checked_sub(sig_len + MODULE_SIGNATURE_ENCODED_SIZE)
        .ok_or(-EBADMSG)?;
    let signature = &info.module[unsigned_len..unsigned_len + sig_len];
    info.len = unsigned_len;
    verify_pkcs7(&info.module[..unsigned_len], signature)
}

pub fn module_sig_check<F>(
    info: &mut LoadInfo<'_>,
    flags: u32,
    policy: ModuleSigPolicy,
    mut verify_pkcs7: F,
) -> Result<(), i32>
where
    F: FnMut(&[u8], &[u8]) -> Result<(), i32>,
{
    let marker = MODULE_SIGNATURE_MARKER.as_bytes();
    let marker_len = marker.len();
    let mangled_module =
        flags & (MODULE_INIT_IGNORE_MODVERSIONS | MODULE_INIT_IGNORE_VERMAGIC) != 0;
    let mut err = -ENODATA;

    if !mangled_module && info.len > marker_len && info.module[..info.len].ends_with(marker) {
        info.len -= marker_len;
        match mod_verify_sig(info, &mut verify_pkcs7) {
            Ok(()) => {
                info.sig_ok = true;
                return Ok(());
            }
            Err(e) => err = e,
        }
    }

    match module_sig_failure_reason(err) {
        Some(_reason) => {
            if is_module_sig_enforced(policy) {
                Err(-EKEYREJECTED)
            } else {
                policy.lockdown_result
            }
        }
        None => Err(err),
    }
}

pub const fn module_sig_failure_reason(err: i32) -> Option<ModuleSigFailureReason> {
    if err == -ENODATA {
        Some(ModuleSigFailureReason::UnsignedModule)
    } else if err == -ENOPKG {
        Some(ModuleSigFailureReason::UnsupportedCrypto)
    } else if err == -ENOKEY {
        Some(ModuleSigFailureReason::UnavailableKey)
    } else {
        None
    }
}

fn parse_module_signature(bytes: &[u8]) -> Result<ModuleSignature, i32> {
    if bytes.len() != MODULE_SIGNATURE_ENCODED_SIZE {
        return Err(-EBADMSG);
    }
    Ok(ModuleSignature {
        algo: bytes[0],
        hash: bytes[1],
        id_type: bytes[2],
        signer_len: bytes[3],
        key_id_len: bytes[4],
        pad: [bytes[5], bytes[6], bytes[7]],
        sig_len_be: u32::from_ne_bytes(bytes[8..12].try_into().map_err(|_| -EBADMSG)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn signed_module(id_type: u8, sig_len: u32) -> Vec<u8> {
        let mut module = b"module-body".to_vec();
        module.extend(core::iter::repeat_n(0xa5, sig_len as usize));
        module.extend([0, 0, id_type, 0, 0, 0, 0, 0]);
        module.extend(sig_len.to_be_bytes());
        module.extend(MODULE_SIGNATURE_MARKER.as_bytes());
        module
    }

    #[test]
    fn module_signature_check_matches_linux_marker_and_verify_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/signing.c"
        ));
        let uapi = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/module.h"
        ));
        let bpf_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/prog_tests/verify_pkcs7_sig.c"
        ));
        assert!(source.contains("static bool sig_enforce = IS_ENABLED(CONFIG_MODULE_SIG_FORCE);"));
        assert!(source.contains("module_param(sig_enforce, bool_enable_only, 0644);"));
        assert!(source.contains("memcpy(&ms, mod + (modlen - sizeof(ms)), sizeof(ms));"));
        assert!(source.contains("ret = mod_check_sig(&ms, modlen, \"module\");"));
        assert!(source.contains("info->len = modlen;"));
        assert!(source.contains("VERIFYING_MODULE_SIGNATURE"));
        assert!(source.contains("MODULE_SIGNATURE_MARKER"));
        assert!(source.contains("MODULE_INIT_IGNORE_MODVERSIONS |"));
        assert!(source.contains("case -ENODATA:"));
        assert!(source.contains("reason = \"unsigned module\";"));
        assert!(source.contains("reason = \"module with unsupported crypto\";"));
        assert!(source.contains("reason = \"module with unavailable key\";"));
        assert!(source.contains("return -EKEYREJECTED;"));
        assert!(source.contains("return security_locked_down(LOCKDOWN_MODULE_SIGNATURE);"));
        assert!(uapi.contains("#define MODULE_INIT_IGNORE_MODVERSIONS\t1"));
        assert!(uapi.contains("#define MODULE_INIT_IGNORE_VERMAGIC\t2"));
        assert!(bpf_selftest.contains("populate_data_item_mod"));
        assert!(bpf_selftest.contains("marker_len = sizeof(MODULE_SIGNATURE_MARKER) - 1;"));
        assert!(bpf_selftest.contains("sig_len = __be32_to_cpu(ms.sig_len);"));

        let bytes = signed_module(2, 4);
        let mut info = LoadInfo::new(&bytes);
        let policy = ModuleSigPolicy {
            sig_enforced: false,
            lockdown_result: Ok(()),
        };
        let result = module_sig_check(&mut info, 0, policy, |payload, signature| {
            assert_eq!(payload, b"module-body");
            assert_eq!(signature, &[0xa5; 4]);
            Ok(())
        });
        assert_eq!(result, Ok(()));
        assert!(info.sig_ok);
        assert_eq!(info.len, b"module-body".len());
    }

    #[test]
    fn module_signature_policy_matches_linux_error_classification() {
        let unsigned = b"module-body";
        let permissive = ModuleSigPolicy {
            sig_enforced: false,
            lockdown_result: Ok(()),
        };
        let enforced = ModuleSigPolicy {
            sig_enforced: true,
            lockdown_result: Ok(()),
        };

        let mut info = LoadInfo::new(unsigned);
        assert_eq!(
            module_sig_check(&mut info, 0, permissive, |_, _| Ok(())),
            Ok(())
        );
        let mut info = LoadInfo::new(unsigned);
        assert_eq!(
            module_sig_check(&mut info, 0, enforced, |_, _| Ok(())),
            Err(-EKEYREJECTED)
        );
        assert_eq!(
            module_sig_failure_reason(-ENODATA),
            Some(ModuleSigFailureReason::UnsignedModule)
        );
        assert_eq!(
            module_sig_failure_reason(-ENOPKG),
            Some(ModuleSigFailureReason::UnsupportedCrypto)
        );
        assert_eq!(
            module_sig_failure_reason(-ENOKEY),
            Some(ModuleSigFailureReason::UnavailableKey)
        );
        assert_eq!(module_sig_failure_reason(-EBADMSG), None);

        let unsupported = signed_module(1, 4);
        let mut info = LoadInfo::new(&unsupported);
        assert_eq!(
            module_sig_check(&mut info, 0, permissive, |_, _| Ok(())),
            Ok(())
        );
        let mut info = LoadInfo::new(&unsupported);
        assert_eq!(
            module_sig_check(&mut info, 0, enforced, |_, _| Ok(())),
            Err(-EKEYREJECTED)
        );

        let mut bad = signed_module(2, 4);
        let sig_len_offset =
            bad.len() - MODULE_SIGNATURE_MARKER.len() - MODULE_SIGNATURE_ENCODED_SIZE + 8;
        bad[sig_len_offset..sig_len_offset + 4].copy_from_slice(&4096u32.to_be_bytes());
        let mut info = LoadInfo::new(&bad);
        assert_eq!(
            module_sig_check(&mut info, 0, permissive, |_, _| Ok(())),
            Err(-EBADMSG)
        );

        let signed = signed_module(2, 4);
        let mut info = LoadInfo::new(&signed);
        assert_eq!(
            module_sig_check(&mut info, MODULE_INIT_IGNORE_VERMAGIC, enforced, |_, _| Ok(
                ()
            )),
            Err(-EKEYREJECTED)
        );
    }
}
