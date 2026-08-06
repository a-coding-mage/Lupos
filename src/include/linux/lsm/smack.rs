// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/lsm/smack.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014261

/// Linux Security Module property storage for Smack.
///
/// `CONFIG_SECURITY_SMACK` is disabled in both frozen configurations, so the
/// C definition has no members for the approved architecture union.
#[repr(C)]
pub struct lsm_prop_smack {}
