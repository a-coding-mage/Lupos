// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/sunrpc_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016395

// The C include guard prevents duplicate preprocessing; this module boundary
// provides the corresponding single-definition behavior.
pub const SUNRPC_FAMILY_NAME: &str = "sunrpc";
pub const SUNRPC_FAMILY_VERSION: i32 = 1;

#[repr(i32)]
pub enum sunrpc_cache_type {
    SUNRPC_CACHE_TYPE_IP_MAP = 1,
    SUNRPC_CACHE_TYPE_UNIX_GID = 2,
}

pub const SUNRPC_A_CACHE_NOTIFY_CACHE_TYPE: i32 = 1;
pub const __SUNRPC_A_CACHE_NOTIFY_MAX: i32 = 2;
pub const SUNRPC_A_CACHE_NOTIFY_MAX: i32 = __SUNRPC_A_CACHE_NOTIFY_MAX - 1;

pub const SUNRPC_A_IP_MAP_SEQNO: i32 = 1;
pub const SUNRPC_A_IP_MAP_CLASS: i32 = 2;
pub const SUNRPC_A_IP_MAP_ADDR: i32 = 3;
pub const SUNRPC_A_IP_MAP_DOMAIN: i32 = 4;
pub const SUNRPC_A_IP_MAP_NEGATIVE: i32 = 5;
pub const SUNRPC_A_IP_MAP_EXPIRY: i32 = 6;
pub const __SUNRPC_A_IP_MAP_MAX: i32 = 7;
pub const SUNRPC_A_IP_MAP_MAX: i32 = __SUNRPC_A_IP_MAP_MAX - 1;

pub const SUNRPC_A_IP_MAP_REQS_REQUESTS: i32 = 1;
pub const __SUNRPC_A_IP_MAP_REQS_MAX: i32 = 2;
pub const SUNRPC_A_IP_MAP_REQS_MAX: i32 = __SUNRPC_A_IP_MAP_REQS_MAX - 1;

pub const SUNRPC_A_UNIX_GID_SEQNO: i32 = 1;
pub const SUNRPC_A_UNIX_GID_UID: i32 = 2;
pub const SUNRPC_A_UNIX_GID_GIDS: i32 = 3;
pub const SUNRPC_A_UNIX_GID_NEGATIVE: i32 = 4;
pub const SUNRPC_A_UNIX_GID_EXPIRY: i32 = 5;
pub const __SUNRPC_A_UNIX_GID_MAX: i32 = 6;
pub const SUNRPC_A_UNIX_GID_MAX: i32 = __SUNRPC_A_UNIX_GID_MAX - 1;

pub const SUNRPC_A_UNIX_GID_REQS_REQUESTS: i32 = 1;
pub const __SUNRPC_A_UNIX_GID_REQS_MAX: i32 = 2;
pub const SUNRPC_A_UNIX_GID_REQS_MAX: i32 = __SUNRPC_A_UNIX_GID_REQS_MAX - 1;

pub const SUNRPC_A_CACHE_FLUSH_MASK: i32 = 1;
pub const __SUNRPC_A_CACHE_FLUSH_MAX: i32 = 2;
pub const SUNRPC_A_CACHE_FLUSH_MAX: i32 = __SUNRPC_A_CACHE_FLUSH_MAX - 1;

pub const SUNRPC_CMD_CACHE_NOTIFY: i32 = 1;
pub const SUNRPC_CMD_IP_MAP_GET_REQS: i32 = 2;
pub const SUNRPC_CMD_IP_MAP_SET_REQS: i32 = 3;
pub const SUNRPC_CMD_UNIX_GID_GET_REQS: i32 = 4;
pub const SUNRPC_CMD_UNIX_GID_SET_REQS: i32 = 5;
pub const SUNRPC_CMD_CACHE_FLUSH: i32 = 6;
pub const __SUNRPC_CMD_MAX: i32 = 7;
pub const SUNRPC_CMD_MAX: i32 = __SUNRPC_CMD_MAX - 1;

pub const SUNRPC_MCGRP_NONE: &str = "none";
pub const SUNRPC_MCGRP_EXPORTD: &str = "exportd";
