// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hsr_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016150

/*
 * Copyright 2011-2013 Autronica Fire and Security AS
 *
 * Author(s):
 *     2011-2013 Arvid Brodin, arvid.brodin@xdin.com
 */

//! Generic Netlink HSR family UAPI constants.

use core::ffi::c_int;

// Attributes for an HSR or PRP node.  These are anonymous C-enum
// enumerators, and therefore are signed C `int` constant expressions rather
// than values of a named C enum type.
pub const HSR_A_UNSPEC: c_int = 0;
pub const HSR_A_NODE_ADDR: c_int = 1;
pub const HSR_A_IFINDEX: c_int = 2;
pub const HSR_A_IF1_AGE: c_int = 3;
pub const HSR_A_IF2_AGE: c_int = 4;
pub const HSR_A_NODE_ADDR_B: c_int = 5;
pub const HSR_A_IF1_SEQ: c_int = 6;
pub const HSR_A_IF2_SEQ: c_int = 7;
pub const HSR_A_IF1_IFINDEX: c_int = 8;
pub const HSR_A_IF2_IFINDEX: c_int = 9;
pub const HSR_A_ADDR_B_IFINDEX: c_int = 10;
pub const __HSR_A_MAX: c_int = 11;
pub const HSR_A_MAX: c_int = __HSR_A_MAX - 1;

// Generic Netlink HSR commands.  Like the attribute enumerators above,
// these are anonymous C-enum `int` constant expressions.
pub const HSR_C_UNSPEC: c_int = 0;
pub const HSR_C_RING_ERROR: c_int = 1;
pub const HSR_C_NODE_DOWN: c_int = 2;
pub const HSR_C_GET_NODE_STATUS: c_int = 3;
pub const HSR_C_SET_NODE_STATUS: c_int = 4;
pub const HSR_C_GET_NODE_LIST: c_int = 5;
pub const HSR_C_SET_NODE_LIST: c_int = 6;
pub const __HSR_C_MAX: c_int = 7;
pub const HSR_C_MAX: c_int = __HSR_C_MAX - 1;
