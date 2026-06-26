//! linux-parity: complete
//! linux-source: vendor/linux/security/ipe/ipe.c
//! test-origin: linux:vendor/linux/security/ipe/ipe.c
//! Integrity Policy Enforcement LSM registration metadata.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::security::hooks::LSM_ID_IPE;

pub const IPE_NAME: &str = "ipe";
pub const IPE_HOOKS: &[&str] = &[
    "bprm_check_security",
    "bprm_creds_for_exec",
    "mmap_file",
    "file_mprotect",
    "kernel_read_file",
    "kernel_load_data",
    "initramfs_populated",
];
pub const IPE_DM_VERITY_HOOKS: &[&str] = &["bdev_free_security", "bdev_setintegrity"];
pub const IPE_FS_VERITY_BUILTIN_SIG_HOOKS: &[&str] = &["inode_setintegrity"];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IpeSuperblock {
    pub initramfs: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IpeBdev {
    pub dm_verity_signed: bool,
    pub root_hash: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IpeInode {
    pub fs_verity_signed: bool,
}

static IPE_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpeBlobSizes {
    pub lbs_superblock: usize,
    pub lbs_bdev: usize,
    pub lbs_inode: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpeLsmId {
    pub name: &'static str,
    pub id: u64,
}

pub const IPE_LSMID: IpeLsmId = IpeLsmId {
    name: IPE_NAME,
    id: LSM_ID_IPE,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpeLsmDefinition {
    pub id: IpeLsmId,
    pub blobs: IpeBlobSizes,
    pub init: &'static str,
    pub initcall_fs: &'static str,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IpeRuntime {
    pub registered_hooks: Vec<&'static str>,
    pub registered_lsm: Option<IpeLsmId>,
    pub active_policy: Option<String>,
    pub securityfs_initcall: Option<&'static str>,
}

pub const fn ipe_blob_sizes(dm_verity: bool, fs_verity_builtin_sig: bool) -> IpeBlobSizes {
    IpeBlobSizes {
        lbs_superblock: core::mem::size_of::<IpeSuperblock>(),
        lbs_bdev: if dm_verity {
            core::mem::size_of::<IpeBdev>()
        } else {
            0
        },
        lbs_inode: if fs_verity_builtin_sig {
            core::mem::size_of::<IpeInode>()
        } else {
            0
        },
    }
}

pub const fn ipe_lsm_definition(dm_verity: bool, fs_verity_builtin_sig: bool) -> IpeLsmDefinition {
    IpeLsmDefinition {
        id: IPE_LSMID,
        blobs: ipe_blob_sizes(dm_verity, fs_verity_builtin_sig),
        init: "ipe_init",
        initcall_fs: "ipe_init_securityfs",
    }
}

pub fn ipe_hooks(dm_verity: bool, fs_verity_builtin_sig: bool) -> Vec<&'static str> {
    let mut hooks = Vec::from(IPE_HOOKS);
    if dm_verity {
        hooks.extend_from_slice(IPE_DM_VERITY_HOOKS);
    }
    if fs_verity_builtin_sig {
        hooks.extend_from_slice(IPE_FS_VERITY_BUILTIN_SIG_HOOKS);
    }
    hooks
}

pub const fn ipe_sb(s_security: usize, blobs: IpeBlobSizes) -> usize {
    s_security + blobs.lbs_superblock
}

pub const fn ipe_bdev(bd_security: usize, blobs: IpeBlobSizes) -> Option<usize> {
    if blobs.lbs_bdev == 0 {
        None
    } else {
        Some(bd_security + blobs.lbs_bdev)
    }
}

pub const fn ipe_inode(i_security: usize, blobs: IpeBlobSizes) -> Option<usize> {
    if blobs.lbs_inode == 0 {
        None
    } else {
        Some(i_security + blobs.lbs_inode)
    }
}

pub fn ipe_init(boot_policy_present: bool, policy_result: Result<(), i32>) -> Result<(), i32> {
    IPE_ENABLED.store(true, Ordering::Release);
    if boot_policy_present {
        policy_result?;
    }
    Ok(())
}

pub fn ipe_init_runtime(
    runtime: &mut IpeRuntime,
    boot_policy: Option<&str>,
    policy_result: Result<(), i32>,
    dm_verity: bool,
    fs_verity_builtin_sig: bool,
) -> Result<(), i32> {
    runtime.registered_hooks = ipe_hooks(dm_verity, fs_verity_builtin_sig);
    runtime.registered_lsm = Some(IPE_LSMID);
    runtime.securityfs_initcall = Some("ipe_init_securityfs");
    IPE_ENABLED.store(true, Ordering::Release);

    if let Some(policy) = boot_policy {
        policy_result?;
        runtime.active_policy = Some(policy.to_string());
    }

    Ok(())
}

pub fn ipe_enabled() -> bool {
    IPE_ENABLED.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn reset_for_test() {
    IPE_ENABLED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipe_matches_linux_lsm_registration_contract() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/ipe/ipe.c"
        ));
        let lsm = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/lsm.h"
        ));
        assert!(source.contains("bool ipe_enabled;"));
        assert!(source.contains(".lbs_superblock = sizeof(struct ipe_superblock),"));
        assert!(source.contains(".name = \"ipe\""));
        assert!(source.contains(".id = LSM_ID_IPE"));
        assert!(source.contains("return sb->s_security + ipe_blobs.lbs_superblock;"));
        assert!(source.contains("return b->bd_security + ipe_blobs.lbs_bdev;"));
        assert!(source.contains("return inode->i_security + ipe_blobs.lbs_inode;"));
        for hook in IPE_HOOKS {
            assert!(source.contains(hook));
        }
        assert!(
            source.contains("security_add_hooks(ipe_hooks, ARRAY_SIZE(ipe_hooks), &ipe_lsmid);")
        );
        assert!(source.contains("ipe_enabled = true;"));
        assert!(source.contains("p = ipe_new_policy(ipe_boot_policy, strlen(ipe_boot_policy),"));
        assert!(source.contains("if (IS_ERR(p))"));
        assert!(source.contains("rcu_assign_pointer(ipe_active_policy, p);"));
        assert!(source.contains("DEFINE_LSM(ipe) = {"));
        assert!(source.contains(".init = ipe_init,"));
        assert!(source.contains(".blobs = &ipe_blobs,"));
        assert!(source.contains(".initcall_fs = ipe_init_securityfs,"));
        assert!(lsm.contains("#define LSM_ID_IPE\t\t113"));

        assert_eq!(IPE_LSMID.id, LSM_ID_IPE);
        assert_eq!(IPE_HOOKS.len(), 7);
        let all_blobs = ipe_blob_sizes(true, true);
        assert_eq!(
            all_blobs.lbs_superblock,
            core::mem::size_of::<IpeSuperblock>()
        );
        assert_eq!(all_blobs.lbs_bdev, core::mem::size_of::<IpeBdev>());
        assert_eq!(all_blobs.lbs_inode, core::mem::size_of::<IpeInode>());
        assert_eq!(ipe_sb(0x1000, all_blobs), 0x1000 + all_blobs.lbs_superblock);
        assert_eq!(
            ipe_bdev(0x2000, all_blobs),
            Some(0x2000 + all_blobs.lbs_bdev)
        );
        assert_eq!(
            ipe_inode(0x3000, all_blobs),
            Some(0x3000 + all_blobs.lbs_inode)
        );
        assert_eq!(ipe_bdev(0x2000, ipe_blob_sizes(false, true)), None);
        assert_eq!(ipe_init(false, Ok(())), Ok(()));
        assert!(ipe_enabled());
        reset_for_test();
        assert_eq!(ipe_init(true, Err(-12)), Err(-12));
        assert!(ipe_enabled());

        reset_for_test();
        let definition = ipe_lsm_definition(true, true);
        assert_eq!(definition.id, IPE_LSMID);
        assert_eq!(definition.init, "ipe_init");
        assert_eq!(definition.initcall_fs, "ipe_init_securityfs");
        assert_eq!(definition.blobs, all_blobs);

        let mut runtime = IpeRuntime::default();
        assert_eq!(
            ipe_init_runtime(&mut runtime, Some("policy_name=boot"), Ok(()), true, true),
            Ok(())
        );
        assert_eq!(runtime.registered_hooks, ipe_hooks(true, true));
        assert_eq!(runtime.registered_lsm, Some(IPE_LSMID));
        assert_eq!(runtime.active_policy.as_deref(), Some("policy_name=boot"));
        assert_eq!(runtime.securityfs_initcall, Some("ipe_init_securityfs"));

        let mut failed_policy = IpeRuntime::default();
        assert_eq!(
            ipe_init_runtime(&mut failed_policy, Some("bad"), Err(-12), false, false),
            Err(-12)
        );
        assert_eq!(failed_policy.registered_hooks, ipe_hooks(false, false));
        assert_eq!(failed_policy.registered_lsm, Some(IPE_LSMID));
        assert_eq!(failed_policy.active_policy, None);
    }
}
