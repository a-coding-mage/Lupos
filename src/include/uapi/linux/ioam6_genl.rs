// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/ioam6_genl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016196

/*
 * IPv6 IOAM Generic Netlink API
 *
 * Author:
 * Justin Iurman <justin.iurman@uliege.be>
 */

pub const IOAM6_GENL_NAME: &str = "IOAM6";
pub const IOAM6_GENL_VERSION: i32 = 0x1;

pub const IOAM6_ATTR_UNSPEC: i32 = 0;
pub const IOAM6_ATTR_NS_ID: i32 = 1;
pub const IOAM6_ATTR_NS_DATA: i32 = 2;
pub const IOAM6_ATTR_NS_DATA_WIDE: i32 = 3;
pub const IOAM6_MAX_SCHEMA_DATA_LEN: i32 = 255_i32 * 4;
pub const IOAM6_ATTR_SC_ID: i32 = 4;
pub const IOAM6_ATTR_SC_DATA: i32 = 5;
pub const IOAM6_ATTR_SC_NONE: i32 = 6;
pub const IOAM6_ATTR_PAD: i32 = 7;
pub const __IOAM6_ATTR_MAX: i32 = 8;
pub const IOAM6_ATTR_MAX: i32 = __IOAM6_ATTR_MAX - 1;

pub const IOAM6_CMD_UNSPEC: i32 = 0;
pub const IOAM6_CMD_ADD_NAMESPACE: i32 = 1;
pub const IOAM6_CMD_DEL_NAMESPACE: i32 = 2;
pub const IOAM6_CMD_DUMP_NAMESPACES: i32 = 3;
pub const IOAM6_CMD_ADD_SCHEMA: i32 = 4;
pub const IOAM6_CMD_DEL_SCHEMA: i32 = 5;
pub const IOAM6_CMD_DUMP_SCHEMAS: i32 = 6;
pub const IOAM6_CMD_NS_SET_SCHEMA: i32 = 7;
pub const __IOAM6_CMD_MAX: i32 = 8;
pub const IOAM6_CMD_MAX: i32 = __IOAM6_CMD_MAX - 1;

pub const IOAM6_GENL_EV_GRP_NAME: &str = "ioam6_events";

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ioam6_event_type {
    IOAM6_EVENT_UNSPEC = 0,
    IOAM6_EVENT_TRACE = 1,
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ioam6_event_attr {
    IOAM6_EVENT_ATTR_UNSPEC = 0,
    IOAM6_EVENT_ATTR_TRACE_NAMESPACE = 1,
    IOAM6_EVENT_ATTR_TRACE_NODELEN = 2,
    IOAM6_EVENT_ATTR_TRACE_TYPE = 3,
    IOAM6_EVENT_ATTR_TRACE_DATA = 4,
    __IOAM6_EVENT_ATTR_MAX = 5,
}

pub const IOAM6_EVENT_ATTR_MAX: i32 = __IOAM6_EVENT_ATTR_MAX as i32 - 1;
