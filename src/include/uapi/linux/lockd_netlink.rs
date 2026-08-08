// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/lockd_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016228

/// Generic Netlink family name, including the terminating NUL required by
/// Linux's fixed-size `genl_family::name` character array.
pub const LOCKD_FAMILY_NAME: &[u8; 6] = b"lockd\0";

pub const LOCKD_FAMILY_VERSION: i32 = 1;

pub const LOCKD_A_SERVER_GRACETIME: i32 = 1;
pub const LOCKD_A_SERVER_TCP_PORT: i32 = 2;
pub const LOCKD_A_SERVER_UDP_PORT: i32 = 3;
pub const __LOCKD_A_SERVER_MAX: i32 = 4;
pub const LOCKD_A_SERVER_MAX: i32 = __LOCKD_A_SERVER_MAX - 1;

pub const LOCKD_CMD_SERVER_SET: i32 = 1;
pub const LOCKD_CMD_SERVER_GET: i32 = 2;
pub const __LOCKD_CMD_MAX: i32 = 3;
pub const LOCKD_CMD_MAX: i32 = __LOCKD_CMD_MAX - 1;
