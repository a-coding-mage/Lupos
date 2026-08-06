// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/psp.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016344

//! PSP Security Protocol generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

// C `enum psp_version` has the frozen C `int` ABI.  Its enumerators are also
// integer constant expressions, so retain that representation for uses such
// as version bit positions.
pub type psp_version = c_int;

// C string-literal macros include a trailing NUL and decay to `const char *`
// in expression context.  Keep static backing storage and expose that
// macro-equivalent pointer form.
static PSP_FAMILY_NAME_BYTES: [c_char; 4] = [b'p' as c_char, b's' as c_char, b'p' as c_char, 0];
pub const PSP_FAMILY_NAME: *const c_char = PSP_FAMILY_NAME_BYTES.as_ptr();
pub const PSP_FAMILY_VERSION: c_int = 1;

pub const PSP_VERSION_HDR0_AES_GCM_128: psp_version = 0;
pub const PSP_VERSION_HDR0_AES_GCM_256: psp_version = 1;
pub const PSP_VERSION_HDR0_AES_GMAC_128: psp_version = 2;
pub const PSP_VERSION_HDR0_AES_GMAC_256: psp_version = 3;

// anonymous C enum
pub const PSP_A_ASSOC_DEV_INFO_IFINDEX: c_int = 1;
pub const PSP_A_ASSOC_DEV_INFO_NSID: c_int = 2;
pub const __PSP_A_ASSOC_DEV_INFO_MAX: c_int = 3;
pub const PSP_A_ASSOC_DEV_INFO_MAX: c_int = __PSP_A_ASSOC_DEV_INFO_MAX - 1;

// anonymous C enum
pub const PSP_A_DEV_ID: c_int = 1;
pub const PSP_A_DEV_IFINDEX: c_int = 2;
pub const PSP_A_DEV_PSP_VERSIONS_CAP: c_int = 3;
pub const PSP_A_DEV_PSP_VERSIONS_ENA: c_int = 4;
pub const PSP_A_DEV_ASSOC_LIST: c_int = 5;
pub const PSP_A_DEV_NSID: c_int = 6;
pub const PSP_A_DEV_BY_ASSOCIATION: c_int = 7;
pub const __PSP_A_DEV_MAX: c_int = 8;
pub const PSP_A_DEV_MAX: c_int = __PSP_A_DEV_MAX - 1;

// anonymous C enum
pub const PSP_A_ASSOC_DEV_ID: c_int = 1;
pub const PSP_A_ASSOC_VERSION: c_int = 2;
pub const PSP_A_ASSOC_RX_KEY: c_int = 3;
pub const PSP_A_ASSOC_TX_KEY: c_int = 4;
pub const PSP_A_ASSOC_SOCK_FD: c_int = 5;
pub const __PSP_A_ASSOC_MAX: c_int = 6;
pub const PSP_A_ASSOC_MAX: c_int = __PSP_A_ASSOC_MAX - 1;

// anonymous C enum
pub const PSP_A_KEYS_KEY: c_int = 1;
pub const PSP_A_KEYS_SPI: c_int = 2;
pub const __PSP_A_KEYS_MAX: c_int = 3;
pub const PSP_A_KEYS_MAX: c_int = __PSP_A_KEYS_MAX - 1;

// anonymous C enum
pub const PSP_A_STATS_DEV_ID: c_int = 1;
pub const PSP_A_STATS_KEY_ROTATIONS: c_int = 2;
pub const PSP_A_STATS_STALE_EVENTS: c_int = 3;
pub const PSP_A_STATS_RX_PACKETS: c_int = 4;
pub const PSP_A_STATS_RX_BYTES: c_int = 5;
pub const PSP_A_STATS_RX_AUTH_FAIL: c_int = 6;
pub const PSP_A_STATS_RX_ERROR: c_int = 7;
pub const PSP_A_STATS_RX_BAD: c_int = 8;
pub const PSP_A_STATS_TX_PACKETS: c_int = 9;
pub const PSP_A_STATS_TX_BYTES: c_int = 10;
pub const PSP_A_STATS_TX_ERROR: c_int = 11;
pub const __PSP_A_STATS_MAX: c_int = 12;
pub const PSP_A_STATS_MAX: c_int = __PSP_A_STATS_MAX - 1;

// anonymous C enum
pub const PSP_CMD_DEV_GET: c_int = 1;
pub const PSP_CMD_DEV_ADD_NTF: c_int = 2;
pub const PSP_CMD_DEV_DEL_NTF: c_int = 3;
pub const PSP_CMD_DEV_SET: c_int = 4;
pub const PSP_CMD_DEV_CHANGE_NTF: c_int = 5;
pub const PSP_CMD_KEY_ROTATE: c_int = 6;
pub const PSP_CMD_KEY_ROTATE_NTF: c_int = 7;
pub const PSP_CMD_RX_ASSOC: c_int = 8;
pub const PSP_CMD_TX_ASSOC: c_int = 9;
pub const PSP_CMD_GET_STATS: c_int = 10;
pub const PSP_CMD_DEV_ASSOC: c_int = 11;
pub const PSP_CMD_DEV_DISASSOC: c_int = 12;
pub const __PSP_CMD_MAX: c_int = 13;
pub const PSP_CMD_MAX: c_int = __PSP_CMD_MAX - 1;

static PSP_MCGRP_MGMT_BYTES: [c_char; 5] = [b'm' as c_char, b'g' as c_char, b'm' as c_char, b't' as c_char, 0];
pub const PSP_MCGRP_MGMT: *const c_char = PSP_MCGRP_MGMT_BYTES.as_ptr();
static PSP_MCGRP_USE_BYTES: [c_char; 4] = [b'u' as c_char, b's' as c_char, b'e' as c_char, 0];
pub const PSP_MCGRP_USE: *const c_char = PSP_MCGRP_USE_BYTES.as_ptr();
