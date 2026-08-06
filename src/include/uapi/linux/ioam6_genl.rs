// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/ioam6_genl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016196

//! IPv6 IOAM generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

/// C `enum ioam6_event_type`: a named enum tag with the frozen signed-`int`
/// ABI. Its enumerators below remain C `int` constant expressions.
pub type ioam6_event_type = c_int;

/// C `enum ioam6_event_attr`: a named enum tag with the frozen signed-`int`
/// ABI. Its enumerators below remain C `int` constant expressions.
pub type ioam6_event_attr = c_int;

// C string-literal macros are represented as array values, so they remain
// usable as aggregate initializers rather than becoming exported objects.
pub const IOAM6_GENL_NAME: [c_char; 6] = [
    b'I' as c_char,
    b'O' as c_char,
    b'A' as c_char,
    b'M' as c_char,
    b'6' as c_char,
    0,
];
pub const IOAM6_GENL_VERSION: c_int = 0x1;

pub const IOAM6_ATTR_UNSPEC: c_int = 0;
pub const IOAM6_ATTR_NS_ID: c_int = 1;
pub const IOAM6_ATTR_NS_DATA: c_int = 2;
pub const IOAM6_ATTR_NS_DATA_WIDE: c_int = 3;
pub const IOAM6_MAX_SCHEMA_DATA_LEN: c_int = 255 * 4;
pub const IOAM6_ATTR_SC_ID: c_int = 4;
pub const IOAM6_ATTR_SC_DATA: c_int = 5;
pub const IOAM6_ATTR_SC_NONE: c_int = 6;
pub const IOAM6_ATTR_PAD: c_int = 7;
pub const __IOAM6_ATTR_MAX: c_int = 8;
pub const IOAM6_ATTR_MAX: c_int = __IOAM6_ATTR_MAX - 1;

pub const IOAM6_CMD_UNSPEC: c_int = 0;
pub const IOAM6_CMD_ADD_NAMESPACE: c_int = 1;
pub const IOAM6_CMD_DEL_NAMESPACE: c_int = 2;
pub const IOAM6_CMD_DUMP_NAMESPACES: c_int = 3;
pub const IOAM6_CMD_ADD_SCHEMA: c_int = 4;
pub const IOAM6_CMD_DEL_SCHEMA: c_int = 5;
pub const IOAM6_CMD_DUMP_SCHEMAS: c_int = 6;
pub const IOAM6_CMD_NS_SET_SCHEMA: c_int = 7;
pub const __IOAM6_CMD_MAX: c_int = 8;
pub const IOAM6_CMD_MAX: c_int = __IOAM6_CMD_MAX - 1;

pub const IOAM6_GENL_EV_GRP_NAME: [c_char; 13] = [
    b'i' as c_char,
    b'o' as c_char,
    b'a' as c_char,
    b'm' as c_char,
    b'6' as c_char,
    b'_' as c_char,
    b'e' as c_char,
    b'v' as c_char,
    b'e' as c_char,
    b'n' as c_char,
    b't' as c_char,
    b's' as c_char,
    0,
];

pub const IOAM6_EVENT_UNSPEC: c_int = 0;
pub const IOAM6_EVENT_TRACE: c_int = 1;

pub const IOAM6_EVENT_ATTR_UNSPEC: c_int = 0;
pub const IOAM6_EVENT_ATTR_TRACE_NAMESPACE: c_int = 1;
pub const IOAM6_EVENT_ATTR_TRACE_NODELEN: c_int = 2;
pub const IOAM6_EVENT_ATTR_TRACE_TYPE: c_int = 3;
pub const IOAM6_EVENT_ATTR_TRACE_DATA: c_int = 4;
pub const __IOAM6_EVENT_ATTR_MAX: c_int = 5;
pub const IOAM6_EVENT_ATTR_MAX: c_int = __IOAM6_EVENT_ATTR_MAX - 1;
