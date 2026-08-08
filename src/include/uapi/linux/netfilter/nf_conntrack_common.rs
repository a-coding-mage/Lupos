// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_conntrack_common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016270

/* Connection state tracking for netfilter. */
pub type ip_conntrack_info = i32;

pub const IP_CT_ESTABLISHED: ip_conntrack_info = 0;
pub const IP_CT_RELATED: ip_conntrack_info = 1;
pub const IP_CT_NEW: ip_conntrack_info = 2;
pub const IP_CT_IS_REPLY: ip_conntrack_info = 3;
pub const IP_CT_ESTABLISHED_REPLY: ip_conntrack_info = IP_CT_ESTABLISHED + IP_CT_IS_REPLY;
pub const IP_CT_RELATED_REPLY: ip_conntrack_info = IP_CT_RELATED + IP_CT_IS_REPLY;
pub const IP_CT_NUMBER: ip_conntrack_info = 5;

/* Frozen Kbuild invocations define __KERNEL__. */
pub const IP_CT_UNTRACKED: ip_conntrack_info = 7;

pub const NF_CT_STATE_INVALID_BIT: i32 = 1i32 << 0;

#[macro_export]
macro_rules! NF_CT_STATE_BIT {
    ($ctinfo:expr) => {
        1i32 << (($ctinfo % IP_CT_IS_REPLY) + 1)
    };
}

pub const NF_CT_STATE_UNTRACKED_BIT: i32 = 1i32 << 6;

/* Bitset representing status of connection. */
pub type ip_conntrack_status = i32;

pub const IPS_EXPECTED_BIT: ip_conntrack_status = 0;
pub const IPS_EXPECTED: ip_conntrack_status = 1i32 << IPS_EXPECTED_BIT;
pub const IPS_SEEN_REPLY_BIT: ip_conntrack_status = 1;
pub const IPS_SEEN_REPLY: ip_conntrack_status = 1i32 << IPS_SEEN_REPLY_BIT;
pub const IPS_ASSURED_BIT: ip_conntrack_status = 2;
pub const IPS_ASSURED: ip_conntrack_status = 1i32 << IPS_ASSURED_BIT;
pub const IPS_CONFIRMED_BIT: ip_conntrack_status = 3;
pub const IPS_CONFIRMED: ip_conntrack_status = 1i32 << IPS_CONFIRMED_BIT;
pub const IPS_SRC_NAT_BIT: ip_conntrack_status = 4;
pub const IPS_SRC_NAT: ip_conntrack_status = 1i32 << IPS_SRC_NAT_BIT;
pub const IPS_DST_NAT_BIT: ip_conntrack_status = 5;
pub const IPS_DST_NAT: ip_conntrack_status = 1i32 << IPS_DST_NAT_BIT;
pub const IPS_NAT_MASK: ip_conntrack_status = IPS_DST_NAT | IPS_SRC_NAT;
pub const IPS_SEQ_ADJUST_BIT: ip_conntrack_status = 6;
pub const IPS_SEQ_ADJUST: ip_conntrack_status = 1i32 << IPS_SEQ_ADJUST_BIT;
pub const IPS_SRC_NAT_DONE_BIT: ip_conntrack_status = 7;
pub const IPS_SRC_NAT_DONE: ip_conntrack_status = 1i32 << IPS_SRC_NAT_DONE_BIT;
pub const IPS_DST_NAT_DONE_BIT: ip_conntrack_status = 8;
pub const IPS_DST_NAT_DONE: ip_conntrack_status = 1i32 << IPS_DST_NAT_DONE_BIT;
pub const IPS_NAT_DONE_MASK: ip_conntrack_status = IPS_DST_NAT_DONE | IPS_SRC_NAT_DONE;
pub const IPS_DYING_BIT: ip_conntrack_status = 9;
pub const IPS_DYING: ip_conntrack_status = 1i32 << IPS_DYING_BIT;
pub const IPS_FIXED_TIMEOUT_BIT: ip_conntrack_status = 10;
pub const IPS_FIXED_TIMEOUT: ip_conntrack_status = 1i32 << IPS_FIXED_TIMEOUT_BIT;
pub const IPS_TEMPLATE_BIT: ip_conntrack_status = 11;
pub const IPS_TEMPLATE: ip_conntrack_status = 1i32 << IPS_TEMPLATE_BIT;
pub const IPS_UNTRACKED_BIT: ip_conntrack_status = 12;
pub const IPS_UNTRACKED: ip_conntrack_status = 1i32 << IPS_UNTRACKED_BIT;
pub const IPS_NAT_CLASH_BIT: ip_conntrack_status = IPS_UNTRACKED_BIT;
pub const IPS_NAT_CLASH: ip_conntrack_status = IPS_UNTRACKED;
pub const IPS_HELPER_BIT: ip_conntrack_status = 13;
pub const IPS_HELPER: ip_conntrack_status = 1i32 << IPS_HELPER_BIT;
pub const IPS_OFFLOAD_BIT: ip_conntrack_status = 14;
pub const IPS_OFFLOAD: ip_conntrack_status = 1i32 << IPS_OFFLOAD_BIT;
pub const IPS_HW_OFFLOAD_BIT: ip_conntrack_status = 15;
pub const IPS_HW_OFFLOAD: ip_conntrack_status = 1i32 << IPS_HW_OFFLOAD_BIT;
pub const IPS_UNCHANGEABLE_MASK: ip_conntrack_status =
    IPS_NAT_DONE_MASK | IPS_NAT_MASK | IPS_EXPECTED | IPS_CONFIRMED | IPS_DYING |
    IPS_SEQ_ADJUST | IPS_TEMPLATE | IPS_UNTRACKED | IPS_OFFLOAD | IPS_HW_OFFLOAD;
pub const __IPS_MAX_BIT: ip_conntrack_status = 16;

/* Connection tracking event types. */
pub type ip_conntrack_events = i32;

pub const IPCT_NEW: ip_conntrack_events = 0;
pub const IPCT_RELATED: ip_conntrack_events = 1;
pub const IPCT_DESTROY: ip_conntrack_events = 2;
pub const IPCT_REPLY: ip_conntrack_events = 3;
pub const IPCT_ASSURED: ip_conntrack_events = 4;
pub const IPCT_PROTOINFO: ip_conntrack_events = 5;
pub const IPCT_HELPER: ip_conntrack_events = 6;
pub const IPCT_MARK: ip_conntrack_events = 7;
pub const IPCT_SEQADJ: ip_conntrack_events = 8;
pub const IPCT_NATSEQADJ: ip_conntrack_events = IPCT_SEQADJ;
pub const IPCT_SECMARK: ip_conntrack_events = 9;
pub const IPCT_LABEL: ip_conntrack_events = 10;
pub const IPCT_SYNPROXY: ip_conntrack_events = 11;
pub const __IPCT_MAX: ip_conntrack_events = 12;

pub type ip_conntrack_expect_events = i32;

pub const IPEXP_NEW: ip_conntrack_expect_events = 0;
pub const IPEXP_DESTROY: ip_conntrack_expect_events = 1;

pub const NF_CT_EXPECT_PERMANENT: i32 = 0x1;
pub const NF_CT_EXPECT_INACTIVE: i32 = 0x2;
pub const NF_CT_EXPECT_USERSPACE: i32 = 0x4;
pub const NF_CT_EXPECT_DEAD: i32 = 0x8;
pub const NF_CT_EXPECT_MASK: i32 =
    NF_CT_EXPECT_PERMANENT | NF_CT_EXPECT_INACTIVE | NF_CT_EXPECT_USERSPACE;
