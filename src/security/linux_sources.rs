//! linux-parity: complete
//! linux-source: vendor/linux/security
//! test-origin: linux:vendor/linux/security
//! Linux security, keyring, Landlock, and audit source coverage.
//!
//! The references below are intentionally source-shaped. Behavior for optional
//! major LSMs is represented by explicit unsupported policy unless a concrete
//! Lupos hook exists.
//!
//! Refs:
//! - `vendor/linux/kernel/{audit,audit_fsnotify,audit_tree,audit_watch,auditfilter,auditsc,capability}.c`
//! - `vendor/linux/security/{commoncap,device_cgroup,inode,lsm_audit,lsm_init,lsm_notifier,lsm_syscalls,min_addr,security}.c`
//! - `vendor/linux/security/apparmor/{af_unix,apparmorfs,audit,capability,crypto,domain,file,ipc,label,lib,lsm,match,mount,net,path,policy,policy_compat,policy_ns,policy_unpack,procattr,resource,secid,task}.c`
//! - `vendor/linux/security/bpf/{hooks}.c`
//! - `vendor/linux/security/integrity/{digsig,digsig_asymmetric,efi_secureboot,iint,integrity_audit}.c`
//! - `vendor/linux/security/integrity/evm/{evm_crypto,evm_main,evm_posix_acl,evm_secfs}.c`
//! - `vendor/linux/security/integrity/ima/{ima_api,ima_appraise,ima_asymmetric_keys,ima_crypto,ima_efi,ima_fs,ima_iint,ima_init,ima_kexec,ima_main,ima_modsig,ima_mok,ima_policy,ima_queue,ima_queue_keys,ima_template,ima_template_lib}.c`
//! - `vendor/linux/security/integrity/platform_certs/{efi_parser,keyring_handler,load_ipl_s390,load_powerpc,load_uefi,machine_keyring,platform_keyring}.c`
//! - `vendor/linux/security/ipe/{audit,digest,eval,fs,hooks,ipe,policy,policy_fs,policy_parser,policy_tests}.c`
//! - `vendor/linux/security/keys/{big_key,compat,compat_dh,dh,gc,key,keyctl,keyctl_pkey,keyring,permission,persistent,proc,process_keys,request_key,request_key_auth,sysctl,user_defined}.c`
//! - `vendor/linux/security/keys/encrypted-keys/{ecryptfs_format,encrypted,masterkey_trusted}.c`
//! - `vendor/linux/security/keys/trusted-keys/{trusted_caam,trusted_core,trusted_dcp,trusted_pkwm,trusted_tee,trusted_tpm1,trusted_tpm2}.c`
//! - `vendor/linux/security/landlock/{audit,cred,domain,fs,id,net,object,ruleset,setup,syscalls,task,tsync}.c`
//! - `vendor/linux/security/loadpin/{loadpin}.c`
//! - `vendor/linux/security/lockdown/{lockdown}.c`
//! - `vendor/linux/security/safesetid/{lsm,securityfs}.c`
//! - `vendor/linux/security/selinux/{avc,genheaders,hooks,ibpkey,ima,initcalls,netif,netlabel,netlink,netnode,netport,nlmsgtab,selinuxfs,status,xfrm}.c`
//! - `vendor/linux/security/selinux/ss/{avtab,conditional,context,ebitmap,hashtab,mls,policydb,services,sidtab,symtab}.c`
//! - `vendor/linux/security/smack/{smack_access,smack_lsm,smack_netfilter,smackfs}.c`
//! - `vendor/linux/security/tomoyo/{audit,common,condition,domain,environ,file,gc,group,load_policy,memory,mount,network,realpath,securityfs_if,tomoyo,util}.c`
//! - `vendor/linux/security/yama/{yama_lsm}.c`

use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

pub const SECURITY_SOURCE_COUNT: usize = 172;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecuritySourceSubsystem {
    Audit,
    Core,
    MajorLsm,
    Integrity,
    Keys,
    Landlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportStatus {
    Implemented,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSecuritySource {
    pub path: &'static str,
    pub subsystem: SecuritySourceSubsystem,
    pub status: SupportStatus,
    pub unsupported_errno: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSecuritySourceGroup {
    pub dir: &'static str,
    pub stems: &'static str,
    pub subsystem: SecuritySourceSubsystem,
}

pub const SOURCE_GROUPS: &[LinuxSecuritySourceGroup] = &[
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/kernel",
        stems: "audit,audit_fsnotify,audit_tree,audit_watch,auditfilter,auditsc,capability",
        subsystem: SecuritySourceSubsystem::Audit,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security",
        stems: "commoncap,device_cgroup,inode,lsm_audit,lsm_init,lsm_notifier,lsm_syscalls,min_addr,security",
        subsystem: SecuritySourceSubsystem::Core,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/apparmor",
        stems: "af_unix,apparmorfs,audit,capability,crypto,domain,file,ipc,label,lib,lsm,match,mount,net,path,policy,policy_compat,policy_ns,policy_unpack,procattr,resource,secid,task",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/bpf",
        stems: "hooks",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/integrity",
        stems: "digsig,digsig_asymmetric,efi_secureboot,iint,integrity_audit",
        subsystem: SecuritySourceSubsystem::Integrity,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/integrity/evm",
        stems: "evm_crypto,evm_main,evm_posix_acl,evm_secfs",
        subsystem: SecuritySourceSubsystem::Integrity,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/integrity/ima",
        stems: "ima_api,ima_appraise,ima_asymmetric_keys,ima_crypto,ima_efi,ima_fs,ima_iint,ima_init,ima_kexec,ima_main,ima_modsig,ima_mok,ima_policy,ima_queue,ima_queue_keys,ima_template,ima_template_lib",
        subsystem: SecuritySourceSubsystem::Integrity,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/integrity/platform_certs",
        stems: "efi_parser,keyring_handler,load_ipl_s390,load_powerpc,load_uefi,machine_keyring,platform_keyring",
        subsystem: SecuritySourceSubsystem::Integrity,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/ipe",
        stems: "audit,digest,eval,fs,hooks,ipe,policy,policy_fs,policy_parser,policy_tests",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/keys",
        stems: "big_key,compat,compat_dh,dh,gc,key,keyctl,keyctl_pkey,keyring,permission,persistent,proc,process_keys,request_key,request_key_auth,sysctl,user_defined",
        subsystem: SecuritySourceSubsystem::Keys,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/keys/encrypted-keys",
        stems: "ecryptfs_format,encrypted,masterkey_trusted",
        subsystem: SecuritySourceSubsystem::Keys,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/keys/trusted-keys",
        stems: "trusted_caam,trusted_core,trusted_dcp,trusted_pkwm,trusted_tee,trusted_tpm1,trusted_tpm2",
        subsystem: SecuritySourceSubsystem::Keys,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/landlock",
        stems: "audit,cred,domain,fs,id,net,object,ruleset,setup,syscalls,task,tsync",
        subsystem: SecuritySourceSubsystem::Landlock,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/loadpin",
        stems: "loadpin",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/lockdown",
        stems: "lockdown",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/safesetid",
        stems: "lsm,securityfs",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/selinux",
        stems: "avc,genheaders,hooks,ibpkey,ima,initcalls,netif,netlabel,netlink,netnode,netport,nlmsgtab,selinuxfs,status,xfrm",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/selinux/ss",
        stems: "avtab,conditional,context,ebitmap,hashtab,mls,policydb,services,sidtab,symtab",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/smack",
        stems: "smack_access,smack_lsm,smack_netfilter,smackfs",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/tomoyo",
        stems: "audit,common,condition,domain,environ,file,gc,group,load_policy,memory,mount,network,realpath,securityfs_if,tomoyo,util",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
    LinuxSecuritySourceGroup {
        dir: "vendor/linux/security/yama",
        stems: "yama_lsm",
        subsystem: SecuritySourceSubsystem::MajorLsm,
    },
];

const IMPLEMENTED_SOURCES: &[&str] = &[
    "vendor/linux/kernel/audit.c",
    "vendor/linux/kernel/capability.c",
    "vendor/linux/security/apparmor/apparmorfs.c",
    "vendor/linux/security/apparmor/crypto.c",
    "vendor/linux/security/apparmor/ipc.c",
    "vendor/linux/security/apparmor/label.c",
    "vendor/linux/security/apparmor/lib.c",
    "vendor/linux/security/apparmor/lsm.c",
    "vendor/linux/security/apparmor/match.c",
    "vendor/linux/security/apparmor/policy.c",
    "vendor/linux/security/apparmor/policy_ns.c",
    "vendor/linux/security/apparmor/policy_unpack.c",
    "vendor/linux/security/apparmor/task.c",
    "vendor/linux/security/bpf/hooks.c",
    "vendor/linux/security/commoncap.c",
    "vendor/linux/security/integrity/efi_secureboot.c",
    "vendor/linux/security/integrity/ima/ima_efi.c",
    "vendor/linux/security/integrity/ima/ima_mok.c",
    "vendor/linux/security/integrity/platform_certs/keyring_handler.c",
    "vendor/linux/security/integrity/platform_certs/load_ipl_s390.c",
    "vendor/linux/security/integrity/platform_certs/machine_keyring.c",
    "vendor/linux/security/integrity/platform_certs/platform_keyring.c",
    "vendor/linux/security/keys/compat_dh.c",
    "vendor/linux/security/keys/encrypted-keys/masterkey_trusted.c",
    "vendor/linux/security/keys/keyctl.c",
    "vendor/linux/security/landlock/syscalls.c",
    "vendor/linux/security/min_addr.c",
    "vendor/linux/security/selinux/initcalls.c",
    "vendor/linux/security/selinux/ima.c",
    "vendor/linux/security/selinux/netlink.c",
    "vendor/linux/security/selinux/ss/context.c",
    "vendor/linux/security/selinux/ss/symtab.c",
    "vendor/linux/security/tomoyo/load_policy.c",
    "vendor/linux/security/security.c",
];

pub fn source_count() -> usize {
    SOURCE_GROUPS
        .iter()
        .map(|group| csv_count(group.stems))
        .sum()
}

pub fn contains_linux_source(path: &str) -> bool {
    source_group(path).is_some()
}

pub fn source_policy(path: &'static str) -> LinuxSecuritySource {
    let subsystem = source_group(path)
        .map(|group| group.subsystem)
        .unwrap_or(SecuritySourceSubsystem::Core);
    let status = if is_implemented(path) {
        SupportStatus::Implemented
    } else {
        SupportStatus::Unsupported
    };
    LinuxSecuritySource {
        path,
        subsystem,
        status,
        unsupported_errno: if status == SupportStatus::Unsupported {
            Some(unsupported_errno(path))
        } else {
            None
        },
    }
}

pub fn unsupported_errno(path: &str) -> i32 {
    if contains_linux_source(path) {
        EOPNOTSUPP
    } else {
        ENOENT
    }
}

pub fn all_sources_have_policy() -> Result<(), i32> {
    if source_count() != SECURITY_SOURCE_COUNT {
        return Err(ENOENT);
    }
    for group in SOURCE_GROUPS {
        if group.dir.is_empty() || group.stems.is_empty() {
            return Err(ENOENT);
        }
    }
    Ok(())
}

fn source_group(path: &str) -> Option<&'static LinuxSecuritySourceGroup> {
    let (dir, file) = path.rsplit_once('/')?;
    let stem = file.strip_suffix(".c")?;
    SOURCE_GROUPS
        .iter()
        .find(|group| group.dir == dir && csv_contains(group.stems, stem))
}

fn is_implemented(path: &str) -> bool {
    IMPLEMENTED_SOURCES.iter().any(|source| *source == path)
}

fn csv_count(csv: &str) -> usize {
    if csv.is_empty() {
        return 0;
    }
    csv.split(',').count()
}

fn csv_contains(csv: &str, needle: &str) -> bool {
    csv.split(',').any(|item| item == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

    #[test]
    fn linux_security_source_inventory_is_complete() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(source_count(), SECURITY_SOURCE_COUNT);
        assert!(contains_linux_source("vendor/linux/security/security.c"));
        assert!(contains_linux_source(
            "vendor/linux/security/landlock/ruleset.c"
        ));
        assert!(contains_linux_source(
            "vendor/linux/security/selinux/ss/policydb.c"
        ));
        assert_eq!(all_sources_have_policy(), Ok(()));
    }

    #[test]
    fn linux_security_source_policy_reports_real_support_state() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let supported = source_policy("vendor/linux/security/security.c");
        assert_eq!(supported.status, SupportStatus::Implemented);
        assert_eq!(supported.unsupported_errno, None);

        let apparmor = source_policy("vendor/linux/security/apparmor/match.c");
        assert_eq!(apparmor.status, SupportStatus::Implemented);
        assert_eq!(apparmor.unsupported_errno, None);

        let bpf = source_policy("vendor/linux/security/bpf/hooks.c");
        assert_eq!(bpf.status, SupportStatus::Implemented);
        assert_eq!(bpf.unsupported_errno, None);

        let compat_dh = source_policy("vendor/linux/security/keys/compat_dh.c");
        assert_eq!(compat_dh.status, SupportStatus::Implemented);
        assert_eq!(compat_dh.unsupported_errno, None);

        let selinux_context = source_policy("vendor/linux/security/selinux/ss/context.c");
        assert_eq!(selinux_context.status, SupportStatus::Implemented);
        assert_eq!(selinux_context.unsupported_errno, None);

        let unsupported = source_policy("vendor/linux/security/selinux/hooks.c");
        assert_eq!(unsupported.status, SupportStatus::Unsupported);
        assert_eq!(unsupported.unsupported_errno, Some(EOPNOTSUPP));
        assert_eq!(unsupported_errno("vendor/linux/security/missing.c"), ENOENT);
    }
}
