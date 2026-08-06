// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/handshake.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016142

//! Handshake generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

// C keeps each enum tag as a named type, while its enumerator identifiers are
// `int` expressions.  Type aliases retain the tag spellings without imposing
// a Rust enum's invalid-value restriction or changing enumerator use sites.
pub type handshake_handler_class = c_int;
pub type handshake_msg_type = c_int;
pub type handshake_auth = c_int;

// C string-literal macros have static storage and decay to `const char *` in
// ordinary expression context. Keep NUL-terminated backing arrays so callers
// retain that same decay with `.as_ptr()`.
pub static HANDSHAKE_FAMILY_NAME: [c_char; 10] = [
    b'h' as c_char, b'a' as c_char, b'n' as c_char, b'd' as c_char,
    b's' as c_char, b'h' as c_char, b'a' as c_char, b'k' as c_char,
    b'e' as c_char, 0,
];
pub const HANDSHAKE_FAMILY_VERSION: c_int = 1;

pub const HANDSHAKE_HANDLER_CLASS_NONE: c_int = 0;
pub const HANDSHAKE_HANDLER_CLASS_TLSHD: c_int = 1;
pub const HANDSHAKE_HANDLER_CLASS_MAX: c_int = 2;

pub const HANDSHAKE_MSG_TYPE_UNSPEC: c_int = 0;
pub const HANDSHAKE_MSG_TYPE_CLIENTHELLO: c_int = 1;
pub const HANDSHAKE_MSG_TYPE_SERVERHELLO: c_int = 2;

pub const HANDSHAKE_AUTH_UNSPEC: c_int = 0;
pub const HANDSHAKE_AUTH_UNAUTH: c_int = 1;
pub const HANDSHAKE_AUTH_PSK: c_int = 2;
pub const HANDSHAKE_AUTH_X509: c_int = 3;

pub const HANDSHAKE_A_X509_CERT: c_int = 1;
pub const HANDSHAKE_A_X509_PRIVKEY: c_int = 2;
pub const __HANDSHAKE_A_X509_MAX: c_int = 3;
pub const HANDSHAKE_A_X509_MAX: c_int = __HANDSHAKE_A_X509_MAX - 1;

pub const HANDSHAKE_A_ACCEPT_SOCKFD: c_int = 1;
pub const HANDSHAKE_A_ACCEPT_HANDLER_CLASS: c_int = 2;
pub const HANDSHAKE_A_ACCEPT_MESSAGE_TYPE: c_int = 3;
pub const HANDSHAKE_A_ACCEPT_TIMEOUT: c_int = 4;
pub const HANDSHAKE_A_ACCEPT_AUTH_MODE: c_int = 5;
pub const HANDSHAKE_A_ACCEPT_PEER_IDENTITY: c_int = 6;
pub const HANDSHAKE_A_ACCEPT_CERTIFICATE: c_int = 7;
pub const HANDSHAKE_A_ACCEPT_PEERNAME: c_int = 8;
pub const HANDSHAKE_A_ACCEPT_KEYRING: c_int = 9;
pub const __HANDSHAKE_A_ACCEPT_MAX: c_int = 10;
pub const HANDSHAKE_A_ACCEPT_MAX: c_int = __HANDSHAKE_A_ACCEPT_MAX - 1;

pub const HANDSHAKE_A_DONE_STATUS: c_int = 1;
pub const HANDSHAKE_A_DONE_SOCKFD: c_int = 2;
pub const HANDSHAKE_A_DONE_REMOTE_AUTH: c_int = 3;
pub const __HANDSHAKE_A_DONE_MAX: c_int = 4;
pub const HANDSHAKE_A_DONE_MAX: c_int = __HANDSHAKE_A_DONE_MAX - 1;

pub const HANDSHAKE_CMD_READY: c_int = 1;
pub const HANDSHAKE_CMD_ACCEPT: c_int = 2;
pub const HANDSHAKE_CMD_DONE: c_int = 3;
pub const __HANDSHAKE_CMD_MAX: c_int = 4;
pub const HANDSHAKE_CMD_MAX: c_int = __HANDSHAKE_CMD_MAX - 1;

pub static HANDSHAKE_MCGRP_NONE: [c_char; 5] = [
    b'n' as c_char, b'o' as c_char, b'n' as c_char, b'e' as c_char, 0,
];
pub static HANDSHAKE_MCGRP_TLSHD: [c_char; 6] = [
    b't' as c_char, b'l' as c_char, b's' as c_char, b'h' as c_char,
    b'd' as c_char, 0,
];
