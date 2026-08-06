// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/xt_state.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S016294

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::{c_int, c_uint};

use super::nf_conntrack_common::{ip_conntrack_info, IP_CT_IS_REPLY, IP_CT_NUMBER};

/// Expands C `XT_STATE_BIT(ctinfo)`.
///
/// # Safety
///
/// `ctinfo.0 % IP_CT_IS_REPLY.0 + 1` must be a valid non-negative C `int`
/// shift count. The C macro has undefined behavior when this precondition is
/// violated; this function retains that caller contract.
pub const unsafe fn XT_STATE_BIT(ctinfo: ip_conntrack_info) -> c_int {
    1 << (ctinfo.0 % IP_CT_IS_REPLY.0 + 1)
}

pub const XT_STATE_INVALID: c_int = 1 << 0;

pub const XT_STATE_UNTRACKED: c_int = 1 << (IP_CT_NUMBER.0 + 1);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct xt_state_info {
    pub statemask: c_uint,
}
