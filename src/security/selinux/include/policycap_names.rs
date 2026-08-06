// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: security/selinux/include/policycap_names.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S018289

use core::ffi::c_uchar;

use super::policycap::__POLICYDB_CAP_MAX;

/// A C `const char *` stored in the policy-capability name array.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct selinux_policycap_name(pub *const c_uchar);

// SAFETY: The stored pointers all designate immutable static NUL-terminated
// byte strings, and the C declaration makes each array element immutable.
unsafe impl Sync for selinux_policycap_name {}

/* Policy capability names */
#[unsafe(no_mangle)]
pub static selinux_policycap_names: [selinux_policycap_name; __POLICYDB_CAP_MAX as usize] = [
    selinux_policycap_name(b"network_peer_controls\0".as_ptr()),
    selinux_policycap_name(b"open_perms\0".as_ptr()),
    selinux_policycap_name(b"extended_socket_class\0".as_ptr()),
    selinux_policycap_name(b"always_check_network\0".as_ptr()),
    selinux_policycap_name(b"cgroup_seclabel\0".as_ptr()),
    selinux_policycap_name(b"nnp_nosuid_transition\0".as_ptr()),
    selinux_policycap_name(b"genfs_seclabel_symlinks\0".as_ptr()),
    selinux_policycap_name(b"ioctl_skip_cloexec\0".as_ptr()),
    selinux_policycap_name(b"userspace_initial_context\0".as_ptr()),
    selinux_policycap_name(b"netlink_xperm\0".as_ptr()),
    selinux_policycap_name(b"netif_wildcard\0".as_ptr()),
    selinux_policycap_name(b"genfs_seclabel_wildcard\0".as_ptr()),
    selinux_policycap_name(b"functionfs_seclabel\0".as_ptr()),
    selinux_policycap_name(b"memfd_class\0".as_ptr()),
    selinux_policycap_name(b"bpf_token_perms\0".as_ptr()),
];
