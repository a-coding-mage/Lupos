// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: security/selinux/include/policycap.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S018288

use core::ffi::c_uchar;

/* Policy capabilities */
pub const POLICYDB_CAP_NETPEER: i32 = 0;
pub const POLICYDB_CAP_OPENPERM: i32 = 1;
pub const POLICYDB_CAP_EXTSOCKCLASS: i32 = 2;
pub const POLICYDB_CAP_ALWAYSNETWORK: i32 = 3;
pub const POLICYDB_CAP_CGROUPSECLABEL: i32 = 4;
pub const POLICYDB_CAP_NNP_NOSUID_TRANSITION: i32 = 5;
pub const POLICYDB_CAP_GENFS_SECLABEL_SYMLINKS: i32 = 6;
pub const POLICYDB_CAP_IOCTL_SKIP_CLOEXEC: i32 = 7;
pub const POLICYDB_CAP_USERSPACE_INITIAL_CONTEXT: i32 = 8;
pub const POLICYDB_CAP_NETLINK_XPERM: i32 = 9;
pub const POLICYDB_CAP_NETIF_WILDCARD: i32 = 10;
pub const POLICYDB_CAP_GENFS_SECLABEL_WILDCARD: i32 = 11;
pub const POLICYDB_CAP_FUNCTIONFS_SECLABEL: i32 = 12;
pub const POLICYDB_CAP_MEMFD_CLASS: i32 = 13;
pub const POLICYDB_CAP_BPF_TOKEN_PERMS: i32 = 14;
pub const __POLICYDB_CAP_MAX: i32 = 15;

pub const POLICYDB_CAP_MAX: i32 = __POLICYDB_CAP_MAX - 1;

unsafe extern "C" {
    pub static selinux_policycap_names: [*const c_uchar; __POLICYDB_CAP_MAX as usize];
}
