// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/sunrpc_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016395

//! SUNRPC cache generic-netlink UAPI definitions.

use core::ffi::c_int;

/// C `enum sunrpc_cache_type` object representation for the frozen targets.
///
/// The selected GNU11 commands do not enable `-fshort-enums`, so the complete
/// range of this enum uses the C `int` representation. Its enumerators below
/// remain `c_int` integer constant expressions: in C an enum tag's object
/// type and the type of its enumerator expressions are distinct surfaces.
pub type sunrpc_cache_type = c_int;

// C string-literal macros are arrays, include a trailing NUL, and decay to
// pointers in ordinary expression context. The frozen Kbuild commands use
// `-funsigned-char`, so their element representation is `u8`. Keep the array
// form: a consuming Rust translation applies `.as_ptr()` at its corresponding
// C pointer-decay use.
pub static SUNRPC_FAMILY_NAME: [u8; 7] = *b"sunrpc\0";
pub const SUNRPC_FAMILY_VERSION: c_int = 1;

pub const SUNRPC_CACHE_TYPE_IP_MAP: c_int = 1;
pub const SUNRPC_CACHE_TYPE_UNIX_GID: c_int = 2;

// anonymous C enum at source line 18
pub const SUNRPC_A_CACHE_NOTIFY_CACHE_TYPE: c_int = 1;
pub const __SUNRPC_A_CACHE_NOTIFY_MAX: c_int = 2;
pub const SUNRPC_A_CACHE_NOTIFY_MAX: c_int = __SUNRPC_A_CACHE_NOTIFY_MAX - 1;

// anonymous C enum at source line 25
pub const SUNRPC_A_IP_MAP_SEQNO: c_int = 1;
pub const SUNRPC_A_IP_MAP_CLASS: c_int = 2;
pub const SUNRPC_A_IP_MAP_ADDR: c_int = 3;
pub const SUNRPC_A_IP_MAP_DOMAIN: c_int = 4;
pub const SUNRPC_A_IP_MAP_NEGATIVE: c_int = 5;
pub const SUNRPC_A_IP_MAP_EXPIRY: c_int = 6;
pub const __SUNRPC_A_IP_MAP_MAX: c_int = 7;
pub const SUNRPC_A_IP_MAP_MAX: c_int = __SUNRPC_A_IP_MAP_MAX - 1;

// anonymous C enum at source line 37
pub const SUNRPC_A_IP_MAP_REQS_REQUESTS: c_int = 1;
pub const __SUNRPC_A_IP_MAP_REQS_MAX: c_int = 2;
pub const SUNRPC_A_IP_MAP_REQS_MAX: c_int = __SUNRPC_A_IP_MAP_REQS_MAX - 1;

// anonymous C enum at source line 44
pub const SUNRPC_A_UNIX_GID_SEQNO: c_int = 1;
pub const SUNRPC_A_UNIX_GID_UID: c_int = 2;
pub const SUNRPC_A_UNIX_GID_GIDS: c_int = 3;
pub const SUNRPC_A_UNIX_GID_NEGATIVE: c_int = 4;
pub const SUNRPC_A_UNIX_GID_EXPIRY: c_int = 5;
pub const __SUNRPC_A_UNIX_GID_MAX: c_int = 6;
pub const SUNRPC_A_UNIX_GID_MAX: c_int = __SUNRPC_A_UNIX_GID_MAX - 1;

// anonymous C enum at source line 55
pub const SUNRPC_A_UNIX_GID_REQS_REQUESTS: c_int = 1;
pub const __SUNRPC_A_UNIX_GID_REQS_MAX: c_int = 2;
pub const SUNRPC_A_UNIX_GID_REQS_MAX: c_int = __SUNRPC_A_UNIX_GID_REQS_MAX - 1;

// anonymous C enum at source line 62
pub const SUNRPC_A_CACHE_FLUSH_MASK: c_int = 1;
pub const __SUNRPC_A_CACHE_FLUSH_MAX: c_int = 2;
pub const SUNRPC_A_CACHE_FLUSH_MAX: c_int = __SUNRPC_A_CACHE_FLUSH_MAX - 1;

// anonymous C enum at source line 69
pub const SUNRPC_CMD_CACHE_NOTIFY: c_int = 1;
pub const SUNRPC_CMD_IP_MAP_GET_REQS: c_int = 2;
pub const SUNRPC_CMD_IP_MAP_SET_REQS: c_int = 3;
pub const SUNRPC_CMD_UNIX_GID_GET_REQS: c_int = 4;
pub const SUNRPC_CMD_UNIX_GID_SET_REQS: c_int = 5;
pub const SUNRPC_CMD_CACHE_FLUSH: c_int = 6;
pub const __SUNRPC_CMD_MAX: c_int = 7;
pub const SUNRPC_CMD_MAX: c_int = __SUNRPC_CMD_MAX - 1;

pub static SUNRPC_MCGRP_NONE: [u8; 5] = *b"none\0";
pub static SUNRPC_MCGRP_EXPORTD: [u8; 8] = *b"exportd\0";
