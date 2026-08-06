// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/mptcp_pm.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016252

//! MPTCP path-manager generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

/// C `enum mptcp_event_type`, preserving its distinct tag and `int` ABI.
///
/// The transparent representation accepts every `c_int` bit pattern, as C
/// does for values received from netlink.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct mptcp_event_type(pub c_int);

// The backing storage models the static-duration string literal.  The public
// macro-equivalent below has the pointer form produced by C array decay.
static MPTCP_PM_NAME_BYTES: [c_char; 9] = [
    b'm' as c_char,
    b'p' as c_char,
    b't' as c_char,
    b'c' as c_char,
    b'p' as c_char,
    b'_' as c_char,
    b'p' as c_char,
    b'm' as c_char,
    0,
];
pub const MPTCP_PM_NAME: *const c_char = MPTCP_PM_NAME_BYTES.as_ptr();
pub const MPTCP_PM_VER: c_int = 1;

pub const MPTCP_EVENT_UNSPEC: mptcp_event_type = mptcp_event_type(0);
pub const MPTCP_EVENT_CREATED: mptcp_event_type = mptcp_event_type(1);
pub const MPTCP_EVENT_ESTABLISHED: mptcp_event_type = mptcp_event_type(2);
pub const MPTCP_EVENT_CLOSED: mptcp_event_type = mptcp_event_type(3);
pub const MPTCP_EVENT_ANNOUNCED: mptcp_event_type = mptcp_event_type(6);
pub const MPTCP_EVENT_REMOVED: mptcp_event_type = mptcp_event_type(7);
pub const MPTCP_EVENT_SUB_ESTABLISHED: mptcp_event_type = mptcp_event_type(10);
pub const MPTCP_EVENT_SUB_CLOSED: mptcp_event_type = mptcp_event_type(11);
pub const MPTCP_EVENT_SUB_PRIORITY: mptcp_event_type = mptcp_event_type(13);
pub const MPTCP_EVENT_LISTENER_CREATED: mptcp_event_type = mptcp_event_type(15);
pub const MPTCP_EVENT_LISTENER_CLOSED: mptcp_event_type = mptcp_event_type(16);

pub const MPTCP_PM_ADDR_ATTR_UNSPEC: c_int = 0;
pub const MPTCP_PM_ADDR_ATTR_FAMILY: c_int = 1;
pub const MPTCP_PM_ADDR_ATTR_ID: c_int = 2;
pub const MPTCP_PM_ADDR_ATTR_ADDR4: c_int = 3;
pub const MPTCP_PM_ADDR_ATTR_ADDR6: c_int = 4;
pub const MPTCP_PM_ADDR_ATTR_PORT: c_int = 5;
pub const MPTCP_PM_ADDR_ATTR_FLAGS: c_int = 6;
pub const MPTCP_PM_ADDR_ATTR_IF_IDX: c_int = 7;
pub const __MPTCP_PM_ADDR_ATTR_MAX: c_int = 8;
pub const MPTCP_PM_ADDR_ATTR_MAX: c_int = __MPTCP_PM_ADDR_ATTR_MAX - 1;

pub const MPTCP_SUBFLOW_ATTR_UNSPEC: c_int = 0;
pub const MPTCP_SUBFLOW_ATTR_TOKEN_REM: c_int = 1;
pub const MPTCP_SUBFLOW_ATTR_TOKEN_LOC: c_int = 2;
pub const MPTCP_SUBFLOW_ATTR_RELWRITE_SEQ: c_int = 3;
pub const MPTCP_SUBFLOW_ATTR_MAP_SEQ: c_int = 4;
pub const MPTCP_SUBFLOW_ATTR_MAP_SFSEQ: c_int = 5;
pub const MPTCP_SUBFLOW_ATTR_SSN_OFFSET: c_int = 6;
pub const MPTCP_SUBFLOW_ATTR_MAP_DATALEN: c_int = 7;
pub const MPTCP_SUBFLOW_ATTR_FLAGS: c_int = 8;
pub const MPTCP_SUBFLOW_ATTR_ID_REM: c_int = 9;
pub const MPTCP_SUBFLOW_ATTR_ID_LOC: c_int = 10;
pub const MPTCP_SUBFLOW_ATTR_PAD: c_int = 11;
pub const __MPTCP_SUBFLOW_ATTR_MAX: c_int = 12;
pub const MPTCP_SUBFLOW_ATTR_MAX: c_int = __MPTCP_SUBFLOW_ATTR_MAX - 1;

pub const MPTCP_PM_ENDPOINT_ADDR: c_int = 1;
pub const __MPTCP_PM_ENDPOINT_MAX: c_int = 2;
pub const MPTCP_PM_ENDPOINT_MAX: c_int = __MPTCP_PM_ENDPOINT_MAX - 1;

pub const MPTCP_PM_ATTR_UNSPEC: c_int = 0;
pub const MPTCP_PM_ATTR_ADDR: c_int = 1;
pub const MPTCP_PM_ATTR_RCV_ADD_ADDRS: c_int = 2;
pub const MPTCP_PM_ATTR_SUBFLOWS: c_int = 3;
pub const MPTCP_PM_ATTR_TOKEN: c_int = 4;
pub const MPTCP_PM_ATTR_LOC_ID: c_int = 5;
pub const MPTCP_PM_ATTR_ADDR_REMOTE: c_int = 6;
pub const __MPTCP_ATTR_AFTER_LAST: c_int = 7;
pub const MPTCP_PM_ATTR_MAX: c_int = __MPTCP_ATTR_AFTER_LAST - 1;

/// C `enum mptcp_event_attr`, preserving its distinct tag and `int` ABI.
///
/// The transparent representation accepts every `c_int` bit pattern, as C
/// does for values received from netlink.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct mptcp_event_attr(pub c_int);

pub const MPTCP_ATTR_UNSPEC: mptcp_event_attr = mptcp_event_attr(0);
pub const MPTCP_ATTR_TOKEN: mptcp_event_attr = mptcp_event_attr(1);
pub const MPTCP_ATTR_FAMILY: mptcp_event_attr = mptcp_event_attr(2);
pub const MPTCP_ATTR_LOC_ID: mptcp_event_attr = mptcp_event_attr(3);
pub const MPTCP_ATTR_REM_ID: mptcp_event_attr = mptcp_event_attr(4);
pub const MPTCP_ATTR_SADDR4: mptcp_event_attr = mptcp_event_attr(5);
pub const MPTCP_ATTR_SADDR6: mptcp_event_attr = mptcp_event_attr(6);
pub const MPTCP_ATTR_DADDR4: mptcp_event_attr = mptcp_event_attr(7);
pub const MPTCP_ATTR_DADDR6: mptcp_event_attr = mptcp_event_attr(8);
pub const MPTCP_ATTR_SPORT: mptcp_event_attr = mptcp_event_attr(9);
pub const MPTCP_ATTR_DPORT: mptcp_event_attr = mptcp_event_attr(10);
pub const MPTCP_ATTR_BACKUP: mptcp_event_attr = mptcp_event_attr(11);
pub const MPTCP_ATTR_ERROR: mptcp_event_attr = mptcp_event_attr(12);
pub const MPTCP_ATTR_FLAGS: mptcp_event_attr = mptcp_event_attr(13);
pub const MPTCP_ATTR_TIMEOUT: mptcp_event_attr = mptcp_event_attr(14);
pub const MPTCP_ATTR_IF_IDX: mptcp_event_attr = mptcp_event_attr(15);
pub const MPTCP_ATTR_RESET_REASON: mptcp_event_attr = mptcp_event_attr(16);
pub const MPTCP_ATTR_RESET_FLAGS: mptcp_event_attr = mptcp_event_attr(17);
pub const MPTCP_ATTR_SERVER_SIDE: mptcp_event_attr = mptcp_event_attr(18);
pub const __MPTCP_ATTR_MAX: mptcp_event_attr = mptcp_event_attr(19);
pub const MPTCP_ATTR_MAX: mptcp_event_attr = mptcp_event_attr(__MPTCP_ATTR_MAX.0 - 1);

pub const MPTCP_PM_CMD_UNSPEC: c_int = 0;
pub const MPTCP_PM_CMD_ADD_ADDR: c_int = 1;
pub const MPTCP_PM_CMD_DEL_ADDR: c_int = 2;
pub const MPTCP_PM_CMD_GET_ADDR: c_int = 3;
pub const MPTCP_PM_CMD_FLUSH_ADDRS: c_int = 4;
pub const MPTCP_PM_CMD_SET_LIMITS: c_int = 5;
pub const MPTCP_PM_CMD_GET_LIMITS: c_int = 6;
pub const MPTCP_PM_CMD_SET_FLAGS: c_int = 7;
pub const MPTCP_PM_CMD_ANNOUNCE: c_int = 8;
pub const MPTCP_PM_CMD_REMOVE: c_int = 9;
pub const MPTCP_PM_CMD_SUBFLOW_CREATE: c_int = 10;
pub const MPTCP_PM_CMD_SUBFLOW_DESTROY: c_int = 11;
pub const __MPTCP_PM_CMD_AFTER_LAST: c_int = 12;
pub const MPTCP_PM_CMD_MAX: c_int = __MPTCP_PM_CMD_AFTER_LAST - 1;
