// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/mptcp_pm.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016252

pub const MPTCP_PM_NAME: &str = "mptcp_pm";
pub const MPTCP_PM_VER: i32 = 1;

pub const MPTCP_EVENT_UNSPEC: i32 = 0;
pub const MPTCP_EVENT_CREATED: i32 = 1;
pub const MPTCP_EVENT_ESTABLISHED: i32 = 2;
pub const MPTCP_EVENT_CLOSED: i32 = 3;
pub const MPTCP_EVENT_ANNOUNCED: i32 = 6;
pub const MPTCP_EVENT_REMOVED: i32 = 7;
pub const MPTCP_EVENT_SUB_ESTABLISHED: i32 = 10;
pub const MPTCP_EVENT_SUB_CLOSED: i32 = 11;
pub const MPTCP_EVENT_SUB_PRIORITY: i32 = 13;
pub const MPTCP_EVENT_LISTENER_CREATED: i32 = 15;
pub const MPTCP_EVENT_LISTENER_CLOSED: i32 = 16;

pub const MPTCP_PM_ADDR_ATTR_UNSPEC: i32 = 0;
pub const MPTCP_PM_ADDR_ATTR_FAMILY: i32 = 1;
pub const MPTCP_PM_ADDR_ATTR_ID: i32 = 2;
pub const MPTCP_PM_ADDR_ATTR_ADDR4: i32 = 3;
pub const MPTCP_PM_ADDR_ATTR_ADDR6: i32 = 4;
pub const MPTCP_PM_ADDR_ATTR_PORT: i32 = 5;
pub const MPTCP_PM_ADDR_ATTR_FLAGS: i32 = 6;
pub const MPTCP_PM_ADDR_ATTR_IF_IDX: i32 = 7;
pub const __MPTCP_PM_ADDR_ATTR_MAX: i32 = 8;
pub const MPTCP_PM_ADDR_ATTR_MAX: i32 = __MPTCP_PM_ADDR_ATTR_MAX - 1;

pub const MPTCP_SUBFLOW_ATTR_UNSPEC: i32 = 0;
pub const MPTCP_SUBFLOW_ATTR_TOKEN_REM: i32 = 1;
pub const MPTCP_SUBFLOW_ATTR_TOKEN_LOC: i32 = 2;
pub const MPTCP_SUBFLOW_ATTR_RELWRITE_SEQ: i32 = 3;
pub const MPTCP_SUBFLOW_ATTR_MAP_SEQ: i32 = 4;
pub const MPTCP_SUBFLOW_ATTR_MAP_SFSEQ: i32 = 5;
pub const MPTCP_SUBFLOW_ATTR_SSN_OFFSET: i32 = 6;
pub const MPTCP_SUBFLOW_ATTR_MAP_DATALEN: i32 = 7;
pub const MPTCP_SUBFLOW_ATTR_FLAGS: i32 = 8;
pub const MPTCP_SUBFLOW_ATTR_ID_REM: i32 = 9;
pub const MPTCP_SUBFLOW_ATTR_ID_LOC: i32 = 10;
pub const MPTCP_SUBFLOW_ATTR_PAD: i32 = 11;
pub const __MPTCP_SUBFLOW_ATTR_MAX: i32 = 12;
pub const MPTCP_SUBFLOW_ATTR_MAX: i32 = __MPTCP_SUBFLOW_ATTR_MAX - 1;

pub const MPTCP_PM_ENDPOINT_ADDR: i32 = 1;
pub const __MPTCP_PM_ENDPOINT_MAX: i32 = 2;
pub const MPTCP_PM_ENDPOINT_MAX: i32 = __MPTCP_PM_ENDPOINT_MAX - 1;

pub const MPTCP_PM_ATTR_UNSPEC: i32 = 0;
pub const MPTCP_PM_ATTR_ADDR: i32 = 1;
pub const MPTCP_PM_ATTR_RCV_ADD_ADDRS: i32 = 2;
pub const MPTCP_PM_ATTR_SUBFLOWS: i32 = 3;
pub const MPTCP_PM_ATTR_TOKEN: i32 = 4;
pub const MPTCP_PM_ATTR_LOC_ID: i32 = 5;
pub const MPTCP_PM_ATTR_ADDR_REMOTE: i32 = 6;
pub const __MPTCP_ATTR_AFTER_LAST: i32 = 7;
pub const MPTCP_PM_ATTR_MAX: i32 = __MPTCP_ATTR_AFTER_LAST - 1;

pub const MPTCP_ATTR_UNSPEC: i32 = 0;
pub const MPTCP_ATTR_TOKEN: i32 = 1;
pub const MPTCP_ATTR_FAMILY: i32 = 2;
pub const MPTCP_ATTR_LOC_ID: i32 = 3;
pub const MPTCP_ATTR_REM_ID: i32 = 4;
pub const MPTCP_ATTR_SADDR4: i32 = 5;
pub const MPTCP_ATTR_SADDR6: i32 = 6;
pub const MPTCP_ATTR_DADDR4: i32 = 7;
pub const MPTCP_ATTR_DADDR6: i32 = 8;
pub const MPTCP_ATTR_SPORT: i32 = 9;
pub const MPTCP_ATTR_DPORT: i32 = 10;
pub const MPTCP_ATTR_BACKUP: i32 = 11;
pub const MPTCP_ATTR_ERROR: i32 = 12;
pub const MPTCP_ATTR_FLAGS: i32 = 13;
pub const MPTCP_ATTR_TIMEOUT: i32 = 14;
pub const MPTCP_ATTR_IF_IDX: i32 = 15;
pub const MPTCP_ATTR_RESET_REASON: i32 = 16;
pub const MPTCP_ATTR_RESET_FLAGS: i32 = 17;
pub const MPTCP_ATTR_SERVER_SIDE: i32 = 18;
pub const __MPTCP_ATTR_MAX: i32 = 19;
pub const MPTCP_ATTR_MAX: i32 = __MPTCP_ATTR_MAX - 1;

pub const MPTCP_PM_CMD_UNSPEC: i32 = 0;
pub const MPTCP_PM_CMD_ADD_ADDR: i32 = 1;
pub const MPTCP_PM_CMD_DEL_ADDR: i32 = 2;
pub const MPTCP_PM_CMD_GET_ADDR: i32 = 3;
pub const MPTCP_PM_CMD_FLUSH_ADDRS: i32 = 4;
pub const MPTCP_PM_CMD_SET_LIMITS: i32 = 5;
pub const MPTCP_PM_CMD_GET_LIMITS: i32 = 6;
pub const MPTCP_PM_CMD_SET_FLAGS: i32 = 7;
pub const MPTCP_PM_CMD_ANNOUNCE: i32 = 8;
pub const MPTCP_PM_CMD_REMOVE: i32 = 9;
pub const MPTCP_PM_CMD_SUBFLOW_CREATE: i32 = 10;
pub const MPTCP_PM_CMD_SUBFLOW_DESTROY: i32 = 11;
pub const __MPTCP_PM_CMD_AFTER_LAST: i32 = 12;
pub const MPTCP_PM_CMD_MAX: i32 = __MPTCP_PM_CMD_AFTER_LAST - 1;
