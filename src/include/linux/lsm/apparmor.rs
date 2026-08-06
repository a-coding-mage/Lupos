// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/lsm/apparmor.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014258

/// Linux Security Module property storage for AppArmor.
///
/// `CONFIG_SECURITY_APPARMOR` is disabled in both frozen configurations, so
/// the C definition has no members for the approved architecture union.
#[repr(C)]
pub struct lsm_prop_apparmor {}
