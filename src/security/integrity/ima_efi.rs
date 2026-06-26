//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/ima/ima_efi.c
//! test-origin: linux:vendor/linux/security/integrity/ima/ima_efi.c
//! IMA secure-boot architecture policy rules.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImaArchPolicyConfig {
    pub ima_arch_policy: bool,
    pub kexec_sig: bool,
    pub module_sig: bool,
    pub integrity_machine_keyring: bool,
    pub ima_keyrings_permit_signed_by_builtin_or_secondary: bool,
}

impl ImaArchPolicyConfig {
    pub const fn linux_secure_boot_defaults() -> Self {
        Self {
            ima_arch_policy: true,
            kexec_sig: false,
            module_sig: false,
            integrity_machine_keyring: false,
            ima_keyrings_permit_signed_by_builtin_or_secondary: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImaArchPolicy {
    pub rules: Vec<&'static str>,
    pub module_sig_enforced: bool,
    pub kexec_sig_enforced: bool,
}

pub fn arch_get_ima_policy(
    secure_boot: bool,
    config: ImaArchPolicyConfig,
) -> Option<ImaArchPolicy> {
    if !config.ima_arch_policy || !secure_boot {
        return None;
    }

    let mut rules = Vec::new();
    if !config.kexec_sig {
        rules.push("appraise func=KEXEC_KERNEL_CHECK appraise_type=imasig");
    }
    rules.push("measure func=KEXEC_KERNEL_CHECK");
    if !config.module_sig {
        rules.push("appraise func=MODULE_CHECK appraise_type=imasig");
    }
    if config.integrity_machine_keyring && config.ima_keyrings_permit_signed_by_builtin_or_secondary
    {
        rules.push("appraise func=POLICY_CHECK appraise_type=imasig");
    }
    rules.push("measure func=MODULE_CHECK");

    Some(ImaArchPolicy {
        rules,
        module_sig_enforced: true,
        kexec_sig_enforced: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_policy_requires_configured_secure_boot() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/ima/ima_efi.c"
        ));
        assert!(source.contains("static const char * const sb_arch_rules[]"));
        assert!(source.contains("arch_get_secureboot()"));
        assert!(source.contains("set_module_sig_enforced();"));
        assert!(source.contains("set_kexec_sig_enforced();"));

        let config = ImaArchPolicyConfig::linux_secure_boot_defaults();
        assert_eq!(arch_get_ima_policy(false, config), None);
        assert_eq!(
            arch_get_ima_policy(
                true,
                ImaArchPolicyConfig {
                    ima_arch_policy: false,
                    ..config
                }
            ),
            None
        );
    }

    #[test]
    fn secure_boot_policy_rules_follow_linux_order_and_config_gates() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let policy = arch_get_ima_policy(
            true,
            ImaArchPolicyConfig {
                integrity_machine_keyring: true,
                ima_keyrings_permit_signed_by_builtin_or_secondary: true,
                ..ImaArchPolicyConfig::linux_secure_boot_defaults()
            },
        )
        .expect("secure boot policy");

        assert_eq!(
            policy.rules.as_slice(),
            &[
                "appraise func=KEXEC_KERNEL_CHECK appraise_type=imasig",
                "measure func=KEXEC_KERNEL_CHECK",
                "appraise func=MODULE_CHECK appraise_type=imasig",
                "appraise func=POLICY_CHECK appraise_type=imasig",
                "measure func=MODULE_CHECK",
            ]
        );
        assert!(policy.module_sig_enforced);
        assert!(policy.kexec_sig_enforced);

        let signed = arch_get_ima_policy(
            true,
            ImaArchPolicyConfig {
                kexec_sig: true,
                module_sig: true,
                ..ImaArchPolicyConfig::linux_secure_boot_defaults()
            },
        )
        .expect("signed policy");
        assert_eq!(
            signed.rules.as_slice(),
            &[
                "measure func=KEXEC_KERNEL_CHECK",
                "measure func=MODULE_CHECK"
            ]
        );
    }
}
