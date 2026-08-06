// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: security/selinux/include/initcalls.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S018281

use core::ffi::c_int;

/* SELinux initcalls */
unsafe extern "C" {
    pub fn init_sel_fs() -> c_int;
    pub fn sel_netport_init() -> c_int;
    pub fn sel_netnode_init() -> c_int;
    pub fn sel_netif_init() -> c_int;
    pub fn sel_netlink_init() -> c_int;
    pub fn sel_ib_pkey_init() -> c_int;
    pub fn selinux_nf_ip_init() -> c_int;

    pub fn selinux_initcall() -> c_int;
}
