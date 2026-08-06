// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_conntrack_common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016270

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::c_int;

/* Connection state tracking for netfilter.  This is separated from,
 * but required by, the NAT layer; it can also be used by an iptables
 * extension. */

/* C enum tags have distinct types even though their selected ABI representation
 * is C `int`.  Transparent wrappers retain that distinction while permitting
 * the integer values outside the enumerator set permitted by C. */
macro_rules! nf_ct_uapi_enum {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub c_int);
    };
}

nf_ct_uapi_enum!(ip_conntrack_info);
nf_ct_uapi_enum!(ip_conntrack_status);
nf_ct_uapi_enum!(ip_conntrack_events);
nf_ct_uapi_enum!(ip_conntrack_expect_events);

pub const IP_CT_ESTABLISHED: ip_conntrack_info = ip_conntrack_info(0);
pub const IP_CT_RELATED: ip_conntrack_info = ip_conntrack_info(1);
pub const IP_CT_NEW: ip_conntrack_info = ip_conntrack_info(2);
pub const IP_CT_IS_REPLY: ip_conntrack_info = ip_conntrack_info(3);
pub const IP_CT_ESTABLISHED_REPLY: ip_conntrack_info =
    ip_conntrack_info(IP_CT_ESTABLISHED.0 + IP_CT_IS_REPLY.0);
pub const IP_CT_RELATED_REPLY: ip_conntrack_info =
    ip_conntrack_info(IP_CT_RELATED.0 + IP_CT_IS_REPLY.0);
pub const IP_CT_NUMBER: ip_conntrack_info = ip_conntrack_info(4);

/* Only for userspace compatibility. */
#[cfg(not(feature = "__KERNEL__"))]
pub const IP_CT_NEW_REPLY: ip_conntrack_info = IP_CT_NUMBER;

#[cfg(feature = "__KERNEL__")]
pub const IP_CT_UNTRACKED: ip_conntrack_info = ip_conntrack_info(7);

pub const NF_CT_STATE_INVALID_BIT: c_int = 1 << 0;

/// Expands C `NF_CT_STATE_BIT(ctinfo)`.
///
/// # Safety
///
/// `ctinfo.0 % IP_CT_IS_REPLY.0 + 1` must be a valid non-negative C `int`
/// shift count.  The C macro has undefined behavior when this precondition is
/// violated; this function retains that caller contract rather than adding a
/// safe, panic-capable invalid-input operation.
pub const unsafe fn NF_CT_STATE_BIT(ctinfo: ip_conntrack_info) -> c_int {
    1 << (ctinfo.0 % IP_CT_IS_REPLY.0 + 1)
}

pub const NF_CT_STATE_UNTRACKED_BIT: c_int = 1 << 6;

/* Bitset representing status of connection. */
pub const IPS_EXPECTED_BIT: ip_conntrack_status = ip_conntrack_status(0);
pub const IPS_EXPECTED: ip_conntrack_status = ip_conntrack_status(1 << IPS_EXPECTED_BIT.0);
pub const IPS_SEEN_REPLY_BIT: ip_conntrack_status = ip_conntrack_status(1);
pub const IPS_SEEN_REPLY: ip_conntrack_status = ip_conntrack_status(1 << IPS_SEEN_REPLY_BIT.0);
pub const IPS_ASSURED_BIT: ip_conntrack_status = ip_conntrack_status(2);
pub const IPS_ASSURED: ip_conntrack_status = ip_conntrack_status(1 << IPS_ASSURED_BIT.0);
pub const IPS_CONFIRMED_BIT: ip_conntrack_status = ip_conntrack_status(3);
pub const IPS_CONFIRMED: ip_conntrack_status = ip_conntrack_status(1 << IPS_CONFIRMED_BIT.0);
pub const IPS_SRC_NAT_BIT: ip_conntrack_status = ip_conntrack_status(4);
pub const IPS_SRC_NAT: ip_conntrack_status = ip_conntrack_status(1 << IPS_SRC_NAT_BIT.0);
pub const IPS_DST_NAT_BIT: ip_conntrack_status = ip_conntrack_status(5);
pub const IPS_DST_NAT: ip_conntrack_status = ip_conntrack_status(1 << IPS_DST_NAT_BIT.0);
pub const IPS_NAT_MASK: ip_conntrack_status = ip_conntrack_status(IPS_DST_NAT.0 | IPS_SRC_NAT.0);
pub const IPS_SEQ_ADJUST_BIT: ip_conntrack_status = ip_conntrack_status(6);
pub const IPS_SEQ_ADJUST: ip_conntrack_status = ip_conntrack_status(1 << IPS_SEQ_ADJUST_BIT.0);
pub const IPS_SRC_NAT_DONE_BIT: ip_conntrack_status = ip_conntrack_status(7);
pub const IPS_SRC_NAT_DONE: ip_conntrack_status = ip_conntrack_status(1 << IPS_SRC_NAT_DONE_BIT.0);
pub const IPS_DST_NAT_DONE_BIT: ip_conntrack_status = ip_conntrack_status(8);
pub const IPS_DST_NAT_DONE: ip_conntrack_status = ip_conntrack_status(1 << IPS_DST_NAT_DONE_BIT.0);
pub const IPS_NAT_DONE_MASK: ip_conntrack_status =
    ip_conntrack_status(IPS_DST_NAT_DONE.0 | IPS_SRC_NAT_DONE.0);
pub const IPS_DYING_BIT: ip_conntrack_status = ip_conntrack_status(9);
pub const IPS_DYING: ip_conntrack_status = ip_conntrack_status(1 << IPS_DYING_BIT.0);
pub const IPS_FIXED_TIMEOUT_BIT: ip_conntrack_status = ip_conntrack_status(10);
pub const IPS_FIXED_TIMEOUT: ip_conntrack_status = ip_conntrack_status(1 << IPS_FIXED_TIMEOUT_BIT.0);
pub const IPS_TEMPLATE_BIT: ip_conntrack_status = ip_conntrack_status(11);
pub const IPS_TEMPLATE: ip_conntrack_status = ip_conntrack_status(1 << IPS_TEMPLATE_BIT.0);
pub const IPS_UNTRACKED_BIT: ip_conntrack_status = ip_conntrack_status(12);
pub const IPS_UNTRACKED: ip_conntrack_status = ip_conntrack_status(1 << IPS_UNTRACKED_BIT.0);

/* Re-purposed for in-kernel use: tags a conntrack entry that clashed with
 * an existing entry on insert. */
#[cfg(feature = "__KERNEL__")]
pub const IPS_NAT_CLASH_BIT: ip_conntrack_status = IPS_UNTRACKED_BIT;
#[cfg(feature = "__KERNEL__")]
pub const IPS_NAT_CLASH: ip_conntrack_status = IPS_UNTRACKED;

pub const IPS_HELPER_BIT: ip_conntrack_status = ip_conntrack_status(13);
pub const IPS_HELPER: ip_conntrack_status = ip_conntrack_status(1 << IPS_HELPER_BIT.0);
pub const IPS_OFFLOAD_BIT: ip_conntrack_status = ip_conntrack_status(14);
pub const IPS_OFFLOAD: ip_conntrack_status = ip_conntrack_status(1 << IPS_OFFLOAD_BIT.0);
pub const IPS_HW_OFFLOAD_BIT: ip_conntrack_status = ip_conntrack_status(15);
pub const IPS_HW_OFFLOAD: ip_conntrack_status = ip_conntrack_status(1 << IPS_HW_OFFLOAD_BIT.0);
pub const IPS_UNCHANGEABLE_MASK: ip_conntrack_status = ip_conntrack_status(
    IPS_NAT_DONE_MASK.0
        | IPS_NAT_MASK.0
        | IPS_EXPECTED.0
        | IPS_CONFIRMED.0
        | IPS_DYING.0
        | IPS_SEQ_ADJUST.0
        | IPS_TEMPLATE.0
        | IPS_UNTRACKED.0
        | IPS_OFFLOAD.0
        | IPS_HW_OFFLOAD.0,
);

#[cfg(feature = "__KERNEL__")]
pub const __IPS_MAX_BIT: ip_conntrack_status = ip_conntrack_status(16);

/* Connection tracking event types. */
pub const IPCT_NEW: ip_conntrack_events = ip_conntrack_events(0);
pub const IPCT_RELATED: ip_conntrack_events = ip_conntrack_events(1);
pub const IPCT_DESTROY: ip_conntrack_events = ip_conntrack_events(2);
pub const IPCT_REPLY: ip_conntrack_events = ip_conntrack_events(3);
pub const IPCT_ASSURED: ip_conntrack_events = ip_conntrack_events(4);
pub const IPCT_PROTOINFO: ip_conntrack_events = ip_conntrack_events(5);
pub const IPCT_HELPER: ip_conntrack_events = ip_conntrack_events(6);
pub const IPCT_MARK: ip_conntrack_events = ip_conntrack_events(7);
pub const IPCT_SEQADJ: ip_conntrack_events = ip_conntrack_events(8);
pub const IPCT_NATSEQADJ: ip_conntrack_events = IPCT_SEQADJ;
pub const IPCT_SECMARK: ip_conntrack_events = ip_conntrack_events(9);
pub const IPCT_LABEL: ip_conntrack_events = ip_conntrack_events(10);
pub const IPCT_SYNPROXY: ip_conntrack_events = ip_conntrack_events(11);
#[cfg(feature = "__KERNEL__")]
pub const __IPCT_MAX: ip_conntrack_events = ip_conntrack_events(12);

pub const IPEXP_NEW: ip_conntrack_expect_events = ip_conntrack_expect_events(0);
pub const IPEXP_DESTROY: ip_conntrack_expect_events = ip_conntrack_expect_events(1);

/* Expectation flags. */
pub const NF_CT_EXPECT_PERMANENT: c_int = 0x1;
pub const NF_CT_EXPECT_INACTIVE: c_int = 0x2;
pub const NF_CT_EXPECT_USERSPACE: c_int = 0x4;
#[cfg(feature = "__KERNEL__")]
pub const NF_CT_EXPECT_DEAD: c_int = 0x8;
#[cfg(feature = "__KERNEL__")]
pub const NF_CT_EXPECT_MASK: c_int =
    NF_CT_EXPECT_PERMANENT | NF_CT_EXPECT_INACTIVE | NF_CT_EXPECT_USERSPACE;
