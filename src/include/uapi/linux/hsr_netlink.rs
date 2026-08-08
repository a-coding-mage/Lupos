// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//!
//! linux-source: include/uapi/linux/hsr_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016150

// Generic Netlink HSR family definition.

// Attributes for HSR or PRP node.  These are C enum constants and therefore
// retain the signed C-int value domain used by the UAPI header.
pub const HSR_A_UNSPEC: i32 = 0;
pub const HSR_A_NODE_ADDR: i32 = 1;
pub const HSR_A_IFINDEX: i32 = 2;
pub const HSR_A_IF1_AGE: i32 = 3;
pub const HSR_A_IF2_AGE: i32 = 4;
pub const HSR_A_NODE_ADDR_B: i32 = 5;
pub const HSR_A_IF1_SEQ: i32 = 6;
pub const HSR_A_IF2_SEQ: i32 = 7;
pub const HSR_A_IF1_IFINDEX: i32 = 8;
pub const HSR_A_IF2_IFINDEX: i32 = 9;
pub const HSR_A_ADDR_B_IFINDEX: i32 = 10;
pub const __HSR_A_MAX: i32 = 11;
pub const HSR_A_MAX: i32 = __HSR_A_MAX - 1;

// Commands.
pub const HSR_C_UNSPEC: i32 = 0;
pub const HSR_C_RING_ERROR: i32 = 1;
pub const HSR_C_NODE_DOWN: i32 = 2;
pub const HSR_C_GET_NODE_STATUS: i32 = 3;
pub const HSR_C_SET_NODE_STATUS: i32 = 4;
pub const HSR_C_GET_NODE_LIST: i32 = 5;
pub const HSR_C_SET_NODE_LIST: i32 = 6;
pub const __HSR_C_MAX: i32 = 7;
pub const HSR_C_MAX: i32 = __HSR_C_MAX - 1;
